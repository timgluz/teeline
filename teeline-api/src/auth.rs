use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;

/// Identifies which user/org/machine a verified API key belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedKey {
    pub subject: String,
}

/// Verifies an opaque API key secret, returning the key's owner if valid.
/// Implementations must reject revoked/expired keys even if the underlying
/// transport reports success — see `ServiceVerifier` for why that matters.
#[async_trait]
pub trait ApiKeyVerifier: Send + Sync {
    async fn verify(&self, key: &str) -> Option<VerifiedKey>;
}

/// Verifies keys issued by the WebAuthn auth service (Cloudflare Pages
/// Functions, `POST /api/auth/keys/verify`) — the self-hosted replacement
/// for the old Clerk verify call. The endpoint is protected by a shared
/// secret header and returns the same `{subject, revoked, expired}` contract
/// Clerk used to, so the verifier logic is a drop-in swap.
pub struct ServiceVerifier {
    service_url: Arc<str>,
    shared_secret: Arc<str>,
    client: reqwest::Client,
}

impl ServiceVerifier {
    pub fn new(service_url: impl Into<Arc<str>>, shared_secret: impl Into<Arc<str>>) -> Self {
        Self {
            service_url: service_url.into(),
            shared_secret: shared_secret.into(),
            client: reqwest::Client::builder()
                // A slow/hung auth-service response must not tie up our
                // request handling indefinitely — reqwest's default is no
                // timeout.
                .timeout(Duration::from_secs(3))
                .build()
                .expect("reqwest client with only a timeout configured always builds"),
        }
    }

    /// Cheap local rejection of obviously-bogus tokens before spending a
    /// network call on the auth service. Not a security control (an attacker
    /// can still send `ak_`-shaped garbage for free) — just avoids the
    /// round-trip for accidental/naive junk under a flood.
    fn passes_shape_check(key: &str) -> bool {
        key.starts_with("ak_") && key.len() >= 10
    }
}

#[derive(Debug, Deserialize)]
struct VerifyResponse {
    subject: String,
    revoked: bool,
    expired: bool,
}

/// A revoked or expired key can still come back with HTTP 200 from the
/// verify endpoint — only the response body says so. Checking status alone
/// would let a revoked/expired key keep authenticating. Pure function so
/// this decision is unit-testable without a network mock.
fn decide(body: VerifyResponse) -> Option<VerifiedKey> {
    (!body.revoked && !body.expired).then_some(VerifiedKey {
        subject: body.subject,
    })
}

#[async_trait]
impl ApiKeyVerifier for ServiceVerifier {
    async fn verify(&self, key: &str) -> Option<VerifiedKey> {
        if !Self::passes_shape_check(key) {
            return None;
        }

        let resp = self
            .client
            .post(format!("{}/api/auth/keys/verify", self.service_url))
            .header("X-Auth-Secret", &*self.shared_secret)
            .json(&serde_json::json!({ "secret": key }))
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None; // secret unknown/revoked at the auth service (404)
        }

        let body: VerifyResponse = resp.json().await.ok()?;
        decide(body)
    }
}

/// Used when no verifier is configured (break-glass-only mode) — the
/// verifier path rejects everything, and only the static key authorizes.
pub struct NullVerifier;

#[async_trait]
impl ApiKeyVerifier for NullVerifier {
    async fn verify(&self, _key: &str) -> Option<VerifiedKey> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn null_verifier_always_rejects() {
        assert!(NullVerifier.verify("ak_anything").await.is_none());
    }

    #[test]
    fn shape_check_rejects_missing_prefix() {
        assert!(!ServiceVerifier::passes_shape_check("not-a-key"));
    }

    #[test]
    fn shape_check_rejects_too_short() {
        assert!(!ServiceVerifier::passes_shape_check("ak_1"));
    }

    #[test]
    fn shape_check_accepts_well_formed_key() {
        assert!(ServiceVerifier::passes_shape_check(
            "ak_3beecc9c60adb5f9b850e91a8ee1e992"
        ));
    }

    #[test]
    fn decide_rejects_revoked_key_despite_200_shape() {
        let body = VerifyResponse {
            subject: "user_abc".to_string(),
            revoked: true,
            expired: false,
        };
        assert_eq!(decide(body), None);
    }

    #[test]
    fn decide_rejects_expired_key() {
        let body = VerifyResponse {
            subject: "user_abc".to_string(),
            revoked: false,
            expired: true,
        };
        assert_eq!(decide(body), None);
    }

    #[test]
    fn decide_accepts_valid_key() {
        let body = VerifyResponse {
            subject: "user_abc".to_string(),
            revoked: false,
            expired: false,
        };
        assert_eq!(
            decide(body),
            Some(VerifiedKey {
                subject: "user_abc".to_string()
            })
        );
    }

    // ---- ServiceVerifier against a stub auth service ---------------------

    /// Minimal stub of the auth service's verify endpoint so the verifier is
    /// exercised over a real HTTP round-trip (no mocks).
    async fn stub_service(
        respond: impl Fn(&str) -> (u16, serde_json::Value) + Send + Sync + 'static,
    ) -> (String, tokio::task::JoinHandle<()>) {
        use axum::{
            Json, Router,
            extract::State,
            http::StatusCode,
            response::{IntoResponse, Response},
            routing::post,
        };

        type Responder = Arc<dyn Fn(&str) -> (u16, serde_json::Value) + Send + Sync>;

        async fn handler(
            State(respond): State<Responder>,
            Json(body): Json<serde_json::Value>,
        ) -> Response {
            let key = body.get("secret").and_then(|v| v.as_str()).unwrap_or("");
            let (status, payload) = respond(key);
            let mut resp = Json(payload).into_response();
            *resp.status_mut() =
                StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            resp
        }

        let router = Router::new()
            .route("/api/auth/keys/verify", post(handler))
            .with_state(Arc::new(respond) as Responder);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (format!("http://{}", addr), handle)
    }

    #[tokio::test]
    async fn service_verifier_accepts_valid_key() {
        let (base, server) = stub_service(|key| {
            if key == "ak_validkey123" {
                (200, serde_json::json!({ "subject": "user_abc", "revoked": false, "expired": false }))
            } else {
                (404, serde_json::json!({ "error": "Unknown or revoked key" }))
            }
        })
        .await;
        let verifier = ServiceVerifier::new(base, "shared-secret");
        assert_eq!(
            verifier.verify("ak_validkey123").await,
            Some(VerifiedKey {
                subject: "user_abc".to_string()
            })
        );
        server.abort();
    }

    #[tokio::test]
    async fn service_verifier_rejects_unknown_key() {
        let (base, server) = stub_service(|_| {
            (
                404,
                serde_json::json!({ "error": "Unknown or revoked key" }),
            )
        })
        .await;
        let verifier = ServiceVerifier::new(base, "shared-secret");
        assert_eq!(verifier.verify("ak_unknownkey123").await, None);
        server.abort();
    }

    #[tokio::test]
    async fn service_verifier_rejects_revoked_via_body_despite_200() {
        // The service returns 200 but flags revoked — decide() must reject.
        let (base, server) = stub_service(|_| {
            (
                200,
                serde_json::json!({ "subject": "user_abc", "revoked": true, "expired": false }),
            )
        })
        .await;
        let verifier = ServiceVerifier::new(base, "shared-secret");
        assert_eq!(verifier.verify("ak_revoked12345").await, None);
        server.abort();
    }

    #[tokio::test]
    async fn service_verifier_shape_check_skips_network() {
        let (base, server) = stub_service(|_| {
            (
                200,
                serde_json::json!({ "subject": "x", "revoked": false, "expired": false }),
            )
        })
        .await;
        let verifier = ServiceVerifier::new(base, "shared-secret");
        assert_eq!(verifier.verify("not-a-key").await, None);
        server.abort();
    }
}

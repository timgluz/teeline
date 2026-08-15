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
        let service_url: Arc<str> = service_url.into();
        let trimmed = service_url.trim_end_matches('/');
        if !trimmed.starts_with("https://") {
            tracing::warn!(
                url = %trimmed,
                "AUTH_SERVICE_URL is not HTTPS — API keys and the shared secret would be sent in cleartext"
            );
        }
        Self {
            service_url: trimmed.into(),
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
            .map_err(|e| {
                // An auth-service outage must not be indistinguishable from a
                // bad key: fail closed (None → 401), but log loudly.
                tracing::warn!(error = %e, "auth service verify request failed");
                e
            })
            .ok()?;

        if !resp.status().is_success() {
            // Expected rejection: unknown/revoked key at the auth service (404).
            tracing::debug!(status = %resp.status(), "auth service rejected key");
            return None;
        }

        let body: VerifyResponse = resp
            .json()
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "failed to parse auth service response");
                e
            })
            .ok()?;
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

    /// Aborts the stub server on drop so a panicking test can't leak the task.
    struct ServerGuard(tokio::task::JoinHandle<()>);
    impl Drop for ServerGuard {
        fn drop(&mut self) {
            self.0.abort();
        }
    }

    /// Minimal stub of the auth service's verify endpoint so the verifier is
    /// exercised over a real HTTP round-trip (no mocks). The stub enforces the
    /// X-Auth-Secret header, so a verifier that stops sending it fails tests.
    async fn stub_service(
        expected_secret: &'static str,
        respond: impl Fn(&str) -> (u16, serde_json::Value) + Send + Sync + 'static,
    ) -> (String, ServerGuard) {
        use axum::{
            Json, Router,
            extract::State,
            http::StatusCode,
            response::{IntoResponse, Response},
            routing::post,
        };

        type Responder = Arc<dyn Fn(&str) -> (u16, serde_json::Value) + Send + Sync>;

        async fn handler(
            State((respond, expected_secret)): State<(Responder, &'static str)>,
            headers: axum::http::HeaderMap,
            Json(body): Json<serde_json::Value>,
        ) -> Response {
            let presented = headers.get("X-Auth-Secret").and_then(|v| v.to_str().ok());
            if presented != Some(expected_secret) {
                return StatusCode::UNAUTHORIZED.into_response();
            }
            let key = body.get("secret").and_then(|v| v.as_str()).unwrap_or("");
            let (status, payload) = respond(key);
            let mut resp = Json(payload).into_response();
            *resp.status_mut() =
                StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            resp
        }

        let router = Router::new()
            .route("/api/auth/keys/verify", post(handler))
            .with_state((Arc::new(respond) as Responder, expected_secret));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (format!("http://{}", addr), ServerGuard(handle))
    }

    #[tokio::test]
    async fn service_verifier_accepts_valid_key() {
        let (base, _server) = stub_service("shared-secret",|key| {
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
    }

    #[tokio::test]
    async fn service_verifier_rejects_unknown_key() {
        let (base, _server) = stub_service("shared-secret", |_| {
            (
                404,
                serde_json::json!({ "error": "Unknown or revoked key" }),
            )
        })
        .await;
        let verifier = ServiceVerifier::new(base, "shared-secret");
        assert_eq!(verifier.verify("ak_unknownkey123").await, None);
    }

    #[tokio::test]
    async fn service_verifier_rejects_revoked_via_body_despite_200() {
        // The service returns 200 but flags revoked — decide() must reject.
        let (base, _server) = stub_service("shared-secret", |_| {
            (
                200,
                serde_json::json!({ "subject": "user_abc", "revoked": true, "expired": false }),
            )
        })
        .await;
        let verifier = ServiceVerifier::new(base, "shared-secret");
        assert_eq!(verifier.verify("ak_revoked12345").await, None);
    }

    #[tokio::test]
    async fn service_verifier_wrong_shared_secret_is_rejected() {
        // The stub 401s when the X-Auth-Secret header doesn't match, proving
        // the verifier actually sends it.
        let (base, _server) = stub_service("expected-secret", |_| {
            (
                200,
                serde_json::json!({ "subject": "x", "revoked": false, "expired": false }),
            )
        })
        .await;
        let verifier = ServiceVerifier::new(base, "wrong-secret");
        assert_eq!(verifier.verify("ak_anything123456").await, None);
    }

    #[tokio::test]
    async fn service_verifier_handles_service_unreachable() {
        // Bind a listener and drop it so the port is closed — the verify
        // request fails at the network layer (the warn! path), and verify()
        // must return None (fail closed).
        let addr = {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap()
        };
        let verifier = ServiceVerifier::new(format!("http://{addr}"), "shared-secret");
        assert_eq!(verifier.verify("ak_whatever123456").await, None);
    }

    #[tokio::test]
    async fn service_verifier_shape_check_skips_network() {
        let (base, _server) = stub_service("shared-secret", |_| {
            (
                200,
                serde_json::json!({ "subject": "x", "revoked": false, "expired": false }),
            )
        })
        .await;
        let verifier = ServiceVerifier::new(base, "shared-secret");
        assert_eq!(verifier.verify("not-a-key").await, None);
    }
}

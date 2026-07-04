use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;

/// Identifies which Clerk user/org/machine a verified API key belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedKey {
    pub subject: String,
}

/// Verifies an opaque API key secret, returning the key's owner if valid.
/// Implementations must reject revoked/expired keys even if the underlying
/// transport reports success — see `ClerkVerifier` for why that matters.
#[async_trait]
pub trait ApiKeyVerifier: Send + Sync {
    async fn verify(&self, key: &str) -> Option<VerifiedKey>;
}

const CLERK_VERIFY_URL: &str = "https://api.clerk.com/v1/api_keys/verify";

/// Verifies keys issued via Clerk's self-serve "API Keys" feature by calling
/// Clerk's Backend API (https://clerk.com/docs/reference/backend-api).
pub struct ClerkVerifier {
    secret_key: Arc<str>,
    client: reqwest::Client,
}

impl ClerkVerifier {
    pub fn new(secret_key: impl Into<Arc<str>>) -> Self {
        Self {
            secret_key: secret_key.into(),
            client: reqwest::Client::builder()
                // A slow/hung Clerk response must not tie up our request
                // handling indefinitely — reqwest's default is no timeout.
                .timeout(Duration::from_secs(3))
                .build()
                .expect("reqwest client with only a timeout configured always builds"),
        }
    }

    /// Cheap local rejection of obviously-bogus tokens before spending a
    /// metered Clerk API call. Not a security control (an attacker can still
    /// send `ak_`-shaped garbage for free) — just avoids paying for
    /// accidental/naive junk under a flood.
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

/// A revoked or expired key still returns HTTP 200 from Clerk — only the
/// response body says so. Checking status alone would let a revoked/expired
/// key keep authenticating. Pure function so this decision is unit-testable
/// without a network mock.
fn decide(body: VerifyResponse) -> Option<VerifiedKey> {
    (!body.revoked && !body.expired).then_some(VerifiedKey {
        subject: body.subject,
    })
}

#[async_trait]
impl ApiKeyVerifier for ClerkVerifier {
    async fn verify(&self, key: &str) -> Option<VerifiedKey> {
        if !Self::passes_shape_check(key) {
            return None;
        }

        let resp = self
            .client
            .post(CLERK_VERIFY_URL)
            .bearer_auth(&*self.secret_key)
            .json(&serde_json::json!({ "secret": key }))
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None; // secret unknown to Clerk (400/404)
        }

        let body: VerifyResponse = resp.json().await.ok()?;
        decide(body)
    }
}

/// Used when `CLERK_SECRET_KEY` is unset — Clerk-based auth is simply
/// disabled, and only the static break-glass key (if any) authorizes.
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
        assert!(!ClerkVerifier::passes_shape_check("not-a-key"));
    }

    #[test]
    fn shape_check_rejects_too_short() {
        assert!(!ClerkVerifier::passes_shape_check("ak_1"));
    }

    #[test]
    fn shape_check_accepts_well_formed_key() {
        assert!(ClerkVerifier::passes_shape_check(
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
}

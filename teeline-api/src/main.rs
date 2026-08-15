use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use teeline_api::{
    AppState,
    auth::{ApiKeyVerifier, NullVerifier, ServiceVerifier},
    metrics::MetricsState,
    middleware,
    services::{SolverRegistry, TspService},
};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

/// Returns the configured requests-per-minute rate limit.
/// `RATE_LIMIT_RPM=0` disables rate limiting; values above 60_000 are ignored.
/// Defaults to 100 RPM.
fn rate_limit_rpm() -> u64 {
    std::env::var("RATE_LIMIT_RPM")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&v| v <= 60_000)
        .unwrap_or(100)
}

/// Returns the configured static break-glass API key, if any. When unset OR
/// empty, this credential is disabled entirely (back-compat with the
/// original no-auth MVP behavior). `std::env::var(...).ok()` alone would
/// treat `API_KEY=""` as "set" (an empty-but-present token), silently
/// enabling auth with a trivially guessable blank credential — worse than
/// disabled, since operators wouldn't know from the logs.
fn api_key() -> Option<String> {
    std::env::var("API_KEY")
        .ok()
        .filter(|token| !token.is_empty())
}

/// Returns the configured auth service URL (WebAuthn auth service, e.g.
/// `https://tspsolver.com`), if any. Same empty-string-safety as `api_key()`.
fn auth_service_url() -> Option<String> {
    std::env::var("AUTH_SERVICE_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

/// Returns the configured shared secret for the auth service's internal
/// verify endpoint, if any. Same empty-string-safety as `api_key()`.
fn auth_service_secret() -> Option<String> {
    std::env::var("AUTH_SERVICE_SECRET")
        .ok()
        .filter(|token| !token.is_empty())
}

/// How the API authenticates requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthMode {
    /// No auth middleware at all (original no-auth MVP behavior).
    Disabled,
    /// Only the static break-glass `API_KEY` — used by local-dev & CI.
    Breakglass,
    /// The auth service verifier (optionally alongside the break-glass key).
    Service,
}

/// Pure decision so it is unit-testable without touching process env.
fn auth_mode_with(env_mode: Option<&str>, has_service: bool, has_breakglass: bool) -> AuthMode {
    match env_mode {
        Some("breakglass") => AuthMode::Breakglass,
        Some("service") => AuthMode::Service,
        Some("disabled") => AuthMode::Disabled,
        Some(other) => {
            tracing::warn!("unknown TEELINE_AUTH_MODE '{other}', treating as disabled");
            AuthMode::Disabled
        }
        // Back-compat inference when TEELINE_AUTH_MODE is unset: service wins
        // if fully configured, else break-glass if a static key exists.
        None => match (has_service, has_breakglass) {
            (true, _) => AuthMode::Service,
            (false, true) => AuthMode::Breakglass,
            (false, false) => AuthMode::Disabled,
        },
    }
}

fn auth_mode() -> AuthMode {
    auth_mode_with(
        std::env::var("TEELINE_AUTH_MODE").ok().as_deref(),
        auth_service_url().is_some() && auth_service_secret().is_some(),
        api_key().is_some(),
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_owned());
    let addr = format!("0.0.0.0:{port}");
    let rpm = rate_limit_rpm();

    let state = AppState {
        solver_service: Arc::new(TspService),
        registry_service: Arc::new(SolverRegistry),
        metrics: Arc::new(MetricsState::new()),
    };

    // Rate limiting and auth are both scoped to /api/v1/* only — Fly.io's
    // scraper must not be throttled or challenged on /metrics, and ops
    // endpoints (/, /healthz, /metrics, /docs, /openapi.json) stay open.
    let mut api: Router<AppState> = teeline_api::build_api_router();

    if let Some(period_ms) = 60_000u64.checked_div(rpm) {
        tracing::info!("rate limiting enabled: {rpm} RPM");
        let governor_conf = GovernorConfigBuilder::default()
            .per_millisecond(period_ms)
            .burst_size(10)
            .finish()
            .expect("rate limiter config is valid (period_ms > 0 guaranteed by rpm filter)");
        let limiter = governor_conf.limiter().clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                limiter.retain_recent();
            }
        });
        api = api.layer(GovernorLayer::new(governor_conf));
    } else {
        tracing::info!("rate limiting disabled (RATE_LIMIT_RPM=0)");
    }

    // Applied after GovernorLayer, so auth wraps outermost for matched
    // routes (each subsequent .layer()/.route_layer() call wraps the
    // previous stack) — unauthenticated requests are rejected before
    // consuming rate-limit budget for the static-key path (the service path
    // is itself a network call and isn't cheap, but that's an accepted v1
    // tradeoff, not something layering order can fix). require_auth uses
    // route_layer internally (not layer) so it only runs for requests that
    // actually match a route.
    let static_key = api_key();
    match auth_mode() {
        AuthMode::Disabled => {
            tracing::info!("API auth disabled (TEELINE_AUTH_MODE=disabled or no credentials)");
        }
        AuthMode::Breakglass => {
            tracing::info!("API auth: break-glass key only (TEELINE_AUTH_MODE=breakglass)");
            // An absent static key becomes "" here, which `token_matches`
            // always rejects (defense in depth against an empty token).
            api = middleware::require_auth(
                api,
                static_key.unwrap_or_default(),
                Arc::new(NullVerifier),
            );
        }
        AuthMode::Service => {
            let url = auth_service_url()
                .expect("AUTH_SERVICE_URL is required when TEELINE_AUTH_MODE=service");
            let secret = auth_service_secret()
                .expect("AUTH_SERVICE_SECRET is required when TEELINE_AUTH_MODE=service");
            tracing::info!(
                static_key = static_key.is_some(),
                "API auth: verifying keys via auth service"
            );
            let verifier: Arc<dyn ApiKeyVerifier> = Arc::new(ServiceVerifier::new(url, secret));
            api = middleware::require_auth(api, static_key.unwrap_or_default(), verifier);
        }
    }

    let app = teeline_api::build_router(state, api);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_mode_wins_over_inference() {
        assert_eq!(
            auth_mode_with(Some("breakglass"), true, true),
            AuthMode::Breakglass
        );
        assert_eq!(
            auth_mode_with(Some("service"), false, false),
            AuthMode::Service
        );
        assert_eq!(
            auth_mode_with(Some("disabled"), true, true),
            AuthMode::Disabled
        );
    }

    #[test]
    fn unknown_mode_treated_as_disabled() {
        assert_eq!(
            auth_mode_with(Some("banana"), true, true),
            AuthMode::Disabled
        );
    }

    #[test]
    fn unset_mode_infers_from_credentials() {
        // service configured → Service (even with a static key present)
        assert_eq!(auth_mode_with(None, true, true), AuthMode::Service);
        // only a static key → Breakglass
        assert_eq!(auth_mode_with(None, false, true), AuthMode::Breakglass);
        // nothing → Disabled (back-compat no-auth MVP)
        assert_eq!(auth_mode_with(None, false, false), AuthMode::Disabled);
        // service URL without secret → not "service", so falls through
        assert_eq!(auth_mode_with(None, false, false), AuthMode::Disabled);
    }
}

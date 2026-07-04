use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use teeline_api::{
    AppState,
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

/// Returns the configured API key, if any. When unset OR empty, auth is
/// disabled entirely (back-compat with the original no-auth MVP behavior).
/// `std::env::var(...).ok()` alone would treat `API_KEY=""` as "set" (an
/// empty-but-present token), silently enabling auth with a trivially
/// guessable blank credential — worse than disabled, since operators
/// wouldn't know from the logs.
fn api_key() -> Option<String> {
    std::env::var("API_KEY")
        .ok()
        .filter(|token| !token.is_empty())
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
    // consuming rate-limit budget. require_auth uses route_layer
    // internally (not layer) so it only runs for requests that actually
    // match a route.
    if let Some(token) = api_key() {
        tracing::info!("API key auth enabled");
        api = middleware::require_auth(api, token);
    } else {
        tracing::info!("API key auth disabled (API_KEY unset)");
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

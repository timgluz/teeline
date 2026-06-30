use std::net::SocketAddr;
use std::sync::Arc;

use teeline_api::{
    AppState,
    services::{SolverRegistry, TspService},
};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_owned());
    let addr = format!("127.0.0.1:{port}");

    // RATE_LIMIT_RPM=0 disables rate limiting (useful for testing).
    // Any value 1–60_000 enables it; anything else defaults to 100 RPM.
    let rate_limit_rpm: Option<u64> = {
        let raw = std::env::var("RATE_LIMIT_RPM")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&v| v <= 60_000)
            .unwrap_or(100);
        (raw > 0).then_some(raw)
    };

    let state = AppState {
        solver_service: Arc::new(TspService),
        registry_service: Arc::new(SolverRegistry),
    };
    let app = if let Some(rpm) = rate_limit_rpm {
        let period_ms = 60_000u64 / rpm;
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
        teeline_api::build_router(state).layer(GovernorLayer::new(governor_conf))
    } else {
        teeline_api::build_router(state)
    };
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

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

    let rpm: u64 = std::env::var("RATE_LIMIT_RPM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let period_ms = 60_000u64 / rpm;
    let governor_conf = GovernorConfigBuilder::default()
        .per_millisecond(period_ms)
        .burst_size(10)
        .finish()
        .unwrap();

    let state = AppState {
        solver_service: Arc::new(TspService),
        registry_service: Arc::new(SolverRegistry),
    };
    let app = teeline_api::build_router(state).layer(GovernorLayer::new(governor_conf));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

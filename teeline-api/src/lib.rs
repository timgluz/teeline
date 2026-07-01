pub mod error;
pub mod metrics;
pub mod middleware;
pub mod models;
pub mod openapi;
pub mod routes;
pub mod services;

use std::sync::Arc;

use axum::routing::{get, post};

use crate::metrics::MetricsState;

#[derive(Clone)]
pub struct AppState {
    pub solver_service: Arc<dyn services::TspSolverService>,
    pub registry_service: Arc<dyn services::SolverRegistryService>,
    pub metrics: Arc<MetricsState>,
}

/// The /api/v1/* routes without state, so callers (e.g. main.rs) can apply
/// a rate-limiting layer before providing state and merging into the full app.
pub fn build_api_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/v1/health", get(routes::health::handler))
        .route("/api/v1/solvers", get(routes::solvers::list_solvers))
        .route("/api/v1/parse", post(routes::parse::parse))
        .route("/api/v1/solve", post(routes::solve::solve))
}

/// Full router used by integration tests. GovernorLayer is not applied here;
/// main.rs applies it to the api sub-router only before calling this, or the
/// test harness omits it entirely.
pub fn build_router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/", get(routes::index::handler))
        .route("/healthz", get(routes::health::handler))
        .route("/metrics", get(routes::metrics::handler))
        .merge(openapi::openapi_router())
        .merge(build_api_router())
        .layer(middleware::MetricsLayer::new(Arc::clone(&state.metrics)))
        .with_state(state)
}

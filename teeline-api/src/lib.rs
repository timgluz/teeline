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

/// The /api/v1/* routes without state, so a rate-limiting layer can be applied
/// before providing state and merging into the full app.
pub fn build_api_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/v1/health", get(routes::health::handler))
        .route("/api/v1/solvers", get(routes::solvers::list_solvers))
        .route("/api/v1/parse", post(routes::parse::parse))
        .route("/api/v1/solve", post(routes::solve::solve))
}

/// Full router with MetricsLayer applied. `api` is the already-assembled
/// (and optionally layered, e.g. with GovernorLayer/AuthLayer) `/api/v1/*`
/// sub-router — callers build it via `build_api_router()` plus whatever
/// layers they need, then pass it here so route wiring lives in one place.
pub fn build_router(state: AppState, api: axum::Router<AppState>) -> axum::Router {
    axum::Router::new()
        .route("/", get(routes::index::handler))
        .route("/healthz", get(routes::health::handler))
        .route("/metrics", get(routes::metrics::handler))
        .merge(openapi::openapi_router())
        .merge(api)
        .layer(middleware::MetricsLayer::new(Arc::clone(&state.metrics)))
        .with_state(state)
}

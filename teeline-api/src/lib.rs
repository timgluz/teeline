pub mod error;
pub mod models;
pub mod openapi;
pub mod routes;
pub mod services;

use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub solver_service: Arc<dyn services::TspSolverService>,
    pub registry_service: Arc<dyn services::SolverRegistryService>,
}

pub fn build_router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route(
            "/api/v1/health",
            axum::routing::get(routes::health::handler),
        )
        .route("/healthz", axum::routing::get(routes::health::handler))
        .route(
            "/api/v1/solvers",
            axum::routing::get(routes::solvers::list_solvers),
        )
        .route("/api/v1/parse", axum::routing::post(routes::parse::parse))
        .route("/api/v1/solve", axum::routing::post(routes::solve::solve))
        .route(
            "/api/v1/compare",
            axum::routing::post(routes::compare::compare),
        )
        .merge(openapi::openapi_router())
        .with_state(state)
}

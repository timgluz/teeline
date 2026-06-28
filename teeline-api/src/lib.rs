pub mod error;
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
        .with_state(state)
}

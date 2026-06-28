use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use teeline_api::{
    AppState,
    services::{StubSolverRegistryService, StubTspSolverService},
};
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let state = AppState {
        solver_service: Arc::new(StubTspSolverService),
        registry_service: Arc::new(StubSolverRegistryService),
    };
    let app = teeline_api::build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
}

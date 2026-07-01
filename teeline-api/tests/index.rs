use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use teeline_api::{
    AppState,
    services::{SolverRegistry, TspService},
};
use tower::ServiceExt;

fn make_app() -> axum::Router {
    let state = AppState {
        solver_service: Arc::new(TspService),
        registry_service: Arc::new(SolverRegistry),
    };
    teeline_api::build_router(state)
}

#[tokio::test]
async fn index_returns_ok() {
    let resp = make_app()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn index_returns_expected_fields() {
    let resp = make_app()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["name"], "teeline-api");
    assert!(json["routes"].is_array());
    assert!(!json["routes"].as_array().unwrap().is_empty());
}

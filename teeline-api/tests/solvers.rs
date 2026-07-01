use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use teeline_api::{
    AppState,
    metrics::MetricsState,
    services::{SolverRegistry, TspService},
};
use tower::ServiceExt;

fn make_app() -> axum::Router {
    let state = AppState {
        solver_service: Arc::new(TspService),
        registry_service: Arc::new(SolverRegistry),
        metrics: Arc::new(MetricsState::new()),
    };
    teeline_api::build_router(state)
}

#[tokio::test]
async fn solvers_returns_ok() {
    let response = make_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/solvers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn solvers_returns_non_empty_array() {
    let response = make_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/solvers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert!(!arr.is_empty(), "solvers list must not be empty");
}

#[tokio::test]
async fn solvers_contains_nn() {
    let response = make_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/solvers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().unwrap();

    let has_nn = arr.iter().any(|e| e["alias"] == "nn");
    assert!(has_nn, "solvers list must contain nn alias");
}

#[tokio::test]
async fn solvers_entries_have_required_fields() {
    let response = make_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/solvers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().unwrap();

    for entry in arr {
        assert!(entry["name"].is_string(), "entry must have 'name'");
        assert!(entry["alias"].is_string(), "entry must have 'alias'");
        assert!(entry["category"].is_string(), "entry must have 'category'");
        assert!(entry["desc"].is_string(), "entry must have 'desc'");
        assert!(
            entry["complexity"].is_string(),
            "entry must have 'complexity'"
        );
        assert!(
            entry["has_options"].is_boolean(),
            "entry must have 'has_options'"
        );
        assert!(entry["exact"].is_boolean(), "entry must have 'exact'");
    }
}

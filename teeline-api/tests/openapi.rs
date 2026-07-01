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
async fn openapi_json_returns_ok() {
    let resp = make_app()
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn openapi_json_is_valid_json() {
    let resp = make_app()
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["openapi"], "3.1.0");
}

#[tokio::test]
async fn openapi_json_contains_all_four_paths() {
    let resp = make_app()
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let paths = json["paths"].as_object().unwrap();
    assert!(
        paths.contains_key("/api/v1/health"),
        "missing /api/v1/health"
    );
    assert!(
        paths.contains_key("/api/v1/solvers"),
        "missing /api/v1/solvers"
    );
    assert!(paths.contains_key("/api/v1/parse"), "missing /api/v1/parse");
    assert!(paths.contains_key("/api/v1/solve"), "missing /api/v1/solve");
}

#[tokio::test]
async fn openapi_json_has_schemas_in_components() {
    let resp = make_app()
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let schemas = json["components"]["schemas"].as_object().unwrap();
    assert!(!schemas.is_empty(), "components/schemas must not be empty");
    assert!(
        schemas.contains_key("SolveRequest"),
        "missing SolveRequest schema"
    );
    assert!(
        schemas.contains_key("ParseResponse"),
        "missing ParseResponse schema"
    );
}

#[tokio::test]
async fn scalar_docs_returns_html() {
    let resp = make_app()
        .oneshot(Request::builder().uri("/docs").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();
    assert!(body.contains("<!doctype html>") || body.contains("<!DOCTYPE html>"));
}

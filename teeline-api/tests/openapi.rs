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
    teeline_api::build_router(state, teeline_api::build_api_router())
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
async fn all_protected_paths_have_security_requirement_except_health() {
    // Enforces that every route under /api/v1/* except the health exemption
    // documents a non-empty `security` requirement in its OpenAPI annotation.
    // AuthLayer protects all of build_api_router() at runtime structurally,
    // but the OpenAPI spec only reflects that via manually-added
    // `security(...)` blocks on each #[utoipa::path] — nothing else ties the
    // two together, so this is what catches a future route that forgets it.
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

    const AUTH_EXEMPT_PATHS: &[&str] = &["/api/v1/health"];

    for (path, methods) in paths {
        if !path.starts_with("/api/v1/") || AUTH_EXEMPT_PATHS.contains(&path.as_str()) {
            continue;
        }
        for (method, spec) in methods.as_object().unwrap() {
            let security = spec.get("security").and_then(|s| s.as_array());
            assert!(
                security.is_some_and(|s| !s.is_empty()),
                "{method} {path} is protected by AuthLayer at runtime but has no \
                 security(...) in its #[utoipa::path] annotation"
            );
        }
    }
}

#[tokio::test]
async fn solve_and_parse_request_bodies_have_named_examples() {
    // /solve and /parse both accept either `cities` or `tsplib` input — named
    // examples in the OpenAPI spec are what let Scalar's "Try it" panel offer
    // a dropdown of ready-to-run payloads instead of an empty body. This
    // guards against someone accidentally dropping the examples() block
    // later.
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

    let solve_examples = &json["paths"]["/api/v1/solve"]["post"]["requestBody"]["content"]["application/json"]
        ["examples"];
    assert!(
        solve_examples.as_object().is_some_and(|o| o.len() >= 2),
        "/api/v1/solve should document at least 2 named examples"
    );

    let parse_examples = &json["paths"]["/api/v1/parse"]["post"]["requestBody"]["content"]["application/json"]
        ["examples"];
    assert!(
        parse_examples.as_object().is_some_and(|o| o.len() >= 2),
        "/api/v1/parse should document at least 2 named examples"
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

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

fn json_body(json: &str) -> Body {
    Body::from(json.to_owned())
}

fn post(uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(body)
        .unwrap()
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

const TINY_CITIES: &str = r#"{"cities":[{"x":0.0,"y":0.0},{"x":1.0,"y":0.0},{"x":0.5,"y":1.0}]}"#;

#[tokio::test]
async fn solve_nn_returns_ok() {
    let body = format!(r#"{{"solver":"nn","input":{TINY_CITIES}}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/solve", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn solve_returns_required_fields() {
    let body = format!(r#"{{"solver":"nn","input":{TINY_CITIES}}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/solve", json_body(&body)))
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["solver"], "nn");
    assert!(json["total"].is_f64(), "total must be a number");
    assert!(json["route"].is_array(), "route must be an array");
    assert!(json["duration_ms"].is_u64(), "duration_ms must be a number");
}

#[tokio::test]
async fn solve_route_contains_all_cities() {
    let body = format!(r#"{{"solver":"nn","input":{TINY_CITIES}}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/solve", json_body(&body)))
        .await
        .unwrap();
    let json = body_json(resp).await;
    let route = json["route"].as_array().unwrap();
    assert_eq!(route.len(), 3, "route must visit all 3 cities");
}

#[tokio::test]
async fn solve_unknown_solver_returns_400() {
    let body = format!(r#"{{"solver":"does_not_exist","input":{TINY_CITIES}}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/solve", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn solve_both_input_fields_returns_400() {
    let body = r#"{"solver":"nn","input":{"cities":[{"x":0.0,"y":0.0},{"x":1.0,"y":0.0},{"x":0.5,"y":1.0}],"tsplib":"NAME: x"}}"#;
    let resp = make_app()
        .oneshot(post("/api/v1/solve", json_body(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn solve_neither_input_field_returns_400() {
    let body = r#"{"solver":"nn","input":{}}"#;
    let resp = make_app()
        .oneshot(post("/api/v1/solve", json_body(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

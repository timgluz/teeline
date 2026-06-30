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

const TINY_INPUT: &str = r#"{"cities":[{"x":0.0,"y":0.0},{"x":1.0,"y":0.0},{"x":0.5,"y":1.0}]}"#;

#[tokio::test]
async fn compare_two_solvers_returns_ok() {
    let body = format!(r#"{{"solvers":["nn","2opt"],"input":{TINY_INPUT}}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/compare", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn compare_returns_entries_for_all_solvers() {
    let body = format!(r#"{{"solvers":["nn","2opt"],"input":{TINY_INPUT}}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/compare", json_body(&body)))
        .await
        .unwrap();
    let json = body_json(resp).await;
    let entries = json["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
}

#[tokio::test]
async fn compare_entries_sorted_by_cost_ascending() {
    let body = format!(r#"{{"solvers":["nn","2opt","sa"],"input":{TINY_INPUT}}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/compare", json_body(&body)))
        .await
        .unwrap();
    let json = body_json(resp).await;
    let entries = json["entries"].as_array().unwrap();
    let ok_entries: Vec<f64> = entries
        .iter()
        .filter(|e| e["status"] == "ok")
        .map(|e| e["total"].as_f64().unwrap())
        .collect();
    for w in ok_entries.windows(2) {
        assert!(w[0] <= w[1], "entries must be sorted by cost ascending");
    }
}

#[tokio::test]
async fn compare_fewer_than_two_solvers_returns_400() {
    let body = format!(r#"{{"solvers":["nn"],"input":{TINY_INPUT}}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/compare", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn compare_no_solvers_returns_400() {
    let body = format!(r#"{{"solvers":[],"input":{TINY_INPUT}}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/compare", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

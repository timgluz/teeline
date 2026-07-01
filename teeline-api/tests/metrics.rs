use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
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

fn metrics_req() -> Request<Body> {
    Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .unwrap()
}

fn solve_req(solver: &str) -> Request<Body> {
    let body = format!(
        r#"{{"input":{{"cities":[{{"x":0.0,"y":0.0}},{{"x":1.0,"y":0.0}},{{"x":0.5,"y":1.0}}]}},"solver":"{solver}"}}"#
    );
    Request::builder()
        .method("POST")
        .uri("/api/v1/solve")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

async fn body_text(resp: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.into()).unwrap()
}

#[tokio::test]
async fn metrics_returns_200() {
    let resp = make_app().oneshot(metrics_req()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn metrics_content_type_is_openmetrics() {
    let resp = make_app().oneshot(metrics_req()).await.unwrap();
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        ct.contains("openmetrics-text"),
        "expected openmetrics-text, got {ct}"
    );
}

#[tokio::test]
async fn metrics_body_contains_http_requests_total() {
    let app = make_app();
    // prometheus-client emits sample lines only after the first increment.
    // Make a health request so http_requests_total gets at least one sample.
    app.clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_text(app.oneshot(metrics_req()).await.unwrap()).await;
    assert!(
        body.contains("http_requests_total"),
        "missing http_requests_total in:\n{body}"
    );
}

#[tokio::test]
async fn solver_metrics_present_after_successful_solve() {
    let app = make_app();
    let solve_resp = app.clone().oneshot(solve_req("nn")).await.unwrap();
    assert_eq!(
        solve_resp.status(),
        StatusCode::OK,
        "solve must succeed before solver metrics are populated"
    );
    let body = body_text(app.oneshot(metrics_req()).await.unwrap()).await;
    assert!(
        body.contains("teeline_solver_requests_total"),
        "missing teeline_solver_requests_total after solve:\n{body}"
    );
    assert!(
        body.contains("teeline_solver_duration_seconds"),
        "missing teeline_solver_duration_seconds after solve:\n{body}"
    );
}

#[tokio::test]
async fn solver_error_metric_increments_for_known_solver_error() {
    let app = make_app();
    // Valid alias that the mock service can fail on — use a real alias but
    // send invalid input so the service returns an error.
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/solve")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"input":{"cities":[]},"solver":"nn"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_text(app.oneshot(metrics_req()).await.unwrap()).await;
    assert!(
        body.contains("http_requests_total"),
        "missing http_requests_total after failed solve:\n{body}"
    );
    // solver error metric must be incremented for a known alias even on failure
    assert!(
        body.contains("teeline_solver_requests_total"),
        "missing teeline_solver_requests_total after failed solve:\n{body}"
    );
}

#[tokio::test]
async fn unknown_solver_does_not_pollute_solver_labels() {
    let app = make_app();
    app.clone()
        .oneshot(solve_req("__definitely_not_a_real_solver__"))
        .await
        .unwrap();
    let body = body_text(app.oneshot(metrics_req()).await.unwrap()).await;
    // Client-supplied unknown solver must not appear as a label value
    assert!(
        !body.contains("__definitely_not_a_real_solver__"),
        "client-controlled solver name leaked into metrics labels:\n{body}"
    );
}

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

const SQUARE_CITIES: &str =
    r#"{"cities":[{"x":0.0,"y":0.0},{"x":1.0,"y":0.0},{"x":1.0,"y":1.0},{"x":0.0,"y":1.0}]}"#;

#[tokio::test]
async fn pipeline_two_stages_returns_ok() {
    let body =
        format!(r#"{{"input":{SQUARE_CITIES},"stages":[{{"solver":"nn"}},{{"solver":"2opt"}}]}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/pipeline", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn pipeline_returns_ordered_stage_results_and_final_fields() {
    let body =
        format!(r#"{{"input":{SQUARE_CITIES},"stages":[{{"solver":"nn"}},{{"solver":"2opt"}}]}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/pipeline", json_body(&body)))
        .await
        .unwrap();
    let json = body_json(resp).await;

    let stages = json["stages"].as_array().unwrap();
    assert_eq!(stages.len(), 2);
    assert_eq!(stages[0]["solver"], "nn");
    assert_eq!(stages[1]["solver"], "2opt");
    assert!(stages[0]["duration_ms"].is_u64());

    assert_eq!(json["final_cost"], stages[1]["cost"]);
    assert_eq!(json["final_tour"], stages[1]["tour"]);
    assert!(json["warnings"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn pipeline_single_stage_returns_400() {
    let body = format!(r#"{{"input":{SQUARE_CITIES},"stages":[{{"solver":"nn"}}]}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/pipeline", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn pipeline_unknown_solver_in_first_stage_returns_400() {
    let body = format!(
        r#"{{"input":{SQUARE_CITIES},"stages":[{{"solver":"does_not_exist"}},{{"solver":"2opt"}}]}}"#
    );
    let resp = make_app()
        .oneshot(post("/api/v1/pipeline", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// Fail-fast: an unknown solver in a *later* stage still fails the whole
/// request, even though earlier stages are well-formed — no partial results.
#[tokio::test]
async fn pipeline_unknown_solver_in_second_stage_returns_400() {
    let body = format!(
        r#"{{"input":{SQUARE_CITIES},"stages":[{{"solver":"nn"}},{{"solver":"does_not_exist"}}]}}"#
    );
    let resp = make_app()
        .oneshot(post("/api/v1/pipeline", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert!(
        json.get("stages").is_none(),
        "no partial results on failure"
    );
}

#[tokio::test]
async fn pipeline_nn_mid_pipeline_returns_warnings() {
    let body =
        format!(r#"{{"input":{SQUARE_CITIES},"stages":[{{"solver":"2opt"}},{{"solver":"nn"}}]}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/pipeline", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let warnings = json["warnings"].as_array().unwrap();
    assert!(!warnings.is_empty());
}

#[tokio::test]
async fn pipeline_with_per_stage_configs_returns_ok() {
    let body = format!(
        r#"{{"input":{SQUARE_CITIES},"stages":[{{"solver":"nn"}},{{"solver":"sa","configs":{{"sa":{{"max_temperature":200.0}}}}}}]}}"#
    );
    let resp = make_app()
        .oneshot(post("/api/v1/pipeline", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn pipeline_both_input_fields_returns_400() {
    let body = r#"{"input":{"cities":[{"x":0.0,"y":0.0},{"x":1.0,"y":0.0}],"tsplib":"NAME: x"},"stages":[{"solver":"nn"},{"solver":"2opt"}]}"#;
    let resp = make_app()
        .oneshot(post("/api/v1/pipeline", json_body(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

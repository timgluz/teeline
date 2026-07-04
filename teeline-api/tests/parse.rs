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

const TINY_TSPLIB: &str = "\
NAME: tiny
TYPE: TSP
COMMENT: three cities
DIMENSION: 3
EDGE_WEIGHT_TYPE: EUC_2D
NODE_COORD_SECTION
1 0.0 0.0
2 1.0 0.0
3 0.5 1.0
EOF
";

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

#[tokio::test]
async fn parse_tsplib_returns_ok() {
    let tsplib = serde_json::to_string(TINY_TSPLIB).unwrap();
    let body = format!(r#"{{"input":{{"tsplib":{tsplib}}}}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/parse", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn parse_tsplib_returns_city_list() {
    let tsplib = serde_json::to_string(TINY_TSPLIB).unwrap();
    let body = format!(r#"{{"input":{{"tsplib":{tsplib}}}}}"#);
    let resp = make_app()
        .oneshot(post("/api/v1/parse", json_body(&body)))
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["name"], "tiny");
    assert_eq!(json["distance_type"], "EUC_2D");
    assert_eq!(json["cities"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn parse_json_cities_returns_ok() {
    let body = r#"{"input":{"cities":[{"x":0.0,"y":0.0},{"x":1.0,"y":0.0},{"x":0.5,"y":1.0}]}}"#;
    let resp = make_app()
        .oneshot(post("/api/v1/parse", json_body(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn parse_json_cities_assigns_sequential_ids() {
    let body = r#"{"input":{"cities":[{"x":0.0,"y":0.0},{"x":1.0,"y":0.0},{"x":0.5,"y":1.0}]}}"#;
    let resp = make_app()
        .oneshot(post("/api/v1/parse", json_body(body)))
        .await
        .unwrap();
    let json = body_json(resp).await;
    let cities = json["cities"].as_array().unwrap();
    assert_eq!(cities[0]["id"], 1);
    assert_eq!(cities[1]["id"], 2);
}

#[tokio::test]
async fn parse_both_fields_returns_400() {
    let tsplib = serde_json::to_string(TINY_TSPLIB).unwrap();
    let body = format!(
        r#"{{"input":{{"tsplib":{tsplib},"cities":[{{"x":0.0,"y":0.0}},{{"x":1.0,"y":0.0}}]}}}}"#
    );
    let resp = make_app()
        .oneshot(post("/api/v1/parse", json_body(&body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn parse_neither_field_returns_400() {
    let body = r#"{"input":{}}"#;
    let resp = make_app()
        .oneshot(post("/api/v1/parse", json_body(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

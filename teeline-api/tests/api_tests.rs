use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use teeline_api::{
    AppState,
    metrics::MetricsState,
    middleware,
    models::{
        request::{ParseRequest, SolveRequest},
        response::{AlgorithmInfo, CityDto, ParseResponse, SolveResponse},
    },
    services::{SolverRegistryService, TspSolverService},
};
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

struct MockSolverService;

#[async_trait]
impl TspSolverService for MockSolverService {
    async fn parse(&self, req: &ParseRequest) -> Result<ParseResponse, String> {
        req.input.validate()?;
        Ok(ParseResponse {
            name: "mock".to_string(),
            comment: String::new(),
            distance_type: "EUC_2D".to_string(),
            cities: vec![CityDto {
                id: 1,
                x: 0.0,
                y: 0.0,
            }],
        })
    }

    async fn solve(&self, req: &SolveRequest) -> Result<SolveResponse, String> {
        // Solver name drives error simulation; TspInput validation is intentionally
        // skipped here — that path is covered by real-service tests in solve.rs.
        match req.solver.as_str() {
            "__error__" => Err("unknown solver: __error__".to_string()),
            "__panic__" => Err("task panic: simulated".to_string()),
            _ => Ok(SolveResponse {
                solver: req.solver.clone(),
                total: 1.0,
                route: vec![1],
                duration_ms: 0,
            }),
        }
    }
}

struct MockRegistry;

impl SolverRegistryService for MockRegistry {
    fn list(&self) -> Vec<AlgorithmInfo> {
        vec![AlgorithmInfo {
            name: "Mock".to_string(),
            alias: "mock".to_string(),
            category: "Test".to_string(),
            desc: "stub".to_string(),
            complexity: "O(1)".to_string(),
            has_options: false,
            exact: false,
        }]
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_app() -> axum::Router {
    let state = AppState {
        solver_service: Arc::new(MockSolverService),
        registry_service: Arc::new(MockRegistry),
        metrics: Arc::new(MetricsState::new()),
    };
    teeline_api::build_router(state, teeline_api::build_api_router())
}

const TEST_TOKEN: &str = "test-secret-token";

fn make_authed_app() -> axum::Router {
    let state = AppState {
        solver_service: Arc::new(MockSolverService),
        registry_service: Arc::new(MockRegistry),
        metrics: Arc::new(MetricsState::new()),
    };
    let api = middleware::require_auth(teeline_api::build_api_router(), TEST_TOKEN);
    teeline_api::build_router(state, api)
}

fn get_with_header(uri: &str, header: &str, value: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header(header, value)
        .body(Body::empty())
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn post_json(uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap()
}

async fn json_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

const TINY_TSPLIB: &str = r#"NAME: tiny\nTYPE: TSP\nCOMMENT: three cities\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EUC_2D\nNODE_COORD_SECTION\n1 0.0 0.0\n2 1.0 0.0\n3 0.5 1.0\nEOF\n"#;

const TINY_CITIES: &str = r#"{"cities":[{"x":0.0,"y":0.0},{"x":1.0,"y":0.0},{"x":0.5,"y":1.0}]}"#;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_returns_ok() {
    let resp = make_app().oneshot(get("/api/v1/health")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn healthz_alias_returns_ok() {
    let resp = make_app().oneshot(get("/healthz")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn index_returns_ok() {
    let resp = make_app().oneshot(get("/")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert_eq!(json["status"], "ok");
    assert_eq!(json["name"], "teeline-api");
    assert!(json["routes"].as_array().is_some_and(|r| !r.is_empty()));
}

#[tokio::test]
async fn solvers_returns_list() {
    let resp = make_app().oneshot(get("/api/v1/solvers")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert!(json.as_array().is_some_and(|a| !a.is_empty()));
}

#[tokio::test]
async fn parse_with_cities_returns_response() {
    let body = format!(r#"{{"input":{TINY_CITIES}}}"#);
    let resp = make_app()
        .oneshot(post_json("/api/v1/parse", &body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert!(json["cities"].as_array().is_some());
}

#[tokio::test]
async fn parse_with_tsplib_returns_response() {
    let body = format!(r#"{{"input":{{"tsplib":"{TINY_TSPLIB}"}}}}"#);
    let resp = make_app()
        .oneshot(post_json("/api/v1/parse", &body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert!(json["distance_type"].as_str().is_some());
}

#[tokio::test]
async fn parse_with_invalid_input_returns_400() {
    let body =
        format!(r#"{{"input":{{"tsplib":"{TINY_TSPLIB}","cities":[{{"x":0.0,"y":0.0}}]}}}}"#);
    let resp = make_app()
        .oneshot(post_json("/api/v1/parse", &body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn solve_returns_response() {
    let body = format!(r#"{{"input":{TINY_CITIES},"solver":"nn"}}"#);
    let resp = make_app()
        .oneshot(post_json("/api/v1/solve", &body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert!(json["solver"].as_str().is_some());
    assert!(json["total"].as_f64().is_some());
    assert!(json["route"].as_array().is_some());
    assert!(json["duration_ms"].as_u64().is_some());
}

#[tokio::test]
async fn solve_service_error_returns_400() {
    let body = format!(r#"{{"input":{TINY_CITIES},"solver":"__error__"}}"#);
    let resp = make_app()
        .oneshot(post_json("/api/v1/solve", &body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn solve_panic_returns_500() {
    let body = format!(r#"{{"input":{TINY_CITIES},"solver":"__panic__"}}"#);
    let resp = make_app()
        .oneshot(post_json("/api/v1/solve", &body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn docs_returns_html() {
    let resp = make_app().oneshot(get("/docs")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();
    assert!(
        body.contains("<!doctype html>") || body.contains("<!DOCTYPE html>"),
        "expected HTML doctype in /docs response"
    );
}

// ---------------------------------------------------------------------------
// Auth (require_auth applied on top of build_api_router() — see make_authed_app)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_open_without_token_when_auth_enabled() {
    let resp = make_authed_app()
        .oneshot(get("/api/v1/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_path_documented_in_openapi_matches_auth_exemption() {
    // Ties AUTH_EXEMPT_PATH (a private const in middleware.rs) to the actual
    // health route via an independent source of truth: the OpenAPI spec
    // generated from routes/health.rs's #[utoipa::path] annotation, rather
    // than re-hardcoding "/api/v1/health" a third time in this test. If the
    // route is ever renamed in one place but not the others, this fails
    // instead of the exemption silently breaking in production.
    let openapi_resp = make_authed_app()
        .oneshot(get("/openapi.json"))
        .await
        .unwrap();
    let openapi = json_body(openapi_resp).await;
    let paths = openapi["paths"].as_object().unwrap();
    let health_path = paths
        .keys()
        .find(|p| p.contains("health"))
        .expect("openapi spec must document a health path");

    let resp = make_authed_app().oneshot(get(health_path)).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "path {health_path} documented in OpenAPI spec must be exempt from auth"
    );
}

#[tokio::test]
async fn solvers_without_token_returns_401() {
    let resp = make_authed_app()
        .oneshot(get("/api/v1/solvers"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert!(
        resp.headers().contains_key("www-authenticate"),
        "401 response must include WWW-Authenticate per RFC 7235 §3.1"
    );
    let json = json_body(resp).await;
    assert_eq!(json["error"], "Unauthorized");
}

#[tokio::test]
async fn solvers_with_wrong_token_returns_401() {
    let resp = make_authed_app()
        .oneshot(get_with_header(
            "/api/v1/solvers",
            "Authorization",
            "Bearer wrong-token",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn solvers_with_correct_bearer_token_returns_200() {
    let resp = make_authed_app()
        .oneshot(get_with_header(
            "/api/v1/solvers",
            "Authorization",
            &format!("Bearer {TEST_TOKEN}"),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn solvers_with_correct_x_api_key_returns_200() {
    let resp = make_authed_app()
        .oneshot(get_with_header("/api/v1/solvers", "X-Api-Key", TEST_TOKEN))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn solvers_with_non_bearer_auth_scheme_returns_401() {
    // Locks in that a non-Bearer Authorization scheme (e.g. Basic) is rejected
    // rather than silently falling through to the X-Api-Key branch unverified.
    let resp = make_authed_app()
        .oneshot(get_with_header(
            "/api/v1/solvers",
            "Authorization",
            "Basic dXNlcjpwYXNz",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn solvers_with_lowercase_bearer_scheme_returns_200() {
    // RFC 7235 §2.1: the auth-scheme token is case-insensitive, so
    // "bearer" (lowercase, as some HTTP clients default to) must be
    // accepted just like "Bearer".
    let resp = make_authed_app()
        .oneshot(get_with_header(
            "/api/v1/solvers",
            "Authorization",
            &format!("bearer {TEST_TOKEN}"),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn parse_without_token_returns_401() {
    let body = format!(r#"{{"input":{TINY_CITIES}}}"#);
    let req = post_json("/api/v1/parse", &body);
    let resp = make_authed_app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn empty_configured_token_never_matches_empty_presented_token() {
    // Regression test for a real bypass: an empty configured token must
    // never authenticate an empty (but present) credential. This is a
    // defense-in-depth check at the require_auth/token_matches level — the
    // actual fix is api_key() in main.rs never producing an empty
    // Some(String) in the first place, which isn't unit-testable here
    // since it reads a process-wide env var.
    let state = AppState {
        solver_service: Arc::new(MockSolverService),
        registry_service: Arc::new(MockRegistry),
        metrics: Arc::new(MetricsState::new()),
    };
    let api = middleware::require_auth(teeline_api::build_api_router(), "");
    let app = teeline_api::build_router(state, api);

    let resp = app
        .oneshot(get_with_header("/api/v1/solvers", "X-Api-Key", ""))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unmatched_path_returns_404_not_401_when_auth_enabled() {
    // Regression test: require_auth must apply the middleware via
    // route_layer (not layer), or it wraps the router's fallback too and
    // turns every unmatched path into a 401 instead of axum's normal 404 —
    // including paths entirely outside /api/v1/* once merged into the full app.
    let resp = make_authed_app()
        .oneshot(get("/api/v1/this-route-does-not-exist"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

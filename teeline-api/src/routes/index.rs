use axum::Json;
use utoipa::ToSchema;

const ROUTES: &[&str] = &[
    "GET /",
    "GET /api/v1/health",
    "GET /healthz",
    "GET /api/v1/solvers",
    "POST /api/v1/parse",
    "POST /api/v1/solve",
    "GET /openapi.json",
    "GET /docs",
];

#[derive(serde::Serialize, ToSchema)]
pub struct IndexResponse {
    pub status: &'static str,
    pub name: &'static str,
    pub routes: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "API index with available routes", body = IndexResponse)
    )
)]
pub async fn handler() -> Json<IndexResponse> {
    Json(IndexResponse {
        status: "ok",
        name: env!("CARGO_PKG_NAME"),
        routes: ROUTES.iter().map(|r| r.to_string()).collect(),
    })
}

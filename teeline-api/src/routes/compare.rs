use axum::{Json, extract::State};

use crate::{
    AppState,
    error::{ApiError, ApiResult},
    models::{request::CompareRequest, response::CompareResponse},
};

#[utoipa::path(
    post,
    path = "/api/v1/compare",
    request_body = CompareRequest,
    responses(
        (status = 200, description = "Solver comparison results", body = CompareResponse),
        (status = 400, description = "Invalid input or fewer than 2 solvers")
    )
)]
pub async fn compare(
    State(state): State<AppState>,
    Json(req): Json<CompareRequest>,
) -> ApiResult<Json<CompareResponse>> {
    state
        .solver_service
        .compare(&req)
        .await
        .map(Json)
        .map_err(ApiError::BadRequest)
}

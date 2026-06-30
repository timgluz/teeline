use axum::{Json, extract::State};

use crate::{
    AppState,
    error::{ApiError, ApiResult},
    models::{request::SolveRequest, response::SolveResponse},
};

#[utoipa::path(
    post,
    path = "/api/v1/solve",
    request_body = SolveRequest,
    responses(
        (status = 200, description = "Solved tour", body = SolveResponse),
        (status = 400, description = "Invalid input or unknown solver"),
        (status = 500, description = "Solver failure")
    )
)]
pub async fn solve(
    State(state): State<AppState>,
    Json(req): Json<SolveRequest>,
) -> ApiResult<Json<SolveResponse>> {
    state
        .solver_service
        .solve(&req)
        .await
        .map(Json)
        .map_err(|e| {
            if e.starts_with("task panic:") {
                ApiError::Internal(e)
            } else {
                ApiError::BadRequest(e)
            }
        })
}

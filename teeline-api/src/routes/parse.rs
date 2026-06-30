use axum::{Json, extract::State};

use crate::{
    AppState,
    error::{ApiError, ApiResult},
    models::{request::ParseRequest, response::ParseResponse},
};

#[utoipa::path(
    post,
    path = "/api/v1/parse",
    request_body = ParseRequest,
    responses(
        (status = 200, description = "Parsed city list", body = ParseResponse),
        (status = 400, description = "Invalid input or malformed TSPLIB")
    )
)]
pub async fn parse(
    State(state): State<AppState>,
    Json(req): Json<ParseRequest>,
) -> ApiResult<Json<ParseResponse>> {
    state
        .solver_service
        .parse(&req)
        .await
        .map(Json)
        .map_err(ApiError::BadRequest)
}

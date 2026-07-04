use axum::{Json, extract::State};

use crate::{
    AppState,
    error::{ApiError, ApiResult},
    models::{request::ParseRequest, response::ParseResponse},
};

#[utoipa::path(
    post,
    path = "/api/v1/parse",
    security(
        ("bearer_token" = []),
        ("api_key" = [])
    ),
    request_body(
        content = ParseRequest,
        examples(
            ("Cities array" = (
                summary = "Inline city coordinates as JSON",
                value = json!({
                    "input": {
                        "cities": [
                            {"x": 0.0, "y": 0.0},
                            {"x": 1.0, "y": 0.0},
                            {"x": 0.5, "y": 1.0}
                        ]
                    }
                })
            )),
            ("TSPLIB format" = (
                summary = "Standard TSPLIB text format",
                value = json!({
                    "input": {
                        "tsplib": "NAME: tiny\nTYPE: TSP\nCOMMENT: three cities\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EUC_2D\nNODE_COORD_SECTION\n1 0.0 0.0\n2 1.0 0.0\n3 0.5 1.0\nEOF\n"
                    }
                })
            ))
        )
    ),
    responses(
        (status = 200, description = "Parsed city list", body = ParseResponse),
        (status = 400, description = "Invalid input or malformed TSPLIB"),
        (status = 401, description = "Missing or invalid API key")
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

use axum::{Json, extract::State};

use crate::{
    AppState,
    error::{ApiError, ApiResult},
    models::{request::PipelineRequest, response::PipelineResponse},
};

#[utoipa::path(
    post,
    path = "/api/v1/pipeline",
    security(
        ("bearer_token" = []),
        ("api_key" = [])
    ),
    request_body(
        content = PipelineRequest,
        examples(
            ("Constructive then local search" = (
                summary = "Nearest neighbor seeds a 2-opt cleanup pass",
                value = json!({
                    "input": {
                        "cities": [
                            {"x": 0.0, "y": 0.0},
                            {"x": 1.0, "y": 0.0},
                            {"x": 1.0, "y": 1.0},
                            {"x": 0.0, "y": 1.0}
                        ]
                    },
                    "stages": [
                        {"solver": "nn"},
                        {"solver": "2opt"}
                    ]
                })
            )),
            ("Per-stage tuned metaheuristic" = (
                summary = "2-opt cleanup, then SA tuned for a lower starting temperature",
                value = json!({
                    "input": {
                        "cities": [
                            {"x": 0.0, "y": 0.0},
                            {"x": 1.0, "y": 0.0},
                            {"x": 1.0, "y": 1.0},
                            {"x": 0.0, "y": 1.0}
                        ]
                    },
                    "stages": [
                        {"solver": "2opt"},
                        {"solver": "sa", "configs": {"sa": {"max_temperature": 200.0}}}
                    ]
                })
            )),
            ("Classic three-stage pipeline" = (
                summary = "nn -> 2opt -> sa: the standard constructor -> local search -> metaheuristic chain",
                value = json!({
                    "input": {
                        "cities": [
                            {"x": 0.0, "y": 0.0},
                            {"x": 1.0, "y": 0.0},
                            {"x": 1.0, "y": 1.0},
                            {"x": 0.0, "y": 1.0},
                            {"x": 0.5, "y": 0.5}
                        ]
                    },
                    "stages": [
                        {"solver": "nn"},
                        {"solver": "2opt"},
                        {"solver": "sa"}
                    ]
                })
            ))
        )
    ),
    responses(
        (status = 200, description = "Pipeline result", body = PipelineResponse),
        (status = 400, description = "Invalid input, unknown solver, or fewer than 2 stages"),
        (status = 401, description = "Missing or invalid API key"),
        (status = 500, description = "Solver failure")
    )
)]
pub async fn pipeline(
    State(state): State<AppState>,
    Json(req): Json<PipelineRequest>,
) -> ApiResult<Json<PipelineResponse>> {
    state
        .solver_service
        .pipeline(&req)
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

use axum::{Json, extract::State};

use crate::{
    AppState,
    error::{ApiError, ApiResult},
    metrics::{SolverDurationLabels, SolverLabels},
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
        (status = 400, description = "Invalid input, unknown solver, or stage count outside [2, 20]"),
        (status = 401, description = "Missing or invalid API key"),
        (status = 500, description = "Solver failure")
    )
)]
pub async fn pipeline(
    State(state): State<AppState>,
    Json(req): Json<PipelineRequest>,
) -> ApiResult<Json<PipelineResponse>> {
    let result = state.solver_service.pipeline(&req).await;

    // Same cardinality guard as /api/v1/solve: only record metrics for known
    // aliases, since an unknown solver name is arbitrary client-controlled
    // input and would otherwise let a caller mint unbounded label values.
    // Unlike solve() (one solver per request), a pipeline chains several
    // stages, so every known-alias stage gets its own counter increment, and
    // (on success) its own duration observation from that stage's own
    // measured `duration_ms` — a more precise per-stage timing than solve()'s
    // single request-wide `Instant::now()` delta.
    let known_aliases: std::collections::HashSet<String> = state
        .registry_service
        .list()
        .into_iter()
        .map(|s| s.alias)
        .collect();
    let status = if result.is_ok() { "success" } else { "error" };
    for stage in &req.stages {
        if !known_aliases.contains(&stage.solver) {
            continue;
        }
        state
            .metrics
            .solver_requests_total
            .get_or_create(&SolverLabels {
                solver: stage.solver.clone(),
                status: status.into(),
            })
            .inc();
    }
    if let Ok(resp) = &result {
        for stage_result in &resp.stages {
            state
                .metrics
                .solver_duration_seconds
                .get_or_create(&SolverDurationLabels {
                    solver: stage_result.solver.clone(),
                })
                .observe(stage_result.duration_ms as f64 / 1000.0);
        }
    }

    result.map(Json).map_err(ApiError::from_solver_error)
}

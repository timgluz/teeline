use std::time::Instant;

use axum::{Json, extract::State};

use crate::{
    AppState,
    error::{ApiError, ApiResult},
    metrics::{SolverDurationLabels, SolverLabels},
    models::{request::SolveRequest, response::SolveResponse},
};

#[utoipa::path(
    post,
    path = "/api/v1/solve",
    security(
        ("bearer_token" = []),
        ("api_key" = [])
    ),
    request_body(
        content = SolveRequest,
        examples(
            ("Cities array (Nearest Neighbor)" = (
                summary = "Inline coordinates, fast constructive solver",
                value = json!({
                    "solver": "nn",
                    "input": {
                        "cities": [
                            {"x": 0.0, "y": 0.0},
                            {"x": 1.0, "y": 0.0},
                            {"x": 0.5, "y": 1.0}
                        ]
                    }
                })
            )),
            ("TSPLIB format (2-opt)" = (
                summary = "TSPLIB-formatted problem, local-search solver",
                value = json!({
                    "solver": "2opt",
                    "input": {
                        "tsplib": "NAME: tiny\nTYPE: TSP\nCOMMENT: three cities\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EUC_2D\nNODE_COORD_SECTION\n1 0.0 0.0\n2 1.0 0.0\n3 0.5 1.0\nEOF\n"
                    }
                })
            )),
            ("Tuned heuristic (Simulated Annealing)" = (
                summary = "Cities array with custom solver options",
                value = json!({
                    "solver": "sa",
                    "input": {
                        "cities": [
                            {"x": 0.0, "y": 0.0},
                            {"x": 1.0, "y": 0.0},
                            {"x": 0.5, "y": 1.0}
                        ]
                    },
                    "configs": {
                        "sa": {
                            "cooling_rate": 0.0005,
                            "heuristic": {"epochs": 5000}
                        }
                    }
                })
            ))
        )
    ),
    responses(
        (status = 200, description = "Solved tour", body = SolveResponse),
        (status = 400, description = "Invalid input or unknown solver"),
        (status = 401, description = "Missing or invalid API key"),
        (status = 500, description = "Solver failure")
    )
)]
pub async fn solve(
    State(state): State<AppState>,
    Json(req): Json<SolveRequest>,
) -> ApiResult<Json<SolveResponse>> {
    let start = Instant::now();
    let result = state.solver_service.solve(&req).await;
    let elapsed = start.elapsed().as_secs_f64();

    // Only record solver metrics for known aliases to prevent client-controlled
    // label cardinality (an unknown solver name is arbitrary user input).
    let is_known = state
        .registry_service
        .list()
        .iter()
        .any(|s| s.alias == req.solver);
    if is_known {
        let status = if result.is_ok() { "success" } else { "error" };
        state
            .metrics
            .solver_requests_total
            .get_or_create(&SolverLabels {
                solver: req.solver.clone(),
                status: status.into(),
            })
            .inc();
        // Duration only recorded on success — error path typically fails before
        // the solver runs (unknown alias, invalid input) so elapsed is meaningless.
        if result.is_ok() {
            state
                .metrics
                .solver_duration_seconds
                .get_or_create(&SolverDurationLabels {
                    solver: req.solver.clone(),
                })
                .observe(elapsed);
        }
    }

    result.map(Json).map_err(|e| {
        if e.starts_with("task panic:") {
            ApiError::Internal(e)
        } else {
            ApiError::BadRequest(e)
        }
    })
}

use axum::{Json, extract::State};

use crate::{AppState, models::response::AlgorithmInfo};

#[utoipa::path(
    get,
    path = "/api/v1/solvers",
    responses(
        (status = 200, description = "List of all TSP solvers", body = Vec<AlgorithmInfo>)
    )
)]
pub async fn list_solvers(State(state): State<AppState>) -> Json<Vec<AlgorithmInfo>> {
    Json(state.registry_service.list())
}

// Response DTOs
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Mirrors `SolverInfo` from `teeline::tsp`. Field names match the lib (`desc`, `alias`).
/// `alias` is always present — every solver in `SOLVER_LIST` has a non-empty alias.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AlgorithmInfo {
    pub name: String,
    pub alias: String,
    pub category: String,
    pub desc: String,
    pub complexity: String,
    pub has_options: bool,
    pub exact: bool,
}

/// A city in a parsed or solved tour. `id` is 1-based, matching the lib.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CityDto {
    pub id: usize,
    pub x: f32,
    pub y: f32,
}

/// `distance_type` is the TSPLIB canonical uppercase string, e.g. `"EUC_2D"` or `"GEO"`.
/// The service layer maps `DistanceType` enum → String before building this response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ParseResponse {
    pub name: String,
    pub comment: String,
    pub distance_type: String,
    pub cities: Vec<CityDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SolveResponse {
    /// The solver alias that produced this result (e.g. `"nn"`, `"sa"`).
    pub solver: String,
    /// Total tour distance (sum of EUC_2D edges, closing the cycle).
    pub total: f32,
    /// Ordered 1-based city IDs. Read from `Solution::route()` — the field is private in the lib.
    pub route: Vec<usize>,
    /// Wall-clock time for the solve call, measured by the service layer.
    pub duration_ms: u64,
}

/// One stage's result within a `PipelineResponse`. Named `PipelineStageResult` to
/// avoid colliding with `teeline::tsp::pipeline::PipelineStage` (the lib's
/// stage-input type) and with `pipeline::StageOutcome` (the lib's own output type,
/// which has no `solver` field — this DTO's `solver` is echoed straight from the
/// request, same convention as `SolveResponse.solver`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PipelineStageResult {
    pub solver: String,
    pub cost: f32,
    pub tour: Vec<usize>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PipelineResponse {
    pub stages: Vec<PipelineStageResult>,
    pub final_cost: f32,
    pub final_tour: Vec<usize>,
    /// Non-fatal composition warnings, e.g. an `nn` stage discarding the
    /// warm-start seed, or an exact solver in a non-terminal position.
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorithm_info_round_trip() {
        let info = AlgorithmInfo {
            name: "Nearest Neighbor".to_string(),
            alias: "nn".to_string(),
            category: "Constructive".to_string(),
            desc: "Greedy heuristic.".to_string(),
            complexity: "O(n²)".to_string(),
            has_options: false,
            exact: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: AlgorithmInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.alias, "nn");
        assert_eq!(back.desc, "Greedy heuristic.");
        // field serializes as "desc", not "description"
        assert!(json.contains(r#""desc":"#));
    }

    #[test]
    fn city_dto_round_trip() {
        let city = CityDto {
            id: 7,
            x: 1.5,
            y: 2.5,
        };
        let json = serde_json::to_string(&city).unwrap();
        let back: CityDto = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 7);
    }

    #[test]
    fn parse_response_round_trip() {
        let resp = ParseResponse {
            name: "berlin52".to_string(),
            comment: "52 locations in Berlin".to_string(),
            distance_type: "EUC_2D".to_string(),
            cities: vec![CityDto {
                id: 1,
                x: 565.0,
                y: 575.0,
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: ParseResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.distance_type, "EUC_2D");
        assert_eq!(back.cities.len(), 1);
    }

    #[test]
    fn solve_response_round_trip() {
        let resp = SolveResponse {
            solver: "nn".to_string(),
            total: 1234.5,
            route: vec![1, 3, 2],
            duration_ms: 42,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: SolveResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.solver, "nn");
        assert_eq!(back.total, 1234.5_f32);
        assert_eq!(back.route, vec![1, 3, 2]);
        assert_eq!(back.duration_ms, 42);
    }

    #[test]
    fn solve_response_schema_builds() {
        use utoipa::ToSchema;
        let name = SolveResponse::name();
        assert_eq!(name, "SolveResponse");
    }

    #[test]
    fn pipeline_response_round_trip() {
        let resp = PipelineResponse {
            stages: vec![
                PipelineStageResult {
                    solver: "nn".to_string(),
                    cost: 2000.0,
                    tour: vec![1, 2, 3],
                    duration_ms: 1,
                },
                PipelineStageResult {
                    solver: "2opt".to_string(),
                    cost: 1500.0,
                    tour: vec![1, 3, 2],
                    duration_ms: 5,
                },
            ],
            final_cost: 1500.0,
            final_tour: vec![1, 3, 2],
            warnings: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: PipelineResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.stages.len(), 2);
        assert_eq!(back.stages[0].solver, "nn");
        assert_eq!(back.final_cost, 1500.0_f32);
        assert_eq!(back.final_tour, vec![1, 3, 2]);
        assert!(back.warnings.is_empty());
    }

    #[test]
    fn pipeline_response_schema_builds() {
        use utoipa::ToSchema;
        assert_eq!(PipelineResponse::name(), "PipelineResponse");
        assert_eq!(PipelineStageResult::name(), "PipelineStageResult");
    }
}

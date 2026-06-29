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

/// A per-solver result in a compare response.
///
/// Tagged enum: JSON carries `"status": "ok"` or `"status": "error"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum CompareEntry {
    Ok {
        solver: String,
        total: f32,
        route: Vec<usize>,
        duration_ms: u64,
    },
    Error {
        solver: String,
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CompareResponse {
    pub entries: Vec<CompareEntry>,
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
        assert_eq!(back.route, vec![1, 3, 2]);
        assert_eq!(back.duration_ms, 42);
    }

    #[test]
    fn compare_entry_ok_has_status_ok() {
        let entry = CompareEntry::Ok {
            solver: "nn".to_string(),
            total: 100.0,
            route: vec![1, 2, 3],
            duration_ms: 5,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains(r#""status":"ok""#));
        assert!(!json.contains("error"));
    }

    #[test]
    fn compare_entry_error_has_status_error() {
        let entry = CompareEntry::Error {
            solver: "bhk".to_string(),
            error: "timeout".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains(r#""status":"error""#));
        assert!(json.contains("timeout"));
    }

    #[test]
    fn compare_response_round_trip() {
        let resp = CompareResponse {
            entries: vec![
                CompareEntry::Ok {
                    solver: "nn".to_string(),
                    total: 100.0,
                    route: vec![1, 2, 3],
                    duration_ms: 5,
                },
                CompareEntry::Error {
                    solver: "bhk".to_string(),
                    error: "too many cities".to_string(),
                },
            ],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: CompareResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entries.len(), 2);
    }

    #[test]
    fn solve_response_schema_builds() {
        use utoipa::ToSchema;
        let name = SolveResponse::name();
        assert_eq!(name, "SolveResponse");
    }

    #[test]
    fn compare_entry_schema_builds() {
        use utoipa::ToSchema;
        let name = CompareEntry::name();
        assert_eq!(name, "CompareEntry");
    }
}

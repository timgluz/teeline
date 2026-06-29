// Request DTOs
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct HeuristicConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epochs: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platoo_epochs: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_nearest: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct NnConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TwoOptConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ThreeOptConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct OrOptConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TabuConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StochasticHillConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PsoConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct GsaConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
}

/// Lin-Kernighan ILS. `max_depth` mirrors `LKOptions::max_depth` (default 5).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LkConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<usize>,
}

/// Simulated Annealing. Mirrors `SAOptions`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SaConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooling_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_temperature: Option<f32>,
}

/// Genetic Algorithm. Mirrors `GAOptions`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct GaConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_probability: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_elite: Option<usize>,
}

/// Cuckoo Search. Mirrors `CSOptions`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_probability: Option<f32>,
}

/// Flower Pollination Algorithm. Mirrors `FPAOptions`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FpaConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_probability: Option<f32>,
}

/// Kohonen SOM. Mirrors `SOMOptions` (f64 fields match the lib).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SomConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epochs: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius_fraction: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neuron_multiplier: Option<usize>,
}

/// Fourier constructive solver. Mirrors `FourierOptions` (f64 fields match the lib).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FourierConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epochs: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k_max: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub m: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lambda: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lambda_decay: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lr: Option<f64>,
}

/// Per-solver optional configuration. Pass only the field matching your chosen solver.
/// BHK, branch_bound, and Christofides have no tunable parameters and are absent.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SolverConfigs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nn: Option<NnConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_opt: Option<TwoOptConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub three_opt: Option<ThreeOptConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub or_opt: Option<OrOptConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tabu: Option<TabuConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stochastic_hill: Option<StochasticHillConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pso: Option<PsoConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gsa: Option<GsaConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lk: Option<LkConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sa: Option<SaConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ga: Option<GaConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cs: Option<CsConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fpa: Option<FpaConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub som: Option<SomConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fourier: Option<FourierConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solver_configs_sa_round_trip() {
        let configs = SolverConfigs {
            sa: Some(SaConfig {
                heuristic: Some(HeuristicConfig {
                    epochs: Some(5000),
                    platoo_epochs: None,
                    n_nearest: None,
                }),
                cooling_rate: Some(0.0005),
                min_temperature: None,
                max_temperature: Some(500.0),
            }),
            ..SolverConfigs::default()
        };
        let json = serde_json::to_string(&configs).unwrap();
        let back: SolverConfigs = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sa.as_ref().unwrap().cooling_rate, Some(0.0005));
        assert_eq!(back.sa.as_ref().unwrap().min_temperature, None);
    }
}

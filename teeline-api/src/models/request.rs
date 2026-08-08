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

/// Ant Colony Optimization. Mirrors `AcoOptions`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AcoConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<HeuristicConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beta: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaporation_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ants: Option<usize>,
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
    pub aco: Option<AcoConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub som: Option<SomConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fourier: Option<FourierConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CityInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<usize>,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TspInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cities: Option<Vec<CityInput>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tsplib: Option<String>,
}

impl TspInput {
    pub fn validate(&self) -> Result<(), String> {
        match (&self.cities, &self.tsplib) {
            (Some(cities), None) => {
                if cities.len() < 2 {
                    Err("`cities` must contain at least 2 cities".to_string())
                } else {
                    Ok(())
                }
            }
            (None, Some(tsplib)) => {
                if tsplib.trim().is_empty() {
                    Err("`tsplib` must not be empty".to_string())
                } else {
                    Ok(())
                }
            }
            (Some(_), Some(_)) => {
                Err("exactly one of `cities` or `tsplib` must be set, not both".to_string())
            }
            (None, None) => Err("one of `cities` or `tsplib` must be set".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ParseRequest {
    pub input: TspInput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SolveRequest {
    pub input: TspInput,
    pub solver: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configs: Option<SolverConfigs>,
}

impl SolveRequest {
    pub fn validate(&self) -> Result<(), String> {
        self.input.validate()?;
        if self.solver.trim().is_empty() {
            return Err("`solver` must not be empty".to_string());
        }
        Ok(())
    }
}

/// One stage in a `PipelineRequest`. Named `PipelineStageRequest` (not just
/// `PipelineStage`) to avoid colliding with `teeline::tsp::pipeline::PipelineStage`,
/// which is a different, already-resolved type used internally by the service layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PipelineStageRequest {
    pub solver: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configs: Option<SolverConfigs>,
}

/// Upper bound on the number of stages a single pipeline request may chain.
/// Each stage is a full solver run (potentially an exact, exponential-time
/// solver per CLAUDE.md's guidance on `bellman_karp`/`branch_bound`), and
/// `run_pipeline_stages` runs them strictly sequentially inside one
/// `spawn_blocking` task — with no per-stage or per-request timeout elsewhere
/// in teeline-api, an unbounded `stages` array is an unmetered compute-cost
/// amplifier for a single HTTP request. 20 comfortably covers every
/// documented/realistic pipeline (the docs' examples top out at 3 stages)
/// while capping worst-case cost.
pub const MAX_PIPELINE_STAGES: usize = 20;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PipelineRequest {
    pub input: TspInput,
    pub stages: Vec<PipelineStageRequest>,
}

impl PipelineRequest {
    pub fn validate(&self) -> Result<(), String> {
        self.input.validate()?;
        if self.stages.len() < 2 {
            return Err("`stages` must contain at least 2 solver stages".to_string());
        }
        if self.stages.len() > MAX_PIPELINE_STAGES {
            return Err(format!(
                "`stages` must contain at most {MAX_PIPELINE_STAGES} solver stages"
            ));
        }
        if self.stages.iter().any(|s| s.solver.trim().is_empty()) {
            return Err("`solver` must not be empty".to_string());
        }
        Ok(())
    }
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

    #[test]
    fn solve_request_cities_round_trip() {
        let req = SolveRequest {
            input: TspInput {
                cities: Some(vec![
                    CityInput {
                        id: Some(1),
                        x: 0.0,
                        y: 0.0,
                    },
                    CityInput {
                        id: Some(2),
                        x: 1.0,
                        y: 0.0,
                    },
                    CityInput {
                        id: Some(3),
                        x: 0.5,
                        y: 1.0,
                    },
                ]),
                tsplib: None,
            },
            solver: "nn".to_string(),
            configs: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: SolveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.solver, "nn");
        assert_eq!(back.input.cities.as_ref().unwrap().len(), 3);
        assert!(back.input.tsplib.is_none());
    }

    #[test]
    fn solve_request_with_configs_round_trip() {
        let req = SolveRequest {
            input: TspInput {
                cities: Some(vec![CityInput {
                    id: None,
                    x: 1.0,
                    y: 2.0,
                }]),
                tsplib: None,
            },
            solver: "sa".to_string(),
            configs: Some(SolverConfigs {
                sa: Some(SaConfig {
                    heuristic: Some(HeuristicConfig {
                        epochs: Some(1000),
                        platoo_epochs: None,
                        n_nearest: None,
                    }),
                    cooling_rate: Some(0.001),
                    min_temperature: None,
                    max_temperature: None,
                }),
                ..SolverConfigs::default()
            }),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: SolveRequest = serde_json::from_str(&json).unwrap();
        let sa = back.configs.unwrap().sa.unwrap();
        assert_eq!(sa.cooling_rate, Some(0.001));
        assert_eq!(sa.heuristic.unwrap().epochs, Some(1000));
    }

    #[test]
    fn solve_request_with_aco_config_round_trip() {
        let req = SolveRequest {
            input: TspInput {
                cities: Some(vec![CityInput {
                    id: None,
                    x: 1.0,
                    y: 2.0,
                }]),
                tsplib: None,
            },
            solver: "aco".to_string(),
            configs: Some(SolverConfigs {
                aco: Some(AcoConfig {
                    heuristic: Some(HeuristicConfig {
                        epochs: Some(300),
                        platoo_epochs: None,
                        n_nearest: None,
                    }),
                    alpha: None,
                    beta: Some(3.0),
                    evaporation_rate: Some(0.3),
                    num_ants: Some(40),
                }),
                ..SolverConfigs::default()
            }),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: SolveRequest = serde_json::from_str(&json).unwrap();
        let aco = back.configs.unwrap().aco.unwrap();
        assert_eq!(aco.beta, Some(3.0));
        assert_eq!(aco.evaporation_rate, Some(0.3));
        assert_eq!(aco.num_ants, Some(40));
        assert_eq!(aco.alpha, None);
        assert_eq!(aco.heuristic.unwrap().epochs, Some(300));
    }

    #[test]
    fn tsp_input_validate_rejects_both() {
        let input = TspInput {
            cities: Some(vec![CityInput {
                id: Some(1),
                x: 0.0,
                y: 0.0,
            }]),
            tsplib: Some("NAME: test".to_string()),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn tsp_input_validate_rejects_neither() {
        let input = TspInput {
            cities: None,
            tsplib: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn tsp_input_validate_rejects_empty_cities() {
        let input = TspInput {
            cities: Some(vec![]),
            tsplib: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn tsp_input_validate_rejects_single_city() {
        let input = TspInput {
            cities: Some(vec![CityInput {
                id: Some(1),
                x: 0.0,
                y: 0.0,
            }]),
            tsplib: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn tsp_input_validate_rejects_blank_tsplib() {
        let input = TspInput {
            cities: None,
            tsplib: Some("   ".to_string()),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn tsp_input_validate_accepts_cities_only() {
        let input = TspInput {
            cities: Some(vec![
                CityInput {
                    id: Some(1),
                    x: 0.0,
                    y: 0.0,
                },
                CityInput {
                    id: Some(2),
                    x: 1.0,
                    y: 0.0,
                },
            ]),
            tsplib: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn tsp_input_validate_accepts_tsplib_only() {
        let input = TspInput {
            cities: None,
            tsplib: Some("NAME: test".to_string()),
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn solve_request_validate_rejects_blank_solver() {
        let req = SolveRequest {
            input: TspInput {
                cities: Some(vec![
                    CityInput {
                        id: Some(1),
                        x: 0.0,
                        y: 0.0,
                    },
                    CityInput {
                        id: Some(2),
                        x: 1.0,
                        y: 0.0,
                    },
                ]),
                tsplib: None,
            },
            solver: "   ".to_string(),
            configs: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn solve_request_validate_accepts_valid() {
        let req = SolveRequest {
            input: TspInput {
                cities: Some(vec![
                    CityInput {
                        id: Some(1),
                        x: 0.0,
                        y: 0.0,
                    },
                    CityInput {
                        id: Some(2),
                        x: 1.0,
                        y: 0.0,
                    },
                ]),
                tsplib: None,
            },
            solver: "nn".to_string(),
            configs: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn solve_request_schema_builds() {
        use utoipa::ToSchema;
        let name = SolveRequest::name();
        assert_eq!(name, "SolveRequest");
    }

    fn tiny_cities_input() -> TspInput {
        TspInput {
            cities: Some(vec![
                CityInput {
                    id: Some(1),
                    x: 0.0,
                    y: 0.0,
                },
                CityInput {
                    id: Some(2),
                    x: 1.0,
                    y: 0.0,
                },
            ]),
            tsplib: None,
        }
    }

    #[test]
    fn pipeline_request_validate_rejects_single_stage() {
        let req = PipelineRequest {
            input: tiny_cities_input(),
            stages: vec![PipelineStageRequest {
                solver: "nn".to_string(),
                configs: None,
            }],
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn pipeline_request_validate_rejects_too_many_stages() {
        let req = PipelineRequest {
            input: tiny_cities_input(),
            stages: (0..=MAX_PIPELINE_STAGES)
                .map(|_| PipelineStageRequest {
                    solver: "nn".to_string(),
                    configs: None,
                })
                .collect(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn pipeline_request_validate_accepts_max_stages() {
        let req = PipelineRequest {
            input: tiny_cities_input(),
            stages: (0..MAX_PIPELINE_STAGES)
                .map(|_| PipelineStageRequest {
                    solver: "nn".to_string(),
                    configs: None,
                })
                .collect(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn pipeline_request_validate_rejects_blank_solver() {
        let req = PipelineRequest {
            input: tiny_cities_input(),
            stages: vec![
                PipelineStageRequest {
                    solver: "nn".to_string(),
                    configs: None,
                },
                PipelineStageRequest {
                    solver: "   ".to_string(),
                    configs: None,
                },
            ],
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn pipeline_request_validate_accepts_valid() {
        let req = PipelineRequest {
            input: tiny_cities_input(),
            stages: vec![
                PipelineStageRequest {
                    solver: "nn".to_string(),
                    configs: None,
                },
                PipelineStageRequest {
                    solver: "2opt".to_string(),
                    configs: None,
                },
            ],
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn pipeline_request_round_trip_with_per_stage_configs() {
        let req = PipelineRequest {
            input: tiny_cities_input(),
            stages: vec![
                PipelineStageRequest {
                    solver: "nn".to_string(),
                    configs: None,
                },
                PipelineStageRequest {
                    solver: "sa".to_string(),
                    configs: Some(SolverConfigs {
                        sa: Some(SaConfig {
                            heuristic: None,
                            cooling_rate: Some(0.0005),
                            min_temperature: None,
                            max_temperature: Some(200.0),
                        }),
                        ..SolverConfigs::default()
                    }),
                },
            ],
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: PipelineRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.stages.len(), 2);
        assert_eq!(back.stages[0].configs, None);
        assert_eq!(
            back.stages[1]
                .configs
                .as_ref()
                .unwrap()
                .sa
                .as_ref()
                .unwrap()
                .cooling_rate,
            Some(0.0005)
        );
    }

    #[test]
    fn pipeline_request_schema_builds() {
        use utoipa::ToSchema;
        assert_eq!(PipelineRequest::name(), "PipelineRequest");
        assert_eq!(PipelineStageRequest::name(), "PipelineStageRequest");
    }
}

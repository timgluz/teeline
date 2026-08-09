use std::time::Instant;

use async_trait::async_trait;
use teeline::tsp::distance_matrix::DistanceMatrix;
use teeline::tsp::kdtree::KDPoint;
use teeline::tsp::pipeline::{PipelineStage, run_pipeline_stages, stage_warnings};
use teeline::tsp::tsplib;
use teeline::tsp::{
    AcoOptions, AppOptions, CSOptions, DistanceType, FPAOptions, FourierOptions, GAOptions,
    HeuristicOptions, LKOptions, SAOptions, SOMOptions, Solvers, TspProblem, find_solver,
    solve_problem,
};

use super::TspSolverService;
use crate::models::{
    request::{
        HeuristicConfig, ParseRequest, PipelineRequest, SolveRequest, SolverConfigs, TspInput,
    },
    response::{CityDto, ParseResponse, PipelineResponse, PipelineStageResult, SolveResponse},
};

pub struct TspService;

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

// Maps partial HeuristicConfig onto a caller-supplied base so that solver-specific
// defaults (e.g. LKOptions has n_nearest=5 vs the global default of 3) are preserved
// when the caller omits a field.
fn map_heuristic_onto(h: &HeuristicConfig, base: HeuristicOptions) -> HeuristicOptions {
    HeuristicOptions {
        epochs: h.epochs.unwrap_or(base.epochs),
        platoo_epochs: h.platoo_epochs.unwrap_or(base.platoo_epochs),
        n_nearest: h.n_nearest.unwrap_or(base.n_nearest),
        verbose: false,
    }
}

fn distance_type_str(dt: DistanceType) -> &'static str {
    match dt {
        DistanceType::Euc2D => "EUC_2D",
        DistanceType::Geo => "GEO",
    }
}

fn input_to_problem(input: &TspInput) -> Result<TspProblem, String> {
    if let Some(tsplib_str) = &input.tsplib {
        let data = tsplib::read_from_str(tsplib_str)?;
        let dm = data.distance_matrix()?;
        Ok(TspProblem::new(data.cities().to_vec(), dm))
    } else if let Some(cities) = &input.cities {
        let pts: Vec<KDPoint> = cities
            .iter()
            .enumerate()
            .map(|(i, c)| KDPoint::new_with_id(c.id.unwrap_or(i + 1), &[c.x, c.y]))
            .collect();
        let dm = DistanceMatrix::from_cities(&pts).map_err(|e| e.to_string())?;
        Ok(TspProblem::new(pts, dm))
    } else {
        Err("input must contain either `cities` or `tsplib`".to_string())
    }
}

fn make_app_options(
    solver_name: &str,
    configs: Option<&SolverConfigs>,
) -> Result<AppOptions, String> {
    let Some(cfg) = configs else {
        return Ok(AppOptions::default());
    };

    // Top-level HeuristicOptions is used by solvers that don't embed their own:
    // nn, 2opt, 3opt, or_opt, tabu_search, stochastic_hill, pso, gsa.
    // SA/GA/CS/FPA/LK embed heuristic inside their own options structs.
    let heuristic: Option<HeuristicOptions> = match solver_name {
        "nn" | "nearest_neighbor" => cfg.nn.as_ref().and_then(|c| c.heuristic.as_ref()),
        "2opt" | "two_opt" => cfg.two_opt.as_ref().and_then(|c| c.heuristic.as_ref()),
        "3opt" | "three_opt" => cfg.three_opt.as_ref().and_then(|c| c.heuristic.as_ref()),
        "or_opt" | "or-opt" => cfg.or_opt.as_ref().and_then(|c| c.heuristic.as_ref()),
        "tabu" | "tabu_search" => cfg.tabu.as_ref().and_then(|c| c.heuristic.as_ref()),
        "stochastic_hill" => cfg
            .stochastic_hill
            .as_ref()
            .and_then(|c| c.heuristic.as_ref()),
        "pso" | "particle_swarm" => cfg.pso.as_ref().and_then(|c| c.heuristic.as_ref()),
        "gsa" | "gravitational_search" => cfg.gsa.as_ref().and_then(|c| c.heuristic.as_ref()),
        _ => None,
    }
    .map(|h| map_heuristic_onto(h, HeuristicOptions::default()));
    if let Some(ref h) = heuristic {
        h.validate()?;
    }

    let sa = cfg.sa.as_ref().map(|c| {
        let d = SAOptions::default();
        SAOptions {
            heuristic: c
                .heuristic
                .as_ref()
                .map(|h| map_heuristic_onto(h, d.heuristic.clone()))
                .unwrap_or(d.heuristic),
            cooling_rate: c.cooling_rate.unwrap_or(d.cooling_rate),
            min_temperature: c.min_temperature.unwrap_or(d.min_temperature),
            max_temperature: c.max_temperature.unwrap_or(d.max_temperature),
        }
    });
    if let Some(ref s) = sa {
        s.validate()?;
    }

    let ga = cfg.ga.as_ref().map(|c| {
        let d = GAOptions::default();
        GAOptions {
            heuristic: c
                .heuristic
                .as_ref()
                .map(|h| map_heuristic_onto(h, d.heuristic.clone()))
                .unwrap_or(d.heuristic),
            mutation_probability: c.mutation_probability.unwrap_or(d.mutation_probability),
            n_elite: c.n_elite.unwrap_or(d.n_elite),
        }
    });
    if let Some(ref g) = ga {
        g.validate()?;
    }

    let cs = cfg.cs.as_ref().map(|c| {
        let d = CSOptions::default();
        CSOptions {
            heuristic: c
                .heuristic
                .as_ref()
                .map(|h| map_heuristic_onto(h, d.heuristic.clone()))
                .unwrap_or(d.heuristic),
            mutation_probability: c.mutation_probability.unwrap_or(d.mutation_probability),
        }
    });
    if let Some(ref c) = cs {
        c.validate()?;
    }

    let fpa = cfg.fpa.as_ref().map(|c| {
        let d = FPAOptions::default();
        FPAOptions {
            heuristic: c
                .heuristic
                .as_ref()
                .map(|h| map_heuristic_onto(h, d.heuristic.clone()))
                .unwrap_or(d.heuristic),
            mutation_probability: c.mutation_probability.unwrap_or(d.mutation_probability),
        }
    });
    if let Some(ref f) = fpa {
        f.validate()?;
    }

    // ACO embeds its own HeuristicOptions, but AcoOptions::default() overrides epochs to 150
    // (the global HeuristicOptions default of 10_000 would take minutes per run at ACO's
    // O(ants * n²) per-epoch cost). So map onto ACO's own default — like the LK block above —
    // rather than map_heuristic, which would re-introduce the 10_000 default and the hang
    // AcoOptions::default()'s doc comment warns about.
    let aco = cfg.aco.as_ref().map(|c| {
        let d = AcoOptions::default();
        AcoOptions {
            heuristic: c
                .heuristic
                .as_ref()
                .map(|h| map_heuristic_onto(h, d.heuristic.clone()))
                .unwrap_or(d.heuristic),
            alpha: c.alpha.unwrap_or(d.alpha),
            beta: c.beta.unwrap_or(d.beta),
            evaporation_rate: c.evaporation_rate.unwrap_or(d.evaporation_rate),
            num_ants: c.num_ants.unwrap_or(d.num_ants),
        }
    });
    if let Some(ref a) = aco {
        a.validate()?;
    }

    let lk = cfg.lk.as_ref().map(|c| {
        let d = LKOptions::default();
        LKOptions {
            heuristic: c
                .heuristic
                .as_ref()
                .map(|h| map_heuristic_onto(h, d.heuristic.clone()))
                .unwrap_or(d.heuristic),
            max_depth: c.max_depth.unwrap_or(d.max_depth),
        }
    });
    if let Some(ref l) = lk {
        l.validate()?;
    }

    let fourier = cfg.fourier.as_ref().map(|c| {
        let d = FourierOptions::default();
        FourierOptions {
            k_max: c.k_max.unwrap_or(d.k_max),
            m: c.m.unwrap_or(d.m),
            lambda: c.lambda.unwrap_or(d.lambda),
            lambda_decay: c.lambda_decay.unwrap_or(d.lambda_decay),
            lr: c.lr.unwrap_or(d.lr),
            epochs: c.epochs.unwrap_or(d.epochs),
        }
    });
    if let Some(ref fr) = fourier {
        fr.validate()?;
    }

    let som = cfg.som.as_ref().map(|c| {
        let d = SOMOptions::default();
        SOMOptions {
            epochs: c.epochs.unwrap_or(d.epochs),
            learning_rate: c.learning_rate.unwrap_or(d.learning_rate),
            radius_fraction: c.radius_fraction.unwrap_or(d.radius_fraction),
            neuron_multiplier: c.neuron_multiplier.unwrap_or(d.neuron_multiplier),
        }
    });
    if let Some(ref sm) = som {
        sm.validate()?;
    }

    Ok(AppOptions {
        sa,
        ga,
        cs,
        fpa,
        lk,
        fourier,
        som,
        aco,
        heuristic,
    })
}

// ---------------------------------------------------------------------------
// TspSolverService impl
// ---------------------------------------------------------------------------

#[async_trait]
impl TspSolverService for TspService {
    async fn parse(&self, req: &ParseRequest) -> Result<ParseResponse, String> {
        req.input.validate()?;

        if let Some(tsplib_str) = &req.input.tsplib {
            let data = tsplib::read_from_str(tsplib_str)?;
            let cities = data
                .cities()
                .iter()
                .map(|c| CityDto {
                    id: c.id,
                    x: c.coords[0],
                    y: c.coords[1],
                })
                .collect();
            Ok(ParseResponse {
                name: data.name.clone(),
                comment: data.comment.clone(),
                distance_type: distance_type_str(data.distance_type).to_string(),
                cities,
            })
        } else {
            let input_cities = req
                .input
                .cities
                .as_ref()
                .ok_or_else(|| "input requires `cities` or `tsplib`".to_string())?;
            let cities = input_cities
                .iter()
                .enumerate()
                .map(|(i, c)| CityDto {
                    id: c.id.unwrap_or(i + 1),
                    x: c.x,
                    y: c.y,
                })
                .collect();
            Ok(ParseResponse {
                name: String::new(),
                comment: String::new(),
                distance_type: "EUC_2D".to_string(),
                cities,
            })
        }
    }

    async fn solve(&self, req: &SolveRequest) -> Result<SolveResponse, String> {
        req.validate()?;
        let solver: Solvers = find_solver(&req.solver)?;
        let problem = input_to_problem(&req.input)?;
        let opts = make_app_options(&req.solver, req.configs.as_ref())?;

        let start = Instant::now();
        let solution = tokio::task::spawn_blocking(move || solve_problem(solver, &problem, &opts))
            .await
            .map_err(|e| format!("task panic: {e}"))??;
        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

        Ok(SolveResponse {
            solver: req.solver.clone(),
            total: solution.total,
            route: solution.route().to_vec(),
            duration_ms,
        })
    }

    async fn pipeline(&self, req: &PipelineRequest) -> Result<PipelineResponse, String> {
        req.validate()?;
        let solvers: Vec<Solvers> = req
            .stages
            .iter()
            .map(|s| find_solver(&s.solver))
            .collect::<Result<_, _>>()?;
        let problem = input_to_problem(&req.input)?;
        let warnings = stage_warnings(&solvers);

        let stages: Vec<PipelineStage> = req
            .stages
            .iter()
            .zip(&solvers)
            .map(|(s, &solver)| {
                let opts = make_app_options(&s.solver, s.configs.as_ref())?;
                Ok(PipelineStage::new(solver, opts, problem.clone(), None))
            })
            .collect::<Result<Vec<_>, String>>()?;

        let outcomes = tokio::task::spawn_blocking(move || run_pipeline_stages(&stages))
            .await
            .map_err(|e| format!("task panic: {e}"))??;

        let stage_results: Vec<PipelineStageResult> = outcomes
            .iter()
            .zip(&req.stages)
            .map(|(o, s)| PipelineStageResult {
                solver: s.solver.clone(),
                cost: o.solution.total,
                tour: o.solution.route().to_vec(),
                duration_ms: o.duration_ms,
            })
            .collect();
        let last = outcomes.last().expect("validated >= 2 stages");

        Ok(PipelineResponse {
            stages: stage_results,
            final_cost: last.solution.total,
            final_tour: last.solution.route().to_vec(),
            warnings,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::request::{CityInput, PipelineStageRequest, TspInput};

    const TINY_TSPLIB: &str = "\
NAME: test
TYPE: TSP
COMMENT: three cities
DIMENSION: 3
EDGE_WEIGHT_TYPE: EUC_2D
NODE_COORD_SECTION
1 0.0 0.0
2 1.0 0.0
3 0.5 1.0
EOF
";

    #[tokio::test]
    async fn test_parse_tsplib_input() {
        let service = TspService;
        let req = ParseRequest {
            input: TspInput {
                tsplib: Some(TINY_TSPLIB.to_string()),
                cities: None,
            },
        };
        let resp = service.parse(&req).await.unwrap();
        assert_eq!(resp.name, "test");
        assert_eq!(resp.comment, "three cities");
        assert_eq!(resp.distance_type, "EUC_2D");
        assert_eq!(resp.cities.len(), 3);
        assert_eq!(resp.cities[0].id, 1);
    }

    #[tokio::test]
    async fn test_parse_json_cities_input() {
        let service = TspService;
        let req = ParseRequest {
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
        };
        let resp = service.parse(&req).await.unwrap();
        assert_eq!(resp.name, "");
        assert_eq!(resp.comment, "");
        assert_eq!(resp.distance_type, "EUC_2D");
        assert_eq!(resp.cities.len(), 3);
        assert_eq!(resp.cities[0].id, 1);
        assert_eq!(resp.cities[1].id, 2);
    }

    #[tokio::test]
    async fn test_parse_json_cities_assigns_ids_when_missing() {
        let service = TspService;
        let req = ParseRequest {
            input: TspInput {
                cities: Some(vec![
                    CityInput {
                        id: None,
                        x: 0.0,
                        y: 0.0,
                    },
                    CityInput {
                        id: None,
                        x: 1.0,
                        y: 0.0,
                    },
                    CityInput {
                        id: None,
                        x: 0.5,
                        y: 1.0,
                    },
                ]),
                tsplib: None,
            },
        };
        let resp = service.parse(&req).await.unwrap();
        assert_eq!(resp.cities[0].id, 1);
        assert_eq!(resp.cities[1].id, 2);
        assert_eq!(resp.cities[2].id, 3);
    }

    fn small_pipeline_cities() -> TspInput {
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
                CityInput {
                    id: Some(3),
                    x: 1.0,
                    y: 1.0,
                },
                CityInput {
                    id: Some(4),
                    x: 0.0,
                    y: 1.0,
                },
            ]),
            tsplib: None,
        }
    }

    #[tokio::test]
    async fn test_pipeline_two_stages_returns_ordered_results() {
        let service = TspService;
        let req = PipelineRequest {
            input: small_pipeline_cities(),
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
        let resp = service.pipeline(&req).await.unwrap();
        assert_eq!(resp.stages.len(), 2);
        assert_eq!(resp.stages[0].solver, "nn");
        assert_eq!(resp.stages[1].solver, "2opt");
        assert_eq!(resp.final_cost, resp.stages[1].cost);
        assert_eq!(resp.final_tour, resp.stages[1].tour);
        assert!(resp.stages[1].cost <= resp.stages[0].cost * 1.001);
    }

    #[tokio::test]
    async fn test_pipeline_unknown_solver_errors() {
        let service = TspService;
        let req = PipelineRequest {
            input: small_pipeline_cities(),
            stages: vec![
                PipelineStageRequest {
                    solver: "does_not_exist".to_string(),
                    configs: None,
                },
                PipelineStageRequest {
                    solver: "2opt".to_string(),
                    configs: None,
                },
            ],
        };
        assert!(service.pipeline(&req).await.is_err());
    }

    #[tokio::test]
    async fn test_pipeline_nn_mid_pipeline_produces_warning() {
        let service = TspService;
        let req = PipelineRequest {
            input: small_pipeline_cities(),
            stages: vec![
                PipelineStageRequest {
                    solver: "2opt".to_string(),
                    configs: None,
                },
                PipelineStageRequest {
                    solver: "nn".to_string(),
                    configs: None,
                },
            ],
        };
        let resp = service.pipeline(&req).await.unwrap();
        assert!(!resp.warnings.is_empty());
        assert!(resp.warnings[0].contains("nn"));
    }

    #[tokio::test]
    async fn test_pipeline_well_formed_has_no_warnings() {
        let service = TspService;
        let req = PipelineRequest {
            input: small_pipeline_cities(),
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
        let resp = service.pipeline(&req).await.unwrap();
        assert!(resp.warnings.is_empty());
    }

    // Regression test: `Solvers::from_str` (teeline core) accepts both "tabu" and
    // "tabu_search", but this file's `make_app_options` maps solver-name strings to
    // config sub-structs by hand and only recognised "tabu_search". A request using
    // the short alias resolved fine but silently dropped `configs.tabu`, falling back
    // to defaults with no error. Both spellings must apply the same config.
    #[test]
    fn test_make_app_options_tabu_alias_applies_same_config_as_tabu_search() {
        use crate::models::request::TabuConfig;

        let configs = SolverConfigs {
            tabu: Some(TabuConfig {
                heuristic: Some(HeuristicConfig {
                    n_nearest: Some(7),
                    ..Default::default()
                }),
            }),
            ..Default::default()
        };

        let via_alias = make_app_options("tabu", Some(&configs)).unwrap();
        let via_full_name = make_app_options("tabu_search", Some(&configs)).unwrap();

        assert_eq!(via_alias.heuristic.as_ref().unwrap().n_nearest, 7);
        assert_eq!(via_alias.heuristic, via_full_name.heuristic);
    }

    // AcoOptions::default() overrides epochs to 150 (vs the global HeuristicOptions default of
    // 10_000) because ACO's O(ants * n²) per-epoch cost would hang at 10_000. The aco mapping
    // must preserve that override when `epochs` is omitted — i.e. map onto AcoOptions's own
    // default via map_heuristic_onto, not map_heuristic (which would re-introduce 10_000).
    #[test]
    fn test_make_app_options_aco_preserves_default_epochs_when_omitted() {
        use crate::models::request::AcoConfig;

        let configs = SolverConfigs {
            aco: Some(AcoConfig {
                heuristic: None,
                alpha: None,
                beta: Some(3.0),
                evaporation_rate: None,
                num_ants: Some(40),
            }),
            ..Default::default()
        };

        let opts = make_app_options("aco", Some(&configs)).unwrap();
        let aco = opts.aco.expect("aco config should map through");
        assert_eq!(
            aco.heuristic.epochs, 150,
            "omitting epochs must fall back to ACO's 150 default, not the global 10_000"
        );
        assert_eq!(aco.beta, 3.0);
        assert_eq!(aco.num_ants, 40);
        assert_eq!(aco.alpha, AcoOptions::default().alpha);
        assert_eq!(aco.evaporation_rate, AcoOptions::default().evaporation_rate);
    }

    // LKOptions::default() overrides n_nearest to 5 (vs the global HeuristicOptions
    // default of 3) and epochs to 100 (vs 10_000). The lk mapping must preserve those
    // overrides when the fields are omitted — i.e. map onto LKOptions's own default
    // via map_heuristic_onto, not the global default.
    #[test]
    fn test_make_app_options_lk_preserves_default_overrides_when_omitted() {
        use crate::models::request::LkConfig;

        let configs = SolverConfigs {
            lk: Some(LkConfig {
                heuristic: None,
                max_depth: Some(8),
            }),
            ..Default::default()
        };

        let opts = make_app_options("lk", Some(&configs)).unwrap();
        let lk = opts.lk.expect("lk config should map through");
        assert_eq!(
            lk.heuristic.n_nearest, 5,
            "omitting n_nearest must fall back to LK's 5 default, not the global 3"
        );
        assert_eq!(
            lk.heuristic.epochs, 100,
            "omitting epochs must fall back to LK's 100 default, not the global 10_000"
        );
        assert_eq!(lk.max_depth, 8);
        assert_eq!(
            lk.heuristic.platoo_epochs,
            LKOptions::default().heuristic.platoo_epochs
        );
    }

    // SA/GA/CS/FPA use map_heuristic_onto(h, d.heuristic.clone()) so that a future
    // solver-side Default override can't be silently discarded. These tests assert the
    // mapping is wired correctly — custom fields flow through, solver defaults survive.
    #[test]
    fn test_make_app_options_sa_maps_heuristic_onto_own_default() {
        use crate::models::request::SaConfig;

        let configs = SolverConfigs {
            sa: Some(SaConfig {
                heuristic: Some(HeuristicConfig {
                    n_nearest: Some(7),
                    ..Default::default()
                }),
                cooling_rate: Some(0.005),
                ..Default::default()
            }),
            ..Default::default()
        };

        let opts = make_app_options("sa", Some(&configs)).unwrap();
        let sa = opts.sa.expect("sa config should map through");
        assert_eq!(sa.heuristic.n_nearest, 7);
        assert_eq!(sa.cooling_rate, 0.005);
        assert_eq!(sa.heuristic.epochs, SAOptions::default().heuristic.epochs);
        assert_eq!(sa.min_temperature, SAOptions::default().min_temperature);
    }

    #[test]
    fn test_make_app_options_ga_maps_heuristic_onto_own_default() {
        use crate::models::request::GaConfig;

        let configs = SolverConfigs {
            ga: Some(GaConfig {
                heuristic: Some(HeuristicConfig {
                    epochs: Some(500),
                    ..Default::default()
                }),
                mutation_probability: Some(0.05),
                ..Default::default()
            }),
            ..Default::default()
        };

        let opts = make_app_options("ga", Some(&configs)).unwrap();
        let ga = opts.ga.expect("ga config should map through");
        assert_eq!(ga.heuristic.epochs, 500);
        assert_eq!(ga.mutation_probability, 0.05);
        assert_eq!(
            ga.heuristic.n_nearest,
            GAOptions::default().heuristic.n_nearest
        );
        assert_eq!(ga.n_elite, GAOptions::default().n_elite);
    }

    #[test]
    fn test_make_app_options_cs_maps_heuristic_onto_own_default() {
        use crate::models::request::CsConfig;

        let configs = SolverConfigs {
            cs: Some(CsConfig {
                heuristic: Some(HeuristicConfig {
                    platoo_epochs: Some(200),
                    ..Default::default()
                }),
                mutation_probability: Some(0.1),
            }),
            ..Default::default()
        };

        let opts = make_app_options("cs", Some(&configs)).unwrap();
        let cs = opts.cs.expect("cs config should map through");
        assert_eq!(cs.heuristic.platoo_epochs, 200);
        assert_eq!(cs.mutation_probability, 0.1);
        assert_eq!(cs.heuristic.epochs, CSOptions::default().heuristic.epochs);
    }

    #[test]
    fn test_make_app_options_fpa_maps_heuristic_onto_own_default() {
        use crate::models::request::FpaConfig;

        let configs = SolverConfigs {
            fpa: Some(FpaConfig {
                heuristic: Some(HeuristicConfig {
                    epochs: Some(800),
                    n_nearest: Some(7),
                    ..Default::default()
                }),
                mutation_probability: Some(0.02),
            }),
            ..Default::default()
        };

        let opts = make_app_options("fpa", Some(&configs)).unwrap();
        let fpa = opts.fpa.expect("fpa config should map through");
        assert_eq!(fpa.heuristic.epochs, 800);
        assert_eq!(fpa.heuristic.n_nearest, 7);
        assert_eq!(fpa.mutation_probability, 0.02);
        assert_eq!(
            fpa.heuristic.platoo_epochs,
            FPAOptions::default().heuristic.platoo_epochs
        );
    }

    #[test]
    fn test_make_app_options_rejects_zero_n_nearest() {
        let configs = SolverConfigs {
            sa: Some(crate::models::request::SaConfig {
                heuristic: Some(HeuristicConfig {
                    n_nearest: Some(0),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = make_app_options("sa", Some(&configs)).unwrap_err();
        assert!(
            err.contains("n_nearest"),
            "expected n_nearest error, got: {err}"
        );
    }

    #[test]
    fn test_make_app_options_rejects_aco_zero_ants() {
        let configs = SolverConfigs {
            aco: Some(crate::models::request::AcoConfig {
                num_ants: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = make_app_options("aco", Some(&configs)).unwrap_err();
        assert!(
            err.contains("num_ants"),
            "expected num_ants error, got: {err}"
        );
    }

    #[test]
    fn test_make_app_options_rejects_aco_beta_out_of_range() {
        let configs = SolverConfigs {
            aco: Some(crate::models::request::AcoConfig {
                beta: Some(8.0),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = make_app_options("aco", Some(&configs)).unwrap_err();
        assert!(err.contains("beta"), "expected beta error, got: {err}");
    }

    #[test]
    fn test_make_app_options_rejects_ga_negative_mutation() {
        let configs = SolverConfigs {
            ga: Some(crate::models::request::GaConfig {
                mutation_probability: Some(-0.1),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = make_app_options("ga", Some(&configs)).unwrap_err();
        assert!(
            err.contains("mutation_probability"),
            "expected mutation error, got: {err}"
        );
    }

    #[test]
    fn test_make_app_options_accepts_valid_defaults() {
        let configs = SolverConfigs {
            aco: Some(crate::models::request::AcoConfig::default()),
            ..Default::default()
        };
        assert!(make_app_options("aco", Some(&configs)).is_ok());
    }

    // NaN is not representable in JSON, so Hurl e2e can't test this path.
    // These tests guard the core-side is_finite() defense-in-depth.
    #[test]
    fn test_ga_validate_rejects_nan_mutation() {
        let ga = GAOptions {
            mutation_probability: f32::NAN,
            ..Default::default()
        };
        assert!(ga.validate().is_err());
    }

    #[test]
    fn test_cs_validate_rejects_nan_mutation() {
        let cs = CSOptions {
            mutation_probability: f32::NAN,
            ..Default::default()
        };
        assert!(cs.validate().is_err());
    }

    #[test]
    fn test_fpa_validate_rejects_nan_mutation() {
        let fpa = FPAOptions {
            mutation_probability: f32::NAN,
            ..Default::default()
        };
        assert!(fpa.validate().is_err());
    }
}

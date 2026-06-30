use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use teeline::tsp::distance_matrix::DistanceMatrix;
use teeline::tsp::kdtree::KDPoint;
use teeline::tsp::tsplib;
use teeline::tsp::{
    AppOptions, CSOptions, DistanceType, FPAOptions, FourierOptions, GAOptions, HeuristicOptions,
    LKOptions, SAOptions, SOMOptions, Solvers, TspProblem, find_solver, solve_problem,
};

use super::TspSolverService;
use crate::models::{
    request::{
        CompareRequest, HeuristicConfig, ParseRequest, SolveRequest, SolverConfigs, TspInput,
    },
    response::{CityDto, CompareEntry, CompareResponse, ParseResponse, SolveResponse},
};

pub struct TspService;

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn map_heuristic(h: &HeuristicConfig) -> HeuristicOptions {
    let d = HeuristicOptions::default();
    HeuristicOptions {
        epochs: h.epochs.unwrap_or(d.epochs),
        platoo_epochs: h.platoo_epochs.unwrap_or(d.platoo_epochs),
        n_nearest: h.n_nearest.unwrap_or(d.n_nearest),
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

fn make_app_options(solver_name: &str, configs: Option<&SolverConfigs>) -> AppOptions {
    let Some(cfg) = configs else {
        return AppOptions::default();
    };

    // Top-level HeuristicOptions is used by solvers that don't embed their own:
    // nn, 2opt, 3opt, or_opt, tabu_search, stochastic_hill, pso, gsa.
    // SA/GA/CS/FPA/LK embed heuristic inside their own options structs.
    let heuristic: Option<HeuristicOptions> = match solver_name {
        "nn" => cfg.nn.as_ref().and_then(|c| c.heuristic.as_ref()),
        "2opt" => cfg.two_opt.as_ref().and_then(|c| c.heuristic.as_ref()),
        "3opt" => cfg.three_opt.as_ref().and_then(|c| c.heuristic.as_ref()),
        "or_opt" => cfg.or_opt.as_ref().and_then(|c| c.heuristic.as_ref()),
        "tabu_search" => cfg.tabu.as_ref().and_then(|c| c.heuristic.as_ref()),
        "stochastic_hill" => cfg
            .stochastic_hill
            .as_ref()
            .and_then(|c| c.heuristic.as_ref()),
        "pso" => cfg.pso.as_ref().and_then(|c| c.heuristic.as_ref()),
        "gsa" => cfg.gsa.as_ref().and_then(|c| c.heuristic.as_ref()),
        _ => None,
    }
    .map(map_heuristic);

    let sa = cfg.sa.as_ref().map(|c| {
        let d = SAOptions::default();
        SAOptions {
            heuristic: c
                .heuristic
                .as_ref()
                .map(map_heuristic)
                .unwrap_or(d.heuristic),
            cooling_rate: c.cooling_rate.unwrap_or(d.cooling_rate),
            min_temperature: c.min_temperature.unwrap_or(d.min_temperature),
            max_temperature: c.max_temperature.unwrap_or(d.max_temperature),
        }
    });

    let ga = cfg.ga.as_ref().map(|c| {
        let d = GAOptions::default();
        GAOptions {
            heuristic: c
                .heuristic
                .as_ref()
                .map(map_heuristic)
                .unwrap_or(d.heuristic),
            mutation_probability: c.mutation_probability.unwrap_or(d.mutation_probability),
            n_elite: c.n_elite.unwrap_or(d.n_elite),
        }
    });

    let cs = cfg.cs.as_ref().map(|c| {
        let d = CSOptions::default();
        CSOptions {
            heuristic: c
                .heuristic
                .as_ref()
                .map(map_heuristic)
                .unwrap_or(d.heuristic),
            mutation_probability: c.mutation_probability.unwrap_or(d.mutation_probability),
        }
    });

    let fpa = cfg.fpa.as_ref().map(|c| {
        let d = FPAOptions::default();
        FPAOptions {
            heuristic: c
                .heuristic
                .as_ref()
                .map(map_heuristic)
                .unwrap_or(d.heuristic),
            mutation_probability: c.mutation_probability.unwrap_or(d.mutation_probability),
        }
    });

    let lk = cfg.lk.as_ref().map(|c| {
        let d = LKOptions::default();
        LKOptions {
            heuristic: c
                .heuristic
                .as_ref()
                .map(map_heuristic)
                .unwrap_or(d.heuristic),
            max_depth: c.max_depth.unwrap_or(d.max_depth),
        }
    });

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

    let som = cfg.som.as_ref().map(|c| {
        let d = SOMOptions::default();
        SOMOptions {
            epochs: c.epochs.unwrap_or(d.epochs),
            learning_rate: c.learning_rate.unwrap_or(d.learning_rate),
            radius_fraction: c.radius_fraction.unwrap_or(d.radius_fraction),
            neuron_multiplier: c.neuron_multiplier.unwrap_or(d.neuron_multiplier),
        }
    });

    AppOptions {
        sa,
        ga,
        cs,
        fpa,
        lk,
        fourier,
        som,
        heuristic,
    }
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
            let input_cities = req.input.cities.as_ref().unwrap();
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
        let opts = make_app_options(&req.solver, req.configs.as_ref());

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

    async fn compare(&self, req: &CompareRequest) -> Result<CompareResponse, String> {
        req.validate()?;
        let problem = Arc::new(input_to_problem(&req.input)?);

        let mut handles: Vec<(
            String,
            tokio::task::JoinHandle<Result<(String, f32, Vec<usize>, u64), String>>,
        )> = Vec::new();

        for solver_name in &req.solvers {
            let sname = solver_name.clone();
            let fallback = solver_name.clone();
            let prob = Arc::clone(&problem);
            let opts = make_app_options(solver_name, req.configs.as_ref());

            let handle = tokio::task::spawn_blocking(move || {
                let solver = find_solver(&sname)?;
                let start = Instant::now();
                let solution = solve_problem(solver, &prob, &opts)?;
                let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
                Ok((
                    sname,
                    solution.total,
                    solution.route().to_vec(),
                    duration_ms,
                ))
            });
            handles.push((fallback, handle));
        }

        let mut entries: Vec<CompareEntry> = Vec::new();
        for (fallback, handle) in handles {
            match handle.await {
                Ok(Ok((solver, total, route, duration_ms))) => {
                    entries.push(CompareEntry::Ok {
                        solver,
                        total,
                        route,
                        duration_ms,
                    });
                }
                Ok(Err(e)) => {
                    entries.push(CompareEntry::Error {
                        solver: fallback,
                        error: e,
                    });
                }
                Err(join_err) => {
                    entries.push(CompareEntry::Error {
                        solver: fallback,
                        error: format!("task panic: {join_err}"),
                    });
                }
            }
        }

        // Ok entries sorted by cost ascending; Error entries appended last.
        entries.sort_by(|a, b| match (a, b) {
            (CompareEntry::Ok { total: ta, .. }, CompareEntry::Ok { total: tb, .. }) => {
                ta.partial_cmp(tb).unwrap_or(std::cmp::Ordering::Equal)
            }
            (CompareEntry::Ok { .. }, CompareEntry::Error { .. }) => std::cmp::Ordering::Less,
            (CompareEntry::Error { .. }, CompareEntry::Ok { .. }) => std::cmp::Ordering::Greater,
            (CompareEntry::Error { .. }, CompareEntry::Error { .. }) => std::cmp::Ordering::Equal,
        });

        Ok(CompareResponse { entries })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::request::{CityInput, TspInput};

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
}

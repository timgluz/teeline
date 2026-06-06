#[allow(warnings)]
mod bindings;

use bindings::Guest;
use bindings::teeline::solver::types::{City, ParsedProblem, Solution, SolveOptions};
use teeline::tsp::{
    AppOptions, CSOptions, FPAOptions, GAOptions, HeuristicOptions, SAOptions, Solvers, TspProblem,
    distance_matrix::DistanceMatrix, kdtree::KDPoint,
};

struct Component;

fn build_opts(solver: Solvers, o: &SolveOptions) -> AppOptions {
    let heuristic = HeuristicOptions {
        epochs: o.epochs as usize,
        platoo_epochs: o.platoo_epochs as usize,
        n_nearest: o.n_nearest as usize,
        verbose: false,
    };
    match solver {
        Solvers::SimulatedAnnealing => AppOptions {
            sa: Some(SAOptions {
                heuristic,
                cooling_rate: o.cooling_rate,
                max_temperature: o.max_temperature,
                min_temperature: o.min_temperature,
            }),
            ..AppOptions::default()
        },
        Solvers::GeneticAlgorithm => AppOptions {
            ga: Some(GAOptions {
                heuristic,
                mutation_probability: o.mutation_probability,
                n_elite: o.n_elite as usize,
            }),
            ..AppOptions::default()
        },
        Solvers::CuckooSearch => AppOptions {
            cs: Some(CSOptions {
                heuristic,
                mutation_probability: o.mutation_probability,
            }),
            ..AppOptions::default()
        },
        Solvers::FlowerPollination => AppOptions {
            fpa: Some(FPAOptions {
                heuristic,
                mutation_probability: o.mutation_probability,
            }),
            ..AppOptions::default()
        },
        _ => AppOptions {
            heuristic: Some(heuristic),
            ..AppOptions::default()
        },
    }
}

fn kd_to_city(c: &KDPoint) -> City {
    City { id: c.id as u32, x: c.x(), y: c.y() }
}

fn solve_with_cities(
    solver: &str,
    kd_cities: Vec<KDPoint>,
    options: &SolveOptions,
) -> Result<Solution, String> {
    let solver_id = teeline::tsp::find_solver(solver)?;
    if kd_cities.len() < 2 {
        return Err("need at least 2 cities".into());
    }
    let distances =
        DistanceMatrix::from_cities(&kd_cities).map_err(|e| format!("distance matrix: {e}"))?;
    let problem = TspProblem::new(kd_cities, distances);
    let opts = build_opts(solver_id, options);
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        teeline::tsp::solve_problem(solver_id, &problem, &opts)
    }))
    .map_err(|_| format!("solver '{solver}' panicked"))?
    .map(|s| Solution {
        total: s.total,
        route: s.route().iter().map(|&id| id as u32).collect(),
    })
}

impl Guest for Component {
    fn solve(solver: String, cities: Vec<City>, options: SolveOptions) -> Result<Solution, String> {
        let kd_cities: Vec<KDPoint> = cities
            .iter()
            .map(|c| KDPoint::new_with_id(c.id as usize, &[c.x, c.y]))
            .collect();
        solve_with_cities(&solver, kd_cities, &options)
    }

    fn parse(input: String) -> Result<ParsedProblem, String> {
        if input.trim_start().starts_with('[') {
            let kd_cities = teeline::tsp::tsplib::parse_json_cities(&input)?;
            Ok(ParsedProblem {
                name: String::new(),
                comment: String::new(),
                distance_type: String::new(),
                cities: kd_cities.iter().map(kd_to_city).collect(),
            })
        } else {
            let data = teeline::tsp::tsplib::read_from_str(&input)?;
            let dt = match data.distance_type {
                teeline::tsp::DistanceType::Euc2D => "EUC_2D",
                teeline::tsp::DistanceType::Geo => "GEO",
            };
            Ok(ParsedProblem {
                name: data.name.clone(),
                comment: data.comment.clone(),
                distance_type: dt.to_string(),
                cities: data.cities().iter().map(kd_to_city).collect(),
            })
        }
    }

    fn parse_and_solve(
        solver: String,
        input: String,
        options: SolveOptions,
    ) -> Result<Solution, String> {
        let kd_cities = if input.trim_start().starts_with('[') {
            teeline::tsp::tsplib::parse_json_cities(&input)?
        } else {
            let data = teeline::tsp::tsplib::read_from_str(&input)?;
            data.cities().to_vec()
        };
        solve_with_cities(&solver, kd_cities, &options)
    }
}

bindings::export!(Component with_types_in bindings);

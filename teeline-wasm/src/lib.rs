#[allow(warnings)]
mod bindings;

use bindings::teeline::solver::types::{City, Solution, SolveOptions};
use bindings::Guest;
use teeline::tsp::{
    distance_matrix::DistanceMatrix, kdtree::KDPoint, AppOptions, CSOptions, FPAOptions,
    GAOptions, HeuristicOptions, SAOptions, Solvers, TspProblem,
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
            cs: Some(CSOptions { heuristic, mutation_probability: o.mutation_probability }),
            ..AppOptions::default()
        },
        Solvers::FlowerPollination => AppOptions {
            fpa: Some(FPAOptions { heuristic, mutation_probability: o.mutation_probability }),
            ..AppOptions::default()
        },
        _ => AppOptions { heuristic: Some(heuristic), ..AppOptions::default() },
    }
}

impl Guest for Component {
    fn solve(
        solver: String,
        cities: Vec<City>,
        options: SolveOptions,
    ) -> Result<Solution, String> {
        let solver_id = teeline::tsp::find_solver(&solver)?;

        let kd_cities: Vec<KDPoint> = cities
            .iter()
            .map(|c| KDPoint::new_with_id(c.id as usize, &[c.x, c.y]))
            .collect();

        let distances = DistanceMatrix::from_cities(&kd_cities)
            .map_err(|e| format!("distance matrix: {e}"))?;

        let problem = TspProblem::new(kd_cities, distances);
        let opts = build_opts(solver_id, &options);

        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            teeline::tsp::solve_problem(solver_id, &problem, &opts)
        }))
        .map_err(|_| format!("solver '{solver}' panicked"))?
        .map(|s| Solution {
            total: s.total,
            route: s.route().iter().map(|&id| id as u32).collect(),
        })
    }
}

bindings::export!(Component with_types_in bindings);

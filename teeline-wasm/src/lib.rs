#[allow(warnings)]
mod bindings;

use bindings::teeline::solver::types::{City, Solution, SolveOptions};
use bindings::Guest;
use teeline::tsp::{distance_matrix::DistanceMatrix, kdtree::KDPoint, SolverOptions};

struct Component;

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

        let opts = SolverOptions {
            epochs: options.epochs as usize,
            platoo_epochs: options.platoo_epochs as usize,
            cooling_rate: options.cooling_rate,
            max_temperature: options.max_temperature,
            min_temperature: options.min_temperature,
            mutation_probability: options.mutation_probability,
            n_elite: options.n_elite as usize,
            n_nearest: options.n_nearest as usize,
            progress_tx: None,
            ..Default::default()
        };

        let s = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            teeline::tsp::solve(solver_id, &kd_cities, &distances, &opts)
        }))
        .map_err(|_| format!("solver '{solver}' panicked"))?;

        Ok(Solution {
            total: s.total,
            route: s.route().iter().map(|&id| id as u32).collect(),
        })
    }
}

bindings::export!(Component with_types_in bindings);

#[allow(warnings)]
mod bindings;

use bindings::teeline::solver::types::{City, Solution, SolveOptions};
use bindings::Guest;
use teeline::tsp::{self, distance_matrix::DistanceMatrix, kdtree::KDPoint, SolverOptions};

struct Component;

impl Guest for Component {
    fn solve(
        solver: String,
        cities: Vec<City>,
        options: SolveOptions,
    ) -> Result<Solution, String> {
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

        // First check for unknown solver before attempting to run
        let solver_fn: Box<dyn FnOnce() -> teeline::tsp::Solution + Send> =
            match solver.as_str() {
                "sa" | "simulated_annealing" => {
                    let (c, d, o) = (kd_cities.clone(), distances.clone(), opts.clone());
                    Box::new(move || tsp::simulated_annealing::solve(&c, &d, &o))
                }
                "2opt" | "two_opt" => {
                    let (c, d, o) = (kd_cities.clone(), distances.clone(), opts.clone());
                    Box::new(move || tsp::two_opt::solve(&c, &d, &o))
                }
                "nn" | "nearest_neighbor" => {
                    let (c, d, o) = (kd_cities.clone(), distances.clone(), opts.clone());
                    Box::new(move || tsp::nearest_neighbor::solve(&c, &d, &o))
                }
                "ga" | "genetic_algorithm" => {
                    let (c, d, o) = (kd_cities.clone(), distances.clone(), opts.clone());
                    Box::new(move || tsp::genetic_algorithm::solve(&c, &d, &o))
                }
                "pso" | "particle_swarm" => {
                    let (c, d, o) = (kd_cities.clone(), distances.clone(), opts.clone());
                    Box::new(move || tsp::particle_swarm::solve(&c, &d, &o))
                }
                "cs" | "cuckoo_search" => {
                    let (c, d, o) = (kd_cities.clone(), distances.clone(), opts.clone());
                    Box::new(move || tsp::cuckoo_search::solve(&c, &d, &o))
                }
                "fpa" | "flower_pollination" => {
                    let (c, d, o) = (kd_cities.clone(), distances.clone(), opts.clone());
                    Box::new(move || tsp::flower_pollination::solve(&c, &d, &o))
                }
                "tabu_search" => {
                    let (c, d, o) = (kd_cities.clone(), distances.clone(), opts.clone());
                    Box::new(move || tsp::tabu_search::solve(&c, &d, &o))
                }
                "stochastic_hill" => {
                    let (c, d, o) = (kd_cities.clone(), distances.clone(), opts.clone());
                    Box::new(move || tsp::stochastic_hill::solve(&c, &d, &o))
                }
                unknown => return Err(format!("unknown solver: {unknown}")),
            };

        let s = std::panic::catch_unwind(std::panic::AssertUnwindSafe(solver_fn))
            .map_err(|_| format!("solver '{solver}' panicked"))?;

        Ok(Solution {
            total: s.total,
            route: s.route().iter().map(|&id| id as u32).collect(),
        })
    }
}

bindings::export!(Component with_types_in bindings);

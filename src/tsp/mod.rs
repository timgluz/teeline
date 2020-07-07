pub mod bellman_karp;
pub mod branch_bound;
pub mod distance_matrix;
pub mod genetic_algorithm;
pub mod kdtree;
pub mod nearest_neighbor;
pub mod route;
pub mod simulated_annealing;
pub mod stochastic_hill;
pub mod tabu_search;
pub mod tour;
pub mod two_opt;

pub const VERSION: &'static str = "0.3.0";
pub const AUTHOR: &'static str = "Timo Sulg <timo@sulg.dev>";

use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub enum Solvers {
    BellmanKarp,
    BranchBound,
    NearestNeighbor,
    GeneticAlgorithm,
    SimulatedAnnealing,
    StochasticHill,
    TabuSearch,
    TwoOpt,
    Unspecified,
}

impl Solvers {
    pub fn variants() -> Vec<&'static str> {
        vec![
            "bellman_karp",
            "bhk",
            "branch_bound",
            "nearest_neighbor",
            "nn",
            "genetic_algorithm",
            "ga",
            "simulated_annealing",
            "sa",
            "stochastic_hill",
            "tabu_search",
            "two_opt",
            "2opt",
        ]
    }
}

impl FromStr for Solvers {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bhk" | "bellman_karp" => Ok(Solvers::BellmanKarp),
            "branch_bound" => Ok(Solvers::BranchBound),
            "nn" | "nearest_neighbor" => Ok(Solvers::NearestNeighbor),
            "ga" | "genetic_algorithm" => Ok(Solvers::GeneticAlgorithm),
            "sa" | "simulated_annealing" => Ok(Solvers::SimulatedAnnealing),
            "stochastic_hill" => Ok(Solvers::StochasticHill),
            "tabu_search" => Ok(Solvers::TabuSearch),
            "2opt" | "two_opt" => Ok(Solvers::TwoOpt),
            _ => Err("unknown solver"),
        }
    }
}

// -- SolverOptions

pub struct SolverOptions {
    pub epochs: usize,        // how many iteration to run
    pub platoo_epochs: usize, // how many iterations to do on the platoo
    pub verbose: bool,
    pub n_nearest: usize,
    pub mutation_probability: f32,
    pub n_elite: usize,
    pub cooling_rate: f32,
    pub max_temperature: f32,
    pub min_temperature: f32,
}

impl SolverOptions {
    pub fn default() -> Self {
        SolverOptions {
            epochs: 10_000,
            platoo_epochs: 500,
            verbose: false,
            n_nearest: 3,
            mutation_probability: 0.001,
            n_elite: 3,
            cooling_rate: 0.0001,
            min_temperature: 0.001,
            max_temperature: 1_000.0,
        }
    }
}

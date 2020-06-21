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
            "bellman_karp" => Ok(Solvers::BellmanKarp),
            "branch_bound" => Ok(Solvers::BranchBound),
            "bhk" => Ok(Solvers::BellmanKarp),
            "nearest_neighbor" => Ok(Solvers::NearestNeighbor),
            "nn" => Ok(Solvers::NearestNeighbor),
            "genetic_algorithm" => Ok(Solvers::GeneticAlgorithm),
            "ga" => Ok(Solvers::GeneticAlgorithm),
            "simulated_annealing" => Ok(Solvers::SimulatedAnnealing),
            "stochastic_hill" => Ok(Solvers::StochasticHill),
            "tabu_search" => Ok(Solvers::TabuSearch),
            "two_opt" => Ok(Solvers::TwoOpt),
            "2opt" => Ok(Solvers::TwoOpt),
            _ => Err("unknown solver"),
        }
    }
}

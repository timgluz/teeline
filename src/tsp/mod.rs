pub mod kdtree;
pub mod nearest_neighbor;
pub mod route;
pub mod simulated_annealing;
pub mod stochastic_hill;
pub mod tabu_search;
pub mod tour;
pub mod two_opt;

pub const VERSION: &'static str = "0.1.0";
pub const AUTHOR: &'static str = "Timo Sulg <timo@sulg.dev>";

use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub enum Solvers {
    NearestNeighbor,
    SimulatedAnnealing,
    StochasticHill,
    TabuSearch,
    TwoOpt,
    Unspecified,
}

impl Solvers {
    pub fn variants() -> Vec<&'static str> {
        vec![
            "nearest_neighbor",
            "nn",
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
            "nearest_neighbor" => Ok(Solvers::NearestNeighbor),
            "nn" => Ok(Solvers::NearestNeighbor),
            "simulated_annealing" => Ok(Solvers::SimulatedAnnealing),
            "stochastic_hill" => Ok(Solvers::StochasticHill),
            "tabu_search" => Ok(Solvers::TabuSearch),
            "two_opt" => Ok(Solvers::TwoOpt),
            "2opt" => Ok(Solvers::TwoOpt),
            _ => Err("unknown solver"),
        }
    }
}

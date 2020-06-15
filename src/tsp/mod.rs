pub mod kdtree;
pub mod nearest_neighbor;
pub mod route;
pub mod simulated_annealing;
pub mod stochastic_hill;
pub mod tabu_search;
pub mod tour;
pub mod two_opt;

#[derive(Clone, Debug)]
pub enum Solvers {
    NearestNeighbor,
    SimulatedAnnealing,
    StochasticHill,
    TabuSearch,
    TwoOpt,
}

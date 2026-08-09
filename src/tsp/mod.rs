pub mod ant_colony;
pub mod bellman_karp;
pub mod branch_bound;
pub mod christofides;
pub mod comparison;
pub use comparison::{ComparisonStats, compare_tours, tour_cost};
pub mod convert;
pub mod cuckoo_search;
pub mod distance_matrix;
pub mod flower_pollination;
pub mod fourier;
pub mod genetic_algorithm;
pub(crate) mod graph;
pub mod gravitational_search;
pub mod greedy_edge;
pub mod kdtree;
pub mod lin_kernighan;
pub mod nearest_neighbor;
pub mod opt_tour;
pub mod or_opt;
pub mod particle_swarm;
pub mod pipeline;
pub mod probability;
pub mod progress;
pub mod random_shuffle;
pub mod route;
pub mod savings;
pub mod simulated_annealing;
pub mod som;
pub mod stochastic_hill;
pub mod tabu_search;
pub mod three_opt;
pub mod tsplib;
pub mod two_opt;

use crate::tsp::distance_matrix::DistanceMatrix;
use crate::tsp::kdtree::KDPoint;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::mpsc;

pub const VERSION: &str = "1.0.1";
pub const AUTHOR: &str = "Timo Sulg <timo@sulg.dev>";

use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Solvers {
    AntColony,
    BellmanKarp,
    BranchBound,
    Christofides,
    Savings,
    CuckooSearch,
    FlowerPollination,
    Fourier,
    LinKernighan,
    NearestNeighbor,
    GeneticAlgorithm,
    GravitationalSearch,
    GreedyEdge,
    OrOpt,
    ParticleSwarmOptimization,
    RandomShuffle,
    SimulatedAnnealing,
    KohonenSom,
    StochasticHill,
    TabuSearch,
    ThreeOpt,
    TwoOpt,
    Unspecified,
}

impl Solvers {
    pub fn variants() -> Vec<&'static str> {
        vec![
            "ant_colony",
            "aco",
            "bellman_karp",
            "bhk",
            "branch_bound",
            "christofides",
            "chr",
            "savings",
            "sav",
            "cs",
            "cuckoo_search",
            "fpa",
            "flower_pollination",
            "fourier",
            "lk",
            "lin_kernighan",
            "nearest_neighbor",
            "nn",
            "genetic_algorithm",
            "ga",
            "gravitational_search",
            "gsa",
            "greedy_edge",
            "gec",
            "particle_swarm",
            "pso",
            "random_shuffle",
            "shuffle",
            "simulated_annealing",
            "sa",
            "som",
            "kohonen",
            "kohonen_som",
            "stochastic_hill",
            "tabu_search",
            "tabu",
            "three_opt",
            "3opt",
            "or_opt",
            "or-opt",
            "two_opt",
            "2opt",
            "classic",
            "fast",
            "thorough",
        ]
    }

    /// Deterministic local-search solvers: monotone hill-climbers that can only reach
    /// solutions reachable from their starting tour. A better start always means a better end.
    /// BranchBound is included because a NN tour seeds the initial upper bound, cutting
    /// proof-of-optimality time from O(n!) toward practical runtimes on ≤20 cities.
    pub fn auto_expand_with_nn(&self) -> bool {
        matches!(
            self,
            Solvers::TwoOpt
                | Solvers::ThreeOpt
                | Solvers::TabuSearch
                | Solvers::BranchBound
                | Solvers::LinKernighan
                | Solvers::OrOpt
        )
    }

    /// Stochastic solvers whose temperature / diversity schedule is calibrated for cold starts.
    /// They benefit from a random shuffle rather than a greedy NN tour: the NN tour's tight
    /// local structure constrains early exploration before the algorithm has warmed up.
    pub fn auto_expand_with_shuffle(&self) -> bool {
        matches!(
            self,
            Solvers::SimulatedAnnealing
                | Solvers::StochasticHill
                | Solvers::GeneticAlgorithm
                | Solvers::GravitationalSearch
                | Solvers::ParticleSwarmOptimization
                | Solvers::CuckooSearch
                | Solvers::FlowerPollination
                | Solvers::Fourier
                | Solvers::AntColony
        )
    }
}

// ---------------------------------------------------------------------------
// Distance type — describes how inter-city distances are computed
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DistanceType {
    #[default]
    Euc2D,
    Geo,
}

impl FromStr for DistanceType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "EUC_2D" | "EUC2D" => Ok(DistanceType::Euc2D),
            "GEO" => Ok(DistanceType::Geo),
            other => Err(format!("unsupported distance type: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Solver catalogue — single source of truth for the `solvers` subcommand
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverKind {
    Exact,
    Heuristic,
    Utility,
}

impl SolverKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SolverKind::Exact => "exact",
            SolverKind::Heuristic => "heuristic",
            SolverKind::Utility => "utility",
        }
    }
}

pub struct SolverMeta {
    pub name: &'static str,
    pub alias: Option<&'static str>,
    pub kind: SolverKind,
}

impl SolverMeta {
    pub fn short(&self) -> &'static str {
        self.alias.unwrap_or(self.name)
    }
}

impl Solvers {
    pub fn all_meta() -> &'static [SolverMeta] {
        &[
            SolverMeta {
                name: "ant_colony",
                alias: Some("aco"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "bellman_karp",
                alias: Some("bhk"),
                kind: SolverKind::Exact,
            },
            SolverMeta {
                name: "branch_bound",
                alias: None,
                kind: SolverKind::Exact,
            },
            // SolverKind::Heuristic is the CLI-facing kind; the WASM-facing SolverInfo uses
            // category: "Approximation" to expose the distinction to the UI. The two registries
            // serve different audiences and intentionally diverge here.
            SolverMeta {
                name: "christofides",
                alias: Some("chr"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "savings",
                alias: Some("sav"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "nearest_neighbor",
                alias: Some("nn"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "two_opt",
                alias: Some("2opt"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "three_opt",
                alias: Some("3opt"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "simulated_annealing",
                alias: Some("sa"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "genetic_algorithm",
                alias: Some("ga"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "gravitational_search",
                alias: Some("gsa"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "greedy_edge",
                alias: Some("gec"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "tabu_search",
                alias: Some("tabu"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "particle_swarm",
                alias: Some("pso"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "cuckoo_search",
                alias: Some("cs"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "flower_pollination",
                alias: Some("fpa"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "fourier",
                alias: Some("fourier"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "lin_kernighan",
                alias: Some("lk"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "or_opt",
                alias: Some("or-opt"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "stochastic_hill",
                alias: None,
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "kohonen_som",
                alias: Some("som"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "random_shuffle",
                alias: Some("shuffle"),
                kind: SolverKind::Utility,
            },
        ]
    }
}

// ---------------------------------------------------------------------------
// UI-facing solver metadata (used by teeline-qt)
// ---------------------------------------------------------------------------

pub struct SolverInfo {
    pub name: &'static str,
    pub alias: &'static str,
    pub category: &'static str,
    pub desc: &'static str,
    pub complexity: &'static str,
    pub has_options: bool,
    pub exact: bool,
}

static SOLVER_LIST: [SolverInfo; 22] = [
    SolverInfo {
        name: "Ant Colony",
        alias: "aco",
        category: "Metaheuristic",
        desc: "Colony of ants probabilistically construct tours biased by a shared pheromone matrix and heuristic desirability (1/distance); pheromone evaporates and reinforces on strong edges over epochs.",
        complexity: "O(epochs \u{00b7} ants \u{00b7} n\u{00b2})",
        has_options: true,
        exact: false,
    },
    SolverInfo {
        name: "Bellman-Held-Karp",
        alias: "bhk",
        category: "Exact",
        desc: "Exact dynamic-programming solution. Optimal tour guaranteed.",
        complexity: "O(n\u{00b2} \u{00b7} 2\u{207f})",
        has_options: false,
        exact: true,
    },
    SolverInfo {
        name: "Branch & Bound",
        alias: "branch_bound",
        category: "Exact",
        desc: "Exact branch-and-bound with lower-bound pruning.",
        complexity: "O(n!)",
        has_options: false,
        exact: true,
    },
    SolverInfo {
        name: "Christofides",
        alias: "christofides",
        category: "Approximation",
        desc: "\u{2264}1.5\u{00d7} approximation via MST + greedy matching + Eulerian shortcut (EUC_2D only).",
        complexity: "O(n\u{00b2})",
        has_options: false,
        exact: false,
    },
    SolverInfo {
        name: "Fourier",
        alias: "fourier",
        category: "Constructive",
        desc: "Closed-curve Fourier-basis gradient descent with argsort decode.",
        complexity: "O(K\u{00b7}epochs\u{00b7}n\u{00b7}M)",
        has_options: true,
        exact: false,
    },
    SolverInfo {
        name: "Nearest Neighbor",
        alias: "nn",
        category: "Constructive",
        desc: "Greedy heuristic: always visit the nearest unvisited city.",
        complexity: "O(n\u{00b2})",
        has_options: false,
        exact: false,
    },
    SolverInfo {
        name: "Greedy Edge",
        alias: "gec",
        category: "Constructive",
        desc: "Kruskal-style construction: sorts all edges shortest-first and greedily \
               accepts each unless it creates degree 3+ or a premature sub-cycle.",
        complexity: "O(n\u{00b2} log n) time, O(n\u{00b2}) memory",
        has_options: false,
        exact: false,
    },
    SolverInfo {
        name: "Savings",
        alias: "sav",
        category: "Constructive",
        desc: "Savings construction: ranks edges by s(i,j)=d(hub,i)+d(hub,j)-d(i,j) relative to a \
               centroid-nearest hub, then greedily accepts each (Kruskal-style degree/cycle guard). \
               Inspired by Clarke-Wright but mechanically a savings-ordered greedy-edge builder.",
        complexity: "O(n\u{00b2} log n) time, O(n\u{00b2}) memory",
        has_options: false,
        exact: false,
    },
    SolverInfo {
        name: "2-opt",
        alias: "2opt",
        category: "Local Search",
        desc: "Iteratively reverses sub-tours to remove crossing edges.",
        complexity: "O(n\u{00b2}) / pass",
        has_options: false,
        exact: false,
    },
    SolverInfo {
        name: "3-opt",
        alias: "3opt",
        category: "Local Search",
        desc: "Extends 2-opt by considering triple-edge reconnections.",
        complexity: "O(n\u{00b3}) / pass",
        has_options: false,
        exact: false,
    },
    SolverInfo {
        name: "Simulated Annealing",
        alias: "sa",
        category: "Metaheuristic",
        desc: "Accepts worse moves with decreasing probability to escape local optima.",
        complexity: "O(epochs \u{00b7} n)",
        has_options: true,
        exact: false,
    },
    SolverInfo {
        name: "Genetic Algorithm",
        alias: "ga",
        category: "Metaheuristic",
        desc: "Evolves a population of tours via crossover and mutation operators.",
        complexity: "O(epochs \u{00b7} pop \u{00b7} n)",
        has_options: true,
        exact: false,
    },
    SolverInfo {
        name: "Gravitational Search",
        alias: "gsa",
        category: "Metaheuristic",
        desc: "Agents (tours) attract each other by mass (fitness); heavier agents pull lighter ones via swap-list velocity.",
        complexity: "O(epochs \u{00b7} pop\u{00b2})",
        has_options: true,
        exact: false,
    },
    SolverInfo {
        name: "Particle Swarm",
        alias: "pso",
        category: "Metaheuristic",
        desc: "Discrete PSO with velocity-capped particles guided by a global best.",
        complexity: "O(epochs \u{00b7} swarm \u{00b7} n)",
        has_options: true,
        exact: false,
    },
    SolverInfo {
        name: "Cuckoo Search",
        alias: "cs",
        category: "Metaheuristic",
        desc: "L\u{00e9}vy-flight search with probabilistic nest abandonment.",
        complexity: "O(epochs \u{00b7} nests \u{00b7} n)",
        has_options: true,
        exact: false,
    },
    SolverInfo {
        name: "Flower Pollination",
        alias: "fpa",
        category: "Metaheuristic",
        desc: "Global L\u{00e9}vy-flight toward best tour; local \u{03b5}-scaled cross-pollination.",
        complexity: "O(epochs \u{00b7} pop \u{00b7} n)",
        has_options: true,
        exact: false,
    },
    SolverInfo {
        name: "Lin-Kernighan",
        alias: "lk",
        category: "Local Search",
        desc: "Lin-Kernighan style ILS: 2-opt with candidate lists + double-bridge kicks.",
        complexity: "O(epochs \u{00b7} n\u{00b2})",
        has_options: true,
        exact: false,
    },
    SolverInfo {
        name: "Or-opt",
        alias: "or_opt",
        category: "Local Search",
        desc: "Relocates segments of 1\u{2013}3 cities to better positions (best-improvement).",
        complexity: "O(n\u{00b2}) / pass",
        has_options: false,
        exact: false,
    },
    SolverInfo {
        name: "Stochastic Hill Climb",
        alias: "stochastic_hill",
        category: "Metaheuristic",
        desc: "Random-restart hill climbing to escape local optima.",
        complexity: "O(epochs \u{00b7} n)",
        has_options: false,
        exact: false,
    },
    SolverInfo {
        name: "Tabu Search",
        alias: "tabu_search",
        category: "Metaheuristic",
        desc: "Local search with a memory structure to avoid revisiting solutions.",
        complexity: "O(epochs \u{00b7} n)",
        has_options: false,
        exact: false,
    },
    SolverInfo {
        name: "Kohonen SOM",
        alias: "som",
        category: "Constructive",
        desc: "Elastic ring of neurons wrapping around cities via Hebbian learning; topology-preserving tour extraction.",
        complexity: "O(epochs\u{00b7}N\u{00b7}n)",
        has_options: true,
        exact: false,
    },
    SolverInfo {
        name: "Random Shuffle",
        alias: "shuffle",
        category: "Utility",
        desc: "Baseline random tour. Useful as a warm-start seed for pipelines.",
        complexity: "O(n)",
        has_options: false,
        exact: false,
    },
];

pub fn list_solvers() -> &'static [SolverInfo] {
    &SOLVER_LIST
}

impl FromStr for Solvers {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aco" | "ant_colony" => Ok(Solvers::AntColony),
            "bhk" | "bellman_karp" => Ok(Solvers::BellmanKarp),
            "branch_bound" => Ok(Solvers::BranchBound),
            "christofides" | "chr" => Ok(Solvers::Christofides),
            "sav" | "savings" => Ok(Solvers::Savings),
            "cs" | "cuckoo_search" => Ok(Solvers::CuckooSearch),
            "fpa" | "flower_pollination" => Ok(Solvers::FlowerPollination),
            "fourier" => Ok(Solvers::Fourier),
            "lk" | "lin_kernighan" => Ok(Solvers::LinKernighan),
            "nn" | "nearest_neighbor" => Ok(Solvers::NearestNeighbor),
            "ga" | "genetic_algorithm" => Ok(Solvers::GeneticAlgorithm),
            "gsa" | "gravitational_search" => Ok(Solvers::GravitationalSearch),
            "gec" | "greedy_edge" => Ok(Solvers::GreedyEdge),
            "pso" | "particle_swarm" => Ok(Solvers::ParticleSwarmOptimization),
            "shuffle" | "random_shuffle" => Ok(Solvers::RandomShuffle),
            "sa" | "simulated_annealing" => Ok(Solvers::SimulatedAnnealing),
            "som" | "kohonen" | "kohonen_som" => Ok(Solvers::KohonenSom),
            "stochastic_hill" => Ok(Solvers::StochasticHill),
            "tabu" | "tabu_search" => Ok(Solvers::TabuSearch),
            "or_opt" | "or-opt" => Ok(Solvers::OrOpt),
            "3opt" | "three_opt" => Ok(Solvers::ThreeOpt),
            "2opt" | "two_opt" => Ok(Solvers::TwoOpt),
            // Presets are handled at the CLI layer (main.rs::resolve_preset), not here.
            _ => Err("unknown solver"),
        }
    }
}

// ---------------------------------------------------------------------------
// Heuristic options shared across all solver-specific option structs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct HeuristicOptions {
    pub epochs: usize,
    pub platoo_epochs: usize,
    pub n_nearest: usize,
    pub verbose: bool,
}

impl Default for HeuristicOptions {
    fn default() -> Self {
        HeuristicOptions {
            epochs: 10_000,
            platoo_epochs: 500,
            n_nearest: 3,
            verbose: false,
        }
    }
}

impl HeuristicOptions {
    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut h = HeuristicOptions::default();
        for (k, v) in table.iter() {
            match k.as_str() {
                "epochs" => {
                    h.epochs = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))?
                        as usize;
                }
                "platoo_epochs" => {
                    h.platoo_epochs = v.as_integer().ok_or_else(|| {
                        format!("config: `platoo_epochs` must be an integer, got {v}")
                    })? as usize;
                }
                "n_nearest" => {
                    h.n_nearest = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `n_nearest` must be an integer, got {v}"))?
                        as usize;
                }
                "verbose" => {
                    h.verbose = v
                        .as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` in [heuristic] — valid: epochs, platoo_epochs, n_nearest, verbose"
                    ));
                }
            }
        }
        h.validate()?;
        Ok(h)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        let mut h = HeuristicOptions::default();
        if let Some(v) = args.get_one::<String>("epochs") {
            h.epochs = v
                .parse()
                .map_err(|_| format!("--epochs: invalid integer `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("platoo_epochs") {
            h.platoo_epochs = v
                .parse()
                .map_err(|_| format!("--platoo-epochs: invalid integer `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("n_nearest") {
            h.n_nearest = v
                .parse()
                .map_err(|_| format!("--n-nearest: invalid integer `{v}`"))?;
        }
        if args.get_flag("verbose") {
            h.verbose = true;
        }
        h.validate()?;
        Ok(h)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.n_nearest == 0 {
            return Err("n_nearest must be >= 1".to_string());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Solver-specific option structs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct SAOptions {
    pub heuristic: HeuristicOptions,
    pub cooling_rate: f32,
    pub min_temperature: f32,
    pub max_temperature: f32,
}

impl Default for SAOptions {
    fn default() -> Self {
        SAOptions {
            heuristic: HeuristicOptions::default(),
            cooling_rate: 0.0001,
            min_temperature: 0.001,
            max_temperature: 1_000.0,
        }
    }
}

impl SAOptions {
    pub fn validate(&self) -> Result<(), String> {
        self.heuristic.validate()?;
        if self.cooling_rate <= 0.0 {
            return Err(format!(
                "cooling_rate must be > 0 (got {})",
                self.cooling_rate
            ));
        }
        if self.cooling_rate >= 1.0 {
            return Err(format!(
                "cooling_rate must be < 1 (got {})",
                self.cooling_rate
            ));
        }
        if self.max_temperature <= 0.0 {
            return Err(format!(
                "max_temperature must be > 0 (got {})",
                self.max_temperature
            ));
        }
        if self.min_temperature < 0.0 {
            return Err(format!(
                "min_temperature must be >= 0 (got {})",
                self.min_temperature
            ));
        }
        if self.min_temperature >= self.max_temperature {
            return Err(format!(
                "min_temperature ({}) must be < max_temperature ({})",
                self.min_temperature, self.max_temperature
            ));
        }
        Ok(())
    }

    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut sa = SAOptions::default();
        for (k, v) in table.iter() {
            match k.as_str() {
                "epochs" => {
                    sa.heuristic.epochs = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))?
                        as usize;
                }
                "platoo_epochs" => {
                    sa.heuristic.platoo_epochs = v.as_integer().ok_or_else(|| {
                        format!("config: `platoo_epochs` must be an integer, got {v}")
                    })? as usize;
                }
                "n_nearest" => {
                    sa.heuristic.n_nearest = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `n_nearest` must be an integer, got {v}"))?
                        as usize;
                }
                "verbose" => {
                    sa.heuristic.verbose = v
                        .as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                "cooling_rate" => {
                    sa.cooling_rate = parse_f32(v, "sa.cooling_rate")?;
                }
                "max_temperature" => {
                    sa.max_temperature = parse_f32(v, "sa.max_temperature")?;
                }
                "min_temperature" => {
                    sa.min_temperature = parse_f32(v, "sa.min_temperature")?;
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` in [sa] — valid: epochs, platoo_epochs, n_nearest, verbose, cooling_rate, max_temperature, min_temperature"
                    ));
                }
            }
        }
        sa.validate()?;
        Ok(sa)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        let mut sa = SAOptions {
            heuristic: HeuristicOptions::from_cli(args)?,
            ..SAOptions::default()
        };
        if let Some(v) = args.get_one::<String>("cooling_rate") {
            sa.cooling_rate = v
                .parse()
                .map_err(|_| format!("--cooling-rate: invalid float `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("min_temperature") {
            sa.min_temperature = v
                .parse()
                .map_err(|_| format!("--min-temperature: invalid float `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("max_temperature") {
            sa.max_temperature = v
                .parse()
                .map_err(|_| format!("--max-temperature: invalid float `{v}`"))?;
        }
        sa.validate()?;
        Ok(sa)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GAOptions {
    pub heuristic: HeuristicOptions,
    pub mutation_probability: f32,
    pub n_elite: usize,
}

impl Default for GAOptions {
    fn default() -> Self {
        GAOptions {
            heuristic: HeuristicOptions::default(),
            mutation_probability: 0.001,
            n_elite: 3,
        }
    }
}

impl GAOptions {
    pub fn validate(&self) -> Result<(), String> {
        self.heuristic.validate()?;
        if self.mutation_probability < 0.0 || self.mutation_probability > 1.0 {
            return Err(format!(
                "mutation_probability must be in [0, 1] (got {})",
                self.mutation_probability
            ));
        }
        Ok(())
    }

    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut ga = GAOptions::default();
        for (k, v) in table.iter() {
            match k.as_str() {
                "epochs" => {
                    ga.heuristic.epochs = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))?
                        as usize;
                }
                "platoo_epochs" => {
                    ga.heuristic.platoo_epochs = v.as_integer().ok_or_else(|| {
                        format!("config: `platoo_epochs` must be an integer, got {v}")
                    })? as usize;
                }
                "n_nearest" => {
                    ga.heuristic.n_nearest = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `n_nearest` must be an integer, got {v}"))?
                        as usize;
                }
                "verbose" => {
                    ga.heuristic.verbose = v
                        .as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                "mutation_probability" => {
                    ga.mutation_probability = parse_f32(v, "ga.mutation_probability")?;
                }
                "n_elite" => {
                    ga.n_elite = v.as_integer().ok_or_else(|| {
                        format!("config: `ga.n_elite` must be an integer, got {v}")
                    })? as usize;
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` in [ga] — valid: epochs, platoo_epochs, n_nearest, verbose, mutation_probability, n_elite"
                    ));
                }
            }
        }
        ga.validate()?;
        Ok(ga)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        let mut ga = GAOptions {
            heuristic: HeuristicOptions::from_cli(args)?,
            ..GAOptions::default()
        };
        if let Some(v) = args.get_one::<String>("mutation_probability") {
            ga.mutation_probability = v
                .parse()
                .map_err(|_| format!("--mutation-probability: invalid float `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("n_elite") {
            ga.n_elite = v
                .parse()
                .map_err(|_| format!("--n-elite: invalid integer `{v}`"))?;
        }
        ga.validate()?;
        Ok(ga)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CSOptions {
    pub heuristic: HeuristicOptions,
    pub mutation_probability: f32,
}

impl Default for CSOptions {
    fn default() -> Self {
        CSOptions {
            heuristic: HeuristicOptions::default(),
            mutation_probability: 0.001,
        }
    }
}

impl CSOptions {
    pub fn validate(&self) -> Result<(), String> {
        self.heuristic.validate()?;
        if self.mutation_probability < 0.0 || self.mutation_probability > 1.0 {
            return Err(format!(
                "mutation_probability must be in [0, 1] (got {})",
                self.mutation_probability
            ));
        }
        Ok(())
    }

    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut cs = CSOptions::default();
        for (k, v) in table.iter() {
            match k.as_str() {
                "epochs" => {
                    cs.heuristic.epochs = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))?
                        as usize;
                }
                "platoo_epochs" => {
                    cs.heuristic.platoo_epochs = v.as_integer().ok_or_else(|| {
                        format!("config: `platoo_epochs` must be an integer, got {v}")
                    })? as usize;
                }
                "n_nearest" => {
                    cs.heuristic.n_nearest = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `n_nearest` must be an integer, got {v}"))?
                        as usize;
                }
                "verbose" => {
                    cs.heuristic.verbose = v
                        .as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                "mutation_probability" => {
                    cs.mutation_probability = parse_f32(v, "cs.mutation_probability")?;
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` in [cs] — valid: epochs, platoo_epochs, n_nearest, verbose, mutation_probability"
                    ));
                }
            }
        }
        cs.validate()?;
        Ok(cs)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        let mut cs = CSOptions {
            heuristic: HeuristicOptions::from_cli(args)?,
            ..CSOptions::default()
        };
        if let Some(v) = args.get_one::<String>("mutation_probability") {
            cs.mutation_probability = v
                .parse()
                .map_err(|_| format!("--mutation-probability: invalid float `{v}`"))?;
        }
        cs.validate()?;
        Ok(cs)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FPAOptions {
    pub heuristic: HeuristicOptions,
    pub mutation_probability: f32,
}

impl Default for FPAOptions {
    fn default() -> Self {
        FPAOptions {
            heuristic: HeuristicOptions::default(),
            mutation_probability: 0.001,
        }
    }
}

impl FPAOptions {
    pub fn validate(&self) -> Result<(), String> {
        self.heuristic.validate()?;
        if self.mutation_probability < 0.0 || self.mutation_probability > 1.0 {
            return Err(format!(
                "mutation_probability must be in [0, 1] (got {})",
                self.mutation_probability
            ));
        }
        Ok(())
    }

    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut fpa = FPAOptions::default();
        for (k, v) in table.iter() {
            match k.as_str() {
                "epochs" => {
                    fpa.heuristic.epochs = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))?
                        as usize;
                }
                "platoo_epochs" => {
                    fpa.heuristic.platoo_epochs = v.as_integer().ok_or_else(|| {
                        format!("config: `platoo_epochs` must be an integer, got {v}")
                    })? as usize;
                }
                "n_nearest" => {
                    fpa.heuristic.n_nearest = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `n_nearest` must be an integer, got {v}"))?
                        as usize;
                }
                "verbose" => {
                    fpa.heuristic.verbose = v
                        .as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                "mutation_probability" => {
                    fpa.mutation_probability = parse_f32(v, "fpa.mutation_probability")?;
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` in [fpa] — valid: epochs, platoo_epochs, n_nearest, verbose, mutation_probability"
                    ));
                }
            }
        }
        fpa.validate()?;
        Ok(fpa)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        let mut fpa = FPAOptions {
            heuristic: HeuristicOptions::from_cli(args)?,
            ..FPAOptions::default()
        };
        if let Some(v) = args.get_one::<String>("mutation_probability") {
            fpa.mutation_probability = v
                .parse()
                .map_err(|_| format!("--mutation-probability: invalid float `{v}`"))?;
        }
        fpa.validate()?;
        Ok(fpa)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AcoOptions {
    pub heuristic: HeuristicOptions,
    pub alpha: f32,
    pub beta: f32,
    pub evaporation_rate: f32,
    pub num_ants: usize,
}

impl Default for AcoOptions {
    fn default() -> Self {
        AcoOptions {
            // ACO's per-epoch cost is O(ants * n^2) — roulette-wheel construction scans
            // every unvisited city each step, unlike CS/FPA/GA's O(pop * n). Inheriting
            // HeuristicOptions::default()'s epochs=10_000 would take minutes on berlin52
            // and far longer on larger instances, a real hang risk for any bare
            // AcoOptions::default() construction (see LKOptions::default() for the same
            // override pattern).
            heuristic: HeuristicOptions {
                epochs: 150,
                platoo_epochs: 20,
                n_nearest: 3,
                verbose: false,
            },
            alpha: 1.0,
            beta: 2.0,
            evaporation_rate: 0.5,
            num_ants: 25,
        }
    }
}

impl AcoOptions {
    pub fn validate(&self) -> Result<(), String> {
        // Not `self.heuristic.validate()?` — that only checks `n_nearest >= 1`, but ACO's
        // transition rule has no candidate-list restriction (see docs/algorithms/ant-colony.md)
        // and never reads `n_nearest`, so rejecting `n_nearest == 0` here would error on a
        // value that has zero effect on this solver.
        //
        // NOTE: `<`/`<=`/`>=` comparisons are all `false` for `NaN`, so a bare `if x < 0.0`
        // silently lets `x = NaN` through — and clippy's `neg_cmp_op_on_partial_ord` lint
        // (deny under this crate's `-D warnings` CI) rightly rejects the tempting fix of
        // just negating the comparison (`!(x >= 0.0)`), since a future refactor could
        // "simplify" that back to `x < 0.0` and silently reintroduce the same NaN gap. Use
        // `.is_finite()` (an explicit, non-comparison check) alongside plain positive
        // comparisons instead, mirroring how `beta`'s `RangeInclusive::contains` already
        // rejects NaN via `low <= x && x <= high`.
        if !self.alpha.is_finite() || self.alpha < 0.0 {
            return Err(format!("alpha must be >= 0 (got {})", self.alpha));
        }
        if !(0.0..=6.0).contains(&self.beta) {
            // Upper bound: with the 1e-6 minimum-distance floor used during transition-weight
            // computation, (1e6)^beta overflows f32 (~3.4e38) once beta exceeds ~6.42
            // (= log10(f32::MAX) / 6). 6.0 keeps a safety margin below that exact cutoff —
            // the previous 10.0 ceiling was above it, so in-range values like beta=8 could
            // already silently overflow eta_beta to +inf for ordinary close-together cities.
            return Err(format!("beta must be in [0, 6] (got {})", self.beta));
        }
        if !self.evaporation_rate.is_finite()
            || self.evaporation_rate <= 0.0
            || self.evaporation_rate >= 1.0
        {
            return Err(format!(
                "evaporation_rate must be in (0, 1) (got {})",
                self.evaporation_rate
            ));
        }
        if self.num_ants == 0 {
            return Err("num_ants must be >= 1".to_string());
        }
        Ok(())
    }

    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut aco = AcoOptions::default();
        for (k, v) in table.iter() {
            match k.as_str() {
                "epochs" => {
                    aco.heuristic.epochs = parse_nonneg_usize(v, "epochs")?;
                }
                "platoo_epochs" => {
                    aco.heuristic.platoo_epochs = parse_nonneg_usize(v, "platoo_epochs")?;
                }
                "n_nearest" => {
                    aco.heuristic.n_nearest = parse_nonneg_usize(v, "n_nearest")?;
                }
                "verbose" => {
                    aco.heuristic.verbose = v
                        .as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                "alpha" => {
                    aco.alpha = parse_f32(v, "aco.alpha")?;
                }
                "beta" => {
                    aco.beta = parse_f32(v, "aco.beta")?;
                }
                "evaporation_rate" => {
                    aco.evaporation_rate = parse_f32(v, "aco.evaporation_rate")?;
                }
                "num_ants" => {
                    aco.num_ants = parse_nonneg_usize(v, "aco.num_ants")?;
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` in [aco] — valid: epochs, platoo_epochs, n_nearest, verbose, alpha, beta, evaporation_rate, num_ants"
                    ));
                }
            }
        }
        aco.validate()?;
        Ok(aco)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        // Deliberately not `heuristic: HeuristicOptions::from_cli(args)?`: that helper seeds
        // from the generic `HeuristicOptions::default()` (epochs=10_000), and since `heuristic`
        // would be assigned explicitly in this struct literal, the trailing
        // `..AcoOptions::default()` spread never gets a chance to apply this struct's own
        // epochs=150 override (a functional-update spread only fills fields that aren't
        // explicitly listed) — every CLI invocation omitting `--epochs` would silently run
        // 10_000 epochs instead of the documented/intended 150. Building `heuristic` field by
        // field from `AcoOptions::default()` keeps the override whenever a flag isn't passed.
        let mut aco = AcoOptions::default();
        if let Some(v) = args.get_one::<String>("epochs") {
            aco.heuristic.epochs = v
                .parse()
                .map_err(|_| format!("--epochs: invalid integer `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("platoo_epochs") {
            aco.heuristic.platoo_epochs = v
                .parse()
                .map_err(|_| format!("--platoo-epochs: invalid integer `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("n_nearest") {
            aco.heuristic.n_nearest = v
                .parse()
                .map_err(|_| format!("--n-nearest: invalid integer `{v}`"))?;
        }
        if args.get_flag("verbose") {
            aco.heuristic.verbose = true;
        }
        if let Some(v) = args.get_one::<String>("alpha") {
            aco.alpha = v
                .parse()
                .map_err(|_| format!("--alpha: invalid float `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("beta") {
            aco.beta = v
                .parse()
                .map_err(|_| format!("--beta: invalid float `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("evaporation_rate") {
            aco.evaporation_rate = v
                .parse()
                .map_err(|_| format!("--evaporation-rate: invalid float `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("num_ants") {
            aco.num_ants = v
                .parse()
                .map_err(|_| format!("--num-ants: invalid integer `{v}`"))?;
        }
        aco.validate()?;
        Ok(aco)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LKOptions {
    pub heuristic: HeuristicOptions,
    pub max_depth: usize,
}

impl Default for LKOptions {
    fn default() -> Self {
        LKOptions {
            heuristic: HeuristicOptions {
                epochs: 100,
                platoo_epochs: 10,
                n_nearest: 5,
                verbose: false,
            },
            max_depth: 5,
        }
    }
}

impl LKOptions {
    pub fn validate(&self) -> Result<(), String> {
        self.heuristic.validate()?;
        if self.max_depth == 0 {
            return Err("max_depth must be >= 1".to_string());
        }
        Ok(())
    }

    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut lk = LKOptions::default();
        for (k, v) in table.iter() {
            match k.as_str() {
                "epochs" => {
                    lk.heuristic.epochs = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))?
                        as usize;
                }
                "platoo_epochs" => {
                    lk.heuristic.platoo_epochs = v.as_integer().ok_or_else(|| {
                        format!("config: `platoo_epochs` must be an integer, got {v}")
                    })? as usize;
                }
                "n_nearest" => {
                    lk.heuristic.n_nearest = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `n_nearest` must be an integer, got {v}"))?
                        as usize;
                }
                "verbose" => {
                    lk.heuristic.verbose = v
                        .as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                "max_depth" => {
                    lk.max_depth = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `max_depth` must be an integer, got {v}"))?
                        as usize;
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` in [lk] — valid: epochs, platoo_epochs, n_nearest, verbose, max_depth"
                    ));
                }
            }
        }
        lk.validate()?;
        Ok(lk)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        let mut lk = LKOptions {
            heuristic: HeuristicOptions::from_cli(args)?,
            ..LKOptions::default()
        };
        if let Some(v) = args.get_one::<String>("max_depth") {
            lk.max_depth = v
                .parse::<usize>()
                .map_err(|_| format!("--max-depth: invalid integer `{v}`"))?;
        }
        lk.validate()?;
        Ok(lk)
    }
}

// ---------------------------------------------------------------------------
// FourierOptions
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct FourierOptions {
    pub k_max: usize,      // max Fourier mode, default 4
    pub m: usize,          // curve sampling resolution, default 200
    pub lambda: f64,       // initial tension weight, default 0.05
    pub lambda_decay: f64, // tension decay multiplier per k_active stage, default 0.5
    pub lr: f64,           // gradient learning rate, default 0.05
    pub epochs: usize,     // gradient steps per k_active stage, default 400
}

impl Default for FourierOptions {
    fn default() -> Self {
        FourierOptions {
            k_max: 4,
            m: 200,
            lambda: 0.05,
            lambda_decay: 0.5,
            lr: 0.05,
            epochs: 400,
        }
    }
}

impl FourierOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.k_max == 0 {
            return Err("k_max must be >= 1".to_string());
        }
        if self.m < 2 {
            return Err("m must be >= 2".to_string());
        }
        if self.lambda <= 0.0 {
            return Err("lambda must be > 0".to_string());
        }
        if self.lambda_decay <= 0.0 || self.lambda_decay >= 1.0 {
            return Err("lambda_decay must be in (0, 1)".to_string());
        }
        if self.lr <= 0.0 {
            return Err("lr must be > 0".to_string());
        }
        if self.epochs == 0 {
            return Err("epochs must be >= 1".to_string());
        }
        Ok(())
    }

    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut f = FourierOptions::default();
        for (k, v) in table.iter() {
            match k.as_str() {
                "k_max" => {
                    f.k_max = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `k_max` must be an integer, got {v}"))?
                        as usize;
                }
                "m" => {
                    f.m = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `m` must be an integer, got {v}"))?
                        as usize;
                }
                "lambda" => {
                    f.lambda = v
                        .as_float()
                        .or_else(|| v.as_integer().map(|i| i as f64))
                        .ok_or_else(|| format!("config: `lambda` must be a float, got {v}"))?;
                }
                "lambda_decay" => {
                    f.lambda_decay = v
                        .as_float()
                        .or_else(|| v.as_integer().map(|i| i as f64))
                        .ok_or_else(|| {
                            format!("config: `lambda_decay` must be a float, got {v}")
                        })?;
                }
                "lr" => {
                    f.lr = v
                        .as_float()
                        .or_else(|| v.as_integer().map(|i| i as f64))
                        .ok_or_else(|| format!("config: `lr` must be a float, got {v}"))?;
                }
                "epochs" => {
                    f.epochs = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))?
                        as usize;
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` in [fourier] — valid: k_max, m, lambda, lambda_decay, lr, epochs"
                    ));
                }
            }
        }
        f.validate()?;
        Ok(f)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        let mut f = FourierOptions::default();
        if let Some(v) = args.get_one::<String>("epochs") {
            f.epochs = v
                .parse::<usize>()
                .map_err(|_| format!("--epochs: invalid integer `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("k_max") {
            f.k_max = v
                .parse::<usize>()
                .map_err(|_| format!("--k-max: invalid integer `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("m") {
            f.m = v
                .parse::<usize>()
                .map_err(|_| format!("--m: invalid integer `{v}`"))?;
        }
        f.validate()?;
        Ok(f)
    }
}

// ---------------------------------------------------------------------------
// SOMOptions
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct SOMOptions {
    pub epochs: usize,            // training iterations, default 100_000
    pub learning_rate: f64,       // η₀ — initial learning rate, default 0.8
    pub radius_fraction: f64,     // σ₀ = radius_fraction × N neurons, default 0.1
    pub neuron_multiplier: usize, // N = n_cities × neuron_multiplier, default 8
}

impl Default for SOMOptions {
    fn default() -> Self {
        SOMOptions {
            epochs: 100_000,
            learning_rate: 0.8,
            radius_fraction: 0.1,
            neuron_multiplier: 8,
        }
    }
}

impl SOMOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.epochs == 0 {
            return Err("epochs must be >= 1".to_string());
        }
        if self.learning_rate <= 0.0 || self.learning_rate > 1.0 {
            return Err("learning_rate must be in (0, 1]".to_string());
        }
        if self.radius_fraction <= 0.0 || self.radius_fraction > 1.0 {
            return Err("radius_fraction must be in (0, 1]".to_string());
        }
        if self.neuron_multiplier == 0 {
            return Err("neuron_multiplier must be >= 1".to_string());
        }
        Ok(())
    }

    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut s = SOMOptions::default();
        for (k, v) in table.iter() {
            match k.as_str() {
                "epochs" => {
                    let raw = v
                        .as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))?;
                    if raw < 1 {
                        return Err(format!("config: `epochs` must be >= 1, got {raw}"));
                    }
                    s.epochs = raw as usize;
                }
                "learning_rate" => {
                    s.learning_rate = v
                        .as_float()
                        .or_else(|| v.as_integer().map(|i| i as f64))
                        .ok_or_else(|| {
                            format!("config: `learning_rate` must be a float, got {v}")
                        })?;
                }
                "radius_fraction" => {
                    s.radius_fraction = v
                        .as_float()
                        .or_else(|| v.as_integer().map(|i| i as f64))
                        .ok_or_else(|| {
                        format!("config: `radius_fraction` must be a float, got {v}")
                    })?;
                }
                "neuron_multiplier" => {
                    let raw = v.as_integer().ok_or_else(|| {
                        format!("config: `neuron_multiplier` must be an integer, got {v}")
                    })?;
                    if raw < 1 {
                        return Err(format!(
                            "config: `neuron_multiplier` must be >= 1, got {raw}"
                        ));
                    }
                    s.neuron_multiplier = raw as usize;
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` in [som] — valid: epochs, learning_rate, radius_fraction, neuron_multiplier"
                    ));
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        let mut s = SOMOptions::default();
        if let Some(v) = args.get_one::<String>("epochs") {
            s.epochs = v
                .parse::<usize>()
                .map_err(|_| format!("--epochs: invalid integer `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("learning_rate") {
            s.learning_rate = v
                .parse::<f64>()
                .map_err(|_| format!("--learning_rate: invalid float `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("radius_fraction") {
            s.radius_fraction = v
                .parse::<f64>()
                .map_err(|_| format!("--radius_fraction: invalid float `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("neuron_multiplier") {
            s.neuron_multiplier = v
                .parse::<usize>()
                .map_err(|_| format!("--neuron_multiplier: invalid integer `{v}`"))?;
        }
        s.validate()?;
        Ok(s)
    }
}

// ---------------------------------------------------------------------------
// AppOptions — pure config shell; no runtime state
// ---------------------------------------------------------------------------

/// Pure config container. No progress channel, no initial tour — those are runtime concerns.
/// Each field is populated only for the solver that uses it.
#[derive(Clone, Debug, Default)]
pub struct AppOptions {
    pub sa: Option<SAOptions>,
    pub ga: Option<GAOptions>,
    pub cs: Option<CSOptions>,
    pub fpa: Option<FPAOptions>,
    pub lk: Option<LKOptions>,
    pub fourier: Option<FourierOptions>,
    pub som: Option<SOMOptions>,
    pub aco: Option<AcoOptions>,
    pub heuristic: Option<HeuristicOptions>,
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn parse_f32(v: &toml::Value, name: &str) -> Result<f32, String> {
    v.as_float()
        .or_else(|| v.as_integer().map(|i| i as f64))
        .ok_or_else(|| format!("config: `{name}` must be a float, got {v}"))
        .map(|f| f as f32)
}

/// Parses a non-negative TOML integer into a `usize`. `toml::Value::as_integer()` returns
/// the raw signed `i64` with no range enforcement, so a bare `as usize` cast on a negative
/// value would silently wrap into a huge positive number (e.g. `-5i64 as usize` is over
/// 1.8*10^19) instead of erroring — `usize::try_from` rejects negatives cleanly instead.
fn parse_nonneg_usize(v: &toml::Value, name: &str) -> Result<usize, String> {
    let n = v
        .as_integer()
        .ok_or_else(|| format!("config: `{name}` must be an integer, got {v}"))?;
    usize::try_from(n).map_err(|_| format!("config: `{name}` must be >= 0, got {n}"))
}

pub fn validate_tour(tour: &[usize], cities: &[KDPoint]) -> Result<(), String> {
    if tour.len() != cities.len() {
        return Err(format!(
            "tour length {} != cities length {}",
            tour.len(),
            cities.len()
        ));
    }
    let city_ids: std::collections::HashSet<usize> = cities.iter().map(|c| c.id).collect();
    let tour_ids: std::collections::HashSet<usize> = tour.iter().copied().collect();
    if tour_ids != city_ids {
        return Err("tour contains invalid or duplicate city IDs".to_string());
    }
    Ok(())
}

pub fn find_solver(name: &str) -> Result<Solvers, String> {
    name.parse::<Solvers>()
        .map_err(|_| format!("unknown solver: {name}"))
}

// ---------------------------------------------------------------------------
// Internal dispatcher — used by pipeline and tests
// ---------------------------------------------------------------------------

/// Public entry point for external crates (e.g. WASM). Runs `solver` against
/// `problem` with `opts`, no progress channel, no warm-start tour.
pub fn solve_problem(
    solver: Solvers,
    problem: &TspProblem,
    opts: &AppOptions,
) -> Result<Solution, String> {
    solve_with_context(solver, problem, opts, None, None)
}

pub fn solve_with_context(
    solver: Solvers,
    problem: &TspProblem,
    opts: &AppOptions,
    progress_tx: Option<mpsc::Sender<progress::ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Result<Solution, String> {
    let tx = progress_tx.as_ref();
    let h = opts.heuristic.as_ref().cloned().unwrap_or_default();
    h.validate()?;
    let solution = match solver {
        Solvers::AntColony => {
            let aco = opts.aco.as_ref().cloned().unwrap_or_default();
            aco.validate()?;
            ant_colony::solve(problem, &aco, tx, init_tour)
        }
        Solvers::BellmanKarp => bellman_karp::solve(problem, &h, tx, init_tour),
        Solvers::BranchBound => branch_bound::solve(problem, &h, tx, init_tour),
        Solvers::Christofides => christofides::solve(problem, &h, tx, init_tour),
        Solvers::Savings => savings::solve(problem, &h, tx, init_tour),
        Solvers::CuckooSearch => {
            let cs = opts.cs.as_ref().cloned().unwrap_or_default();
            cs.validate()?;
            cuckoo_search::solve(problem, &cs, tx, init_tour)
        }
        Solvers::FlowerPollination => {
            let fpa = opts.fpa.as_ref().cloned().unwrap_or_default();
            fpa.validate()?;
            flower_pollination::solve(problem, &fpa, tx, init_tour)
        }
        Solvers::Fourier => {
            let f = opts.fourier.as_ref().cloned().unwrap_or_default();
            f.validate()?;
            fourier::solve(problem, &f, tx, init_tour)
        }
        Solvers::LinKernighan => {
            let lk = opts.lk.as_ref().cloned().unwrap_or_default();
            lk.validate()?;
            lin_kernighan::solve(problem, &lk, tx, init_tour)
        }
        Solvers::NearestNeighbor => nearest_neighbor::solve(problem, &h, tx, init_tour),
        Solvers::GeneticAlgorithm => {
            let ga = opts.ga.as_ref().cloned().unwrap_or_default();
            ga.validate()?;
            genetic_algorithm::solve(problem, &ga, tx, init_tour)
        }
        Solvers::GravitationalSearch => gravitational_search::solve(problem, &h, tx, init_tour),
        Solvers::GreedyEdge => greedy_edge::solve(problem, &h, tx, init_tour),
        Solvers::ParticleSwarmOptimization => particle_swarm::solve(problem, &h, tx, init_tour),
        Solvers::RandomShuffle => random_shuffle::solve(problem, &h, tx, init_tour),
        Solvers::KohonenSom => {
            let s = opts.som.as_ref().cloned().unwrap_or_default();
            s.validate()?;
            som::solve(problem, &s, tx, init_tour)
        }
        Solvers::SimulatedAnnealing => {
            let sa = opts.sa.as_ref().cloned().unwrap_or_default();
            sa.validate()?;
            simulated_annealing::solve(problem, &sa, tx, init_tour)
        }
        Solvers::StochasticHill => stochastic_hill::solve(problem, &h, tx, init_tour),
        Solvers::TabuSearch => tabu_search::solve(problem, &h, tx, init_tour),
        Solvers::OrOpt => or_opt::solve(problem, &h, tx, init_tour),
        Solvers::ThreeOpt => three_opt::solve(problem, &h, tx, init_tour),
        Solvers::TwoOpt => two_opt::solve(problem, &h, tx, init_tour),
        Solvers::Unspecified => return Err("solver not specified".to_string()),
    };
    Ok(solution)
}

// ---------------------------------------------------------------------------
// TspProblem
// ---------------------------------------------------------------------------

/// The TSP problem instance: city layout + precomputed distance matrix.
/// Always created together from a TSPLIB file and passed as a unit.
#[derive(Clone)]
pub struct TspProblem {
    pub cities: Vec<KDPoint>,
    pub distances: DistanceMatrix,
}

impl TspProblem {
    pub fn new(cities: Vec<KDPoint>, distances: DistanceMatrix) -> Self {
        TspProblem { cities, distances }
    }
}

// ---------------------------------------------------------------------------
// Solution
// ---------------------------------------------------------------------------

pub type CityTable = HashMap<usize, KDPoint>;

pub fn city_table_from_vec(cities: &[kdtree::KDPoint]) -> CityTable {
    cities.iter().map(|c| (c.id, *c)).collect()
}

pub struct Solution {
    pub total: f32,
    route: Vec<usize>,
    cities: Vec<KDPoint>,
    cities_idx: HashMap<usize, usize>,
}

impl Solution {
    pub fn new(route: &[usize], problem: &TspProblem) -> Self {
        let cities = &problem.cities;
        let distances = &problem.distances;
        let cities_idx = cities.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

        Solution {
            total: distances.tour_length(route),
            route: route.to_vec(),
            cities: cities.to_vec(),
            cities_idx,
        }
    }

    /// Convenience constructor for solver functions that already hold separate `cities`
    /// and `distances` slices and do not need to construct a full [`TspProblem`].
    pub(crate) fn from_parts(
        route: &[usize],
        cities: &[kdtree::KDPoint],
        distances: &DistanceMatrix,
    ) -> Self {
        let cities_idx = cities.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

        Solution {
            total: distances.tour_length(route),
            route: route.to_vec(),
            cities: cities.to_vec(),
            cities_idx,
        }
    }

    pub fn len(&self) -> usize {
        self.route.len()
    }

    pub fn is_empty(&self) -> bool {
        self.route.is_empty()
    }

    pub fn route(&self) -> &[usize] {
        self.route[..].as_ref()
    }

    pub fn cities(&self) -> &[kdtree::KDPoint] {
        &self.cities[..]
    }

    pub fn get_by_city_id(&self, city_id: usize) -> Option<&KDPoint> {
        if let Some(vec_pos) = self.cities_idx.get(&city_id) {
            self.cities.get(*vec_pos)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct NearestResult {
    pub target: KDPoint,
    pub n: usize,
    results: Vec<NearestResultItem>,
}

impl NearestResult {
    /// Creates a new k-NN accumulator for `n` nearest neighbours of `point`.
    ///
    /// **Id-collision footgun**: `add()` skips any candidate whose `id` matches
    /// `point.id` (the self-exclusion guard). If the query point's `id` collides
    /// with a tree point's `id` from a different id space, that point is silently
    /// excluded. Callers querying a different id space should use a sentinel id
    /// like `usize::MAX` — see `src/tsp/fourier.rs` for the workaround.
    pub fn new(point: KDPoint, n: usize) -> Self {
        NearestResult {
            target: point,
            n,
            results: Vec::with_capacity(n),
        }
    }

    fn add(&mut self, pt: KDPoint, new_distance: f32) {
        if self.n == 0 || pt.id == self.target.id {
            return;
        }

        // Use search_radius so we always fill the buffer before applying the
        // farthest-distance gate. Without this, farthest_distance() returns the
        // distance of the current last item (not INFINITY) and subsequent farther
        // candidates are rejected even when the buffer is not yet full.
        if new_distance < self.search_radius() {
            // Binary-search insertion: O(log k) to find position + O(k) memmove,
            // replacing the previous pop + push + full sort_by which was O(k log k).
            let pos = self.results.partition_point(|r| r.distance <= new_distance);
            self.results
                .insert(pos, NearestResultItem::new(pt, new_distance));
            self.results.truncate(self.n);
        }
    }

    /// Returns the sorted k-NN buffer as a slice — no allocation.
    pub fn nearest(&self) -> &[NearestResultItem] {
        &self.results
    }

    /// The closest point found so far, or `None` if the buffer is empty.
    pub fn closest_point(&self) -> Option<KDPoint> {
        self.results.first().map(|r| r.point)
    }

    /// Distance to the closest point, or `INFINITY` if the buffer is empty.
    pub fn closest_distance(&self) -> f32 {
        self.results
            .first()
            .map(|r| r.distance)
            .unwrap_or(f32::INFINITY)
    }

    pub fn farthest_distance(&self) -> f32 {
        self.results.last().map(|x| x.distance).unwrap_or(f32::MAX)
    }

    /// Pruning radius for k-NN search: INFINITY until the result buffer holds n
    /// items (so we never prune while the buffer is still filling), then the
    /// distance to the k-th (farthest) candidate.
    pub fn search_radius(&self) -> f32 {
        if self.results.len() < self.n {
            f32::INFINITY
        } else {
            self.farthest_distance()
        }
    }
}

#[derive(Debug, Clone)]
pub struct NearestResultItem {
    pub distance: f32,
    pub point: KDPoint,
}

impl NearestResultItem {
    pub fn new(point: KDPoint, distance: f32) -> Self {
        NearestResultItem { point, distance }
    }
}

impl PartialOrd for NearestResultItem {
    fn partial_cmp(&self, other: &NearestResultItem) -> Option<Ordering> {
        self.distance.partial_cmp(&other.distance)
    }
}

impl PartialEq for NearestResultItem {
    fn eq(&self, other: &NearestResultItem) -> bool {
        self.distance == other.distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::assert_approx;

    #[test]
    fn test_heuristic_options_has_expected_fields() {
        let h = HeuristicOptions {
            epochs: 100,
            platoo_epochs: 10,
            n_nearest: 3,
            verbose: false,
        };
        assert_eq!(h.epochs, 100);
        assert_eq!(h.platoo_epochs, 10);
    }

    #[test]
    fn test_sa_options_embeds_heuristic() {
        let sa = SAOptions::default();
        assert!(sa.heuristic.epochs > 0);
        assert!(sa.cooling_rate > 0.0);
    }

    #[test]
    fn test_app_options_has_no_flat_epoch_fields() {
        let a = AppOptions {
            sa: None,
            ga: None,
            cs: None,
            fpa: None,
            lk: None,
            fourier: None,
            som: None,
            aco: None,
            heuristic: None,
        };
        drop(a);
    }

    #[test]
    fn test_heuristic_options_from_toml() {
        let t: toml::Table =
            toml::from_str("epochs=5000\nplatoo_epochs=200\nn_nearest=5\nverbose=true").unwrap();
        let h = HeuristicOptions::from_toml(&t).unwrap();
        assert_eq!(h.epochs, 5000);
        assert_eq!(h.n_nearest, 5);
        assert!(h.verbose);
    }

    #[test]
    fn test_heuristic_validate_rejects_n_nearest_zero() {
        let h = HeuristicOptions {
            epochs: 100,
            platoo_epochs: 50,
            n_nearest: 0,
            verbose: false,
        };
        let err = h.validate().unwrap_err();
        assert!(
            err.contains("n_nearest"),
            "error should name the field: {err}"
        );
    }

    #[test]
    fn test_heuristic_validate_accepts_epochs_zero_and_platoo_zero() {
        // epochs=0 is "run forever"; platoo_epochs=0 disables plateau restarts — both valid.
        let h = HeuristicOptions {
            epochs: 0,
            platoo_epochs: 0,
            n_nearest: 1,
            verbose: false,
        };
        assert!(h.validate().is_ok());
    }

    #[test]
    fn test_heuristic_from_cli_errors_on_bad_integer() {
        use clap::{Arg, ArgAction, Command};
        let cmd = Command::new("t")
            .arg(Arg::new("epochs").long("epochs").action(ArgAction::Set))
            .arg(
                Arg::new("platoo_epochs")
                    .long("platoo_epochs")
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("n_nearest")
                    .long("n_nearest")
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("verbose")
                    .long("verbose")
                    .action(ArgAction::SetTrue),
            );
        let args = cmd.get_matches_from(["t", "--epochs", "bad"]);
        let result = HeuristicOptions::from_cli(&args);
        assert!(result.is_err(), "expected Err for --epochs bad");
    }

    fn sa_test_cmd() -> clap::Command {
        use clap::{Arg, ArgAction, Command};
        Command::new("t")
            .arg(Arg::new("epochs").long("epochs").action(ArgAction::Set))
            .arg(
                Arg::new("platoo_epochs")
                    .long("platoo_epochs")
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("n_nearest")
                    .long("n_nearest")
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("verbose")
                    .long("verbose")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("cooling_rate")
                    .long("cooling_rate")
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("min_temperature")
                    .long("min_temperature")
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("max_temperature")
                    .long("max_temperature")
                    .action(ArgAction::Set),
            )
    }

    #[test]
    fn test_sa_from_cli_errors_on_bad_float() {
        let args = sa_test_cmd().get_matches_from(["t", "--cooling_rate", "xyz"]);
        assert!(
            SAOptions::from_cli(&args).is_err(),
            "expected Err for --cooling_rate xyz"
        );
    }

    #[test]
    fn test_sa_from_cli_errors_on_out_of_range_cooling_rate() {
        let args = sa_test_cmd().get_matches_from(["t", "--cooling_rate", "5.0"]);
        let result = SAOptions::from_cli(&args);
        assert!(
            result.is_err(),
            "expected Err for cooling_rate=5.0 (out of (0,1))"
        );
    }

    #[test]
    fn test_sa_options_from_toml_parses_all_fields() {
        let t: toml::Table = toml::from_str(
            "epochs=5000\ncooling_rate=0.0005\nmax_temperature=200.0\nmin_temperature=0.001",
        )
        .unwrap();
        let sa = SAOptions::from_toml(&t).unwrap();
        assert_eq!(sa.heuristic.epochs, 5000);
        assert!((sa.cooling_rate - 0.0005).abs() < 1e-6);
    }

    #[test]
    fn test_sa_options_from_toml_unknown_key_errors() {
        let t: toml::Table = toml::from_str("bogus=1.0").unwrap();
        assert!(SAOptions::from_toml(&t).unwrap_err().contains("bogus"));
    }

    #[test]
    fn test_ga_options_from_toml_parses_fields() {
        let t: toml::Table =
            toml::from_str("mutation_probability=0.05\nn_elite=5\nepochs=2000").unwrap();
        let ga = GAOptions::from_toml(&t).unwrap();
        assert!((ga.mutation_probability - 0.05).abs() < 1e-7);
        assert_eq!(ga.heuristic.epochs, 2000);
    }

    #[test]
    fn test_validate_tour_accepts_valid_tour() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]]);
        let tour: Vec<usize> = cities.iter().map(|c| c.id).collect();
        assert!(validate_tour(&tour, &cities).is_ok());
    }

    #[test]
    fn test_validate_tour_rejects_wrong_length() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]]);
        let short_tour = vec![cities[0].id, cities[1].id];
        assert!(validate_tour(&short_tour, &cities).is_err());
    }

    #[test]
    fn test_validate_tour_rejects_invalid_ids() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]]);
        let bad_tour = vec![cities[0].id, cities[0].id, cities[1].id]; // duplicate
        assert!(validate_tour(&bad_tour, &cities).is_err());
    }

    #[test]
    fn test_auto_expand_nn_for_deterministic_local_search() {
        assert!(Solvers::TwoOpt.auto_expand_with_nn());
        assert!(Solvers::ThreeOpt.auto_expand_with_nn());
        assert!(Solvers::TabuSearch.auto_expand_with_nn());
        assert!(Solvers::OrOpt.auto_expand_with_nn());
    }

    #[test]
    fn test_auto_expand_nn_false_for_stochastic_and_constructors() {
        assert!(!Solvers::SimulatedAnnealing.auto_expand_with_nn());
        assert!(!Solvers::StochasticHill.auto_expand_with_nn());
        assert!(!Solvers::GeneticAlgorithm.auto_expand_with_nn());
        assert!(!Solvers::ParticleSwarmOptimization.auto_expand_with_nn());
        assert!(!Solvers::CuckooSearch.auto_expand_with_nn());
        assert!(!Solvers::FlowerPollination.auto_expand_with_nn());
        assert!(!Solvers::NearestNeighbor.auto_expand_with_nn());
        assert!(!Solvers::BellmanKarp.auto_expand_with_nn());
        assert!(!Solvers::Unspecified.auto_expand_with_nn());
    }

    #[test]
    fn test_branch_bound_auto_expands_with_nn() {
        // B&B needs a good initial upper bound to prune early; seeding from NN
        // before backtracking cuts proof-of-optimality time by ~10×.
        assert!(Solvers::BranchBound.auto_expand_with_nn());
    }

    #[test]
    fn test_auto_expand_shuffle_for_stochastic_solvers() {
        assert!(Solvers::SimulatedAnnealing.auto_expand_with_shuffle());
        assert!(Solvers::StochasticHill.auto_expand_with_shuffle());
        assert!(Solvers::GeneticAlgorithm.auto_expand_with_shuffle());
        assert!(Solvers::ParticleSwarmOptimization.auto_expand_with_shuffle());
        assert!(Solvers::CuckooSearch.auto_expand_with_shuffle());
        assert!(Solvers::FlowerPollination.auto_expand_with_shuffle());
        assert!(Solvers::Fourier.auto_expand_with_shuffle());
        assert!(Solvers::GravitationalSearch.auto_expand_with_shuffle());
        assert!(Solvers::AntColony.auto_expand_with_shuffle());
    }

    #[test]
    fn test_auto_expand_shuffle_false_for_deterministic_and_constructors() {
        assert!(!Solvers::TwoOpt.auto_expand_with_shuffle());
        assert!(!Solvers::ThreeOpt.auto_expand_with_shuffle());
        assert!(!Solvers::TabuSearch.auto_expand_with_shuffle());
        assert!(!Solvers::NearestNeighbor.auto_expand_with_shuffle());
        assert!(!Solvers::BellmanKarp.auto_expand_with_shuffle());
        assert!(!Solvers::RandomShuffle.auto_expand_with_shuffle());
    }

    #[test]
    fn test_solution_total_for_tsp_5_1() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);

        let route = vec![
            cities[0].id,
            cities[1].id,
            cities[2].id,
            cities[3].id,
            cities[4].id,
        ];

        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);
        let sol = Solution::new(&route, &problem);
        assert_approx(4.0, sol.total);
    }

    // Registration-consistency guards: a solver added to only some of
    // `variants()`/`all_meta()`/`SOLVER_LIST` (missing `FromStr`) would otherwise
    // compile fine and only fail at runtime when a user picks that exact alias.
    #[test]
    fn solver_list_aliases_all_round_trip_through_fromstr() {
        use std::str::FromStr;
        for info in list_solvers() {
            assert!(
                Solvers::from_str(info.alias).is_ok(),
                "SOLVER_LIST alias {:?} must resolve via FromStr",
                info.alias
            );
        }
    }

    #[test]
    fn all_meta_names_and_aliases_round_trip_through_fromstr() {
        use std::str::FromStr;
        for meta in Solvers::all_meta() {
            assert!(
                Solvers::from_str(meta.name).is_ok(),
                "all_meta name {:?} must resolve via FromStr",
                meta.name
            );
            if let Some(alias) = meta.alias {
                assert!(
                    Solvers::from_str(alias).is_ok(),
                    "all_meta alias {:?} must resolve via FromStr",
                    alias
                );
            }
        }
    }

    #[test]
    fn variants_all_round_trip_through_fromstr_except_presets() {
        use std::str::FromStr;
        let presets = ["classic", "fast", "thorough"];
        for v in Solvers::variants() {
            if presets.contains(&v) {
                continue;
            }
            assert!(
                Solvers::from_str(v).is_ok(),
                "variants() entry {v:?} must resolve via FromStr (or be added to the presets exclusion list)"
            );
        }
    }

    #[test]
    fn lk_solver_can_be_parsed_from_string() {
        use std::str::FromStr;
        assert!(Solvers::from_str("lk").is_ok());
        assert!(Solvers::from_str("lin_kernighan").is_ok());
    }

    #[test]
    fn lk_options_default_is_valid() {
        LKOptions::default()
            .validate()
            .expect("default LKOptions must be valid");
    }

    #[test]
    fn lk_options_validate_rejects_zero_n_nearest() {
        let opts = LKOptions {
            heuristic: HeuristicOptions {
                n_nearest: 0,
                ..HeuristicOptions::default()
            },
            max_depth: 5,
        };
        assert!(opts.validate().is_err(), "n_nearest=0 must be rejected");
    }

    #[test]
    fn lk_options_validate_rejects_zero_max_depth() {
        let opts = LKOptions {
            max_depth: 0,
            ..LKOptions::default()
        };
        assert!(opts.validate().is_err(), "max_depth=0 must be rejected");
    }

    #[test]
    fn lk_options_from_toml_parses_all_fields() {
        let t: toml::Table = toml::from_str("epochs=50\nn_nearest=7\nmax_depth=3").unwrap();
        let opts = LKOptions::from_toml(&t).unwrap();
        assert_eq!(opts.heuristic.epochs, 50);
        assert_eq!(opts.heuristic.n_nearest, 7);
        assert_eq!(opts.max_depth, 3);
    }

    #[test]
    fn lk_options_from_cli_parses_max_depth() {
        use clap::{Arg, ArgAction, Command};
        let cmd = Command::new("t")
            .arg(Arg::new("epochs").long("epochs").action(ArgAction::Set))
            .arg(
                Arg::new("platoo_epochs")
                    .long("platoo_epochs")
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("n_nearest")
                    .long("n_nearest")
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("verbose")
                    .long("verbose")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("max_depth")
                    .long("max-depth")
                    .action(ArgAction::Set),
            ); // hyphen matches production CLI
        let args = cmd.get_matches_from(["t", "--max-depth", "3"]); // hyphen matches production CLI
        let opts = LKOptions::from_cli(&args).unwrap();
        assert_eq!(opts.max_depth, 3);
    }

    #[test]
    fn aco_options_default_is_valid() {
        AcoOptions::default()
            .validate()
            .expect("default AcoOptions must be valid");
    }

    #[test]
    fn aco_options_validate_rejects_zero_evaporation_rate() {
        let opts = AcoOptions {
            evaporation_rate: 0.0,
            ..AcoOptions::default()
        };
        assert!(
            opts.validate().is_err(),
            "evaporation_rate=0.0 must be rejected"
        );
    }

    #[test]
    fn aco_options_validate_rejects_evaporation_rate_at_or_above_one() {
        let opts = AcoOptions {
            evaporation_rate: 1.0,
            ..AcoOptions::default()
        };
        assert!(
            opts.validate().is_err(),
            "evaporation_rate=1.0 must be rejected"
        );
    }

    #[test]
    fn aco_options_validate_rejects_zero_num_ants() {
        let opts = AcoOptions {
            num_ants: 0,
            ..AcoOptions::default()
        };
        assert!(opts.validate().is_err(), "num_ants=0 must be rejected");
    }

    #[test]
    fn aco_options_validate_rejects_negative_alpha() {
        let opts = AcoOptions {
            alpha: -0.1,
            ..AcoOptions::default()
        };
        assert!(opts.validate().is_err(), "negative alpha must be rejected");
    }

    #[test]
    fn aco_options_validate_rejects_beta_above_upper_bound() {
        let opts = AcoOptions {
            beta: 6.1,
            ..AcoOptions::default()
        };
        assert!(opts.validate().is_err(), "beta above 6.0 must be rejected");
    }

    #[test]
    fn aco_options_from_toml_parses_all_fields() {
        let t: toml::Table = toml::from_str(
            "epochs=50\nn_nearest=7\nalpha=1.5\nbeta=3.0\nevaporation_rate=0.4\nnum_ants=10",
        )
        .unwrap();
        let opts = AcoOptions::from_toml(&t).unwrap();
        assert_eq!(opts.heuristic.epochs, 50);
        assert_eq!(opts.heuristic.n_nearest, 7);
        assert!((opts.alpha - 1.5).abs() < 1e-6);
        assert!((opts.beta - 3.0).abs() < 1e-6);
        assert!((opts.evaporation_rate - 0.4).abs() < 1e-6);
        assert_eq!(opts.num_ants, 10);
    }

    #[test]
    fn aco_options_from_toml_rejects_unknown_field() {
        let t: toml::Table = toml::from_str("not_a_real_field=1").unwrap();
        let err = AcoOptions::from_toml(&t).unwrap_err();
        assert!(err.contains("unknown field"), "got: {err}");
    }

    #[test]
    fn aco_options_from_cli_parses_own_flags() {
        use clap::{Arg, ArgAction, Command};
        let cmd = Command::new("t")
            .arg(Arg::new("epochs").long("epochs").action(ArgAction::Set))
            .arg(
                Arg::new("platoo_epochs")
                    .long("platoo_epochs")
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("n_nearest")
                    .long("n_nearest")
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("verbose")
                    .long("verbose")
                    .action(ArgAction::SetTrue),
            )
            .arg(Arg::new("alpha").long("alpha").action(ArgAction::Set))
            .arg(Arg::new("beta").long("beta").action(ArgAction::Set))
            .arg(
                Arg::new("evaporation_rate")
                    .long("evaporation-rate")
                    .action(ArgAction::Set),
            )
            .arg(Arg::new("num_ants").long("num-ants").action(ArgAction::Set));
        let args = cmd.get_matches_from([
            "t",
            "--alpha",
            "2.0",
            "--beta",
            "4.0",
            "--evaporation-rate",
            "0.3",
            "--num-ants",
            "12",
        ]);
        let opts = AcoOptions::from_cli(&args).unwrap();
        assert!((opts.alpha - 2.0).abs() < 1e-6);
        assert!((opts.beta - 4.0).abs() < 1e-6);
        assert!((opts.evaporation_rate - 0.3).abs() < 1e-6);
        assert_eq!(opts.num_ants, 12);
    }

    fn fourier_cli_command() -> clap::Command {
        use clap::{Arg, ArgAction, Command};
        Command::new("t")
            .arg(Arg::new("epochs").long("epochs").action(ArgAction::Set))
            .arg(Arg::new("k_max").long("k-max").action(ArgAction::Set)) // hyphen matches production CLI
            .arg(Arg::new("m").long("m").action(ArgAction::Set))
    }

    #[test]
    fn test_fourier_from_cli_parses_k_max_and_m() {
        let cmd = fourier_cli_command();
        let args = cmd.get_matches_from(["t", "--k-max", "32", "--m", "1120"]);
        let opts = FourierOptions::from_cli(&args).unwrap();
        assert_eq!(opts.k_max, 32);
        assert_eq!(opts.m, 1120);
    }

    #[test]
    fn test_fourier_from_cli_errors_on_bad_integer() {
        let cmd = fourier_cli_command();
        let args = cmd.get_matches_from(["t", "--k-max", "abc"]);
        let err = FourierOptions::from_cli(&args).unwrap_err();
        assert!(err.contains("--k-max"), "got: {err}");
    }

    #[test]
    fn test_fourier_from_cli_epochs_zero_errors() {
        // Unlike HeuristicOptions.epochs (0 = run forever), FourierOptions.epochs is
        // gradient steps per k_active stage and validate() rejects 0.
        let cmd = fourier_cli_command();
        let args = cmd.get_matches_from(["t", "--epochs", "0"]);
        let err = FourierOptions::from_cli(&args).unwrap_err();
        assert!(err.contains("epochs"), "got: {err}");
    }
}

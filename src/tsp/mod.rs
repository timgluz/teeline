pub mod bellman_karp;
pub mod branch_bound;
pub mod comparison;
pub use comparison::{compare_tours, tour_cost, ComparisonStats};
pub mod convert;
pub mod cuckoo_search;
pub mod distance_matrix;
pub mod flower_pollination;
pub mod lin_kernighan;
pub mod genetic_algorithm;
pub mod kdtree;
pub mod nearest_neighbor;
pub mod opt_tour;
pub mod or_opt;
pub mod particle_swarm;
pub mod pipeline;
pub mod probability;
pub mod progress;
pub mod random_shuffle;
pub mod route;
pub mod simulated_annealing;
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
    BellmanKarp,
    BranchBound,
    CuckooSearch,
    FlowerPollination,
    LinKernighan,
    NearestNeighbor,
    GeneticAlgorithm,
    OrOpt,
    ParticleSwarmOptimization,
    RandomShuffle,
    SimulatedAnnealing,
    StochasticHill,
    TabuSearch,
    ThreeOpt,
    TwoOpt,
    Unspecified,
}

impl Solvers {
    pub fn variants() -> Vec<&'static str> {
        vec![
            "bellman_karp",
            "bhk",
            "branch_bound",
            "cs",
            "cuckoo_search",
            "fpa",
            "flower_pollination",
            "lk",
            "lin_kernighan",
            "nearest_neighbor",
            "nn",
            "genetic_algorithm",
            "ga",
            "particle_swarm",
            "pso",
            "random_shuffle",
            "shuffle",
            "simulated_annealing",
            "sa",
            "stochastic_hill",
            "tabu_search",
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
                | Solvers::ParticleSwarmOptimization
                | Solvers::CuckooSearch
                | Solvers::FlowerPollination
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
            SolverMeta { name: "bellman_karp", alias: Some("bhk"), kind: SolverKind::Exact },
            SolverMeta { name: "branch_bound", alias: None, kind: SolverKind::Exact },
            SolverMeta { name: "nearest_neighbor", alias: Some("nn"), kind: SolverKind::Heuristic },
            SolverMeta { name: "two_opt", alias: Some("2opt"), kind: SolverKind::Heuristic },
            SolverMeta { name: "three_opt", alias: Some("3opt"), kind: SolverKind::Heuristic },
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
            SolverMeta { name: "tabu_search", alias: Some("tabu"), kind: SolverKind::Heuristic },
            SolverMeta {
                name: "particle_swarm",
                alias: Some("pso"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta { name: "cuckoo_search", alias: Some("cs"), kind: SolverKind::Heuristic },
            SolverMeta {
                name: "flower_pollination",
                alias: Some("fpa"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta {
                name: "lin_kernighan",
                alias: Some("lk"),
                kind: SolverKind::Heuristic,
            },
            SolverMeta { name: "or_opt", alias: Some("or-opt"), kind: SolverKind::Heuristic },
            SolverMeta { name: "stochastic_hill", alias: None, kind: SolverKind::Heuristic },
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
    pub name:        &'static str,
    pub alias:       &'static str,
    pub category:    &'static str,
    pub desc:        &'static str,
    pub complexity:  &'static str,
    pub has_options: bool,
    pub exact:       bool,
}

static SOLVER_LIST: [SolverInfo; 15] = [
    SolverInfo { name: "Bellman-Held-Karp",     alias: "bhk",             category: "Exact",
                 desc: "Exact dynamic-programming solution. Optimal tour guaranteed.",
                 complexity: "O(n\u{00b2} \u{00b7} 2\u{207f})", has_options: false, exact: true },
    SolverInfo { name: "Branch & Bound",        alias: "branch_bound",    category: "Exact",
                 desc: "Exact branch-and-bound with lower-bound pruning.",
                 complexity: "O(n!)", has_options: false, exact: true },
    SolverInfo { name: "Nearest Neighbor",      alias: "nn",              category: "Constructive",
                 desc: "Greedy heuristic: always visit the nearest unvisited city.",
                 complexity: "O(n\u{00b2})", has_options: false, exact: false },
    SolverInfo { name: "2-opt",                 alias: "2opt",            category: "Local Search",
                 desc: "Iteratively reverses sub-tours to remove crossing edges.",
                 complexity: "O(n\u{00b2}) / pass", has_options: false, exact: false },
    SolverInfo { name: "3-opt",                 alias: "3opt",            category: "Local Search",
                 desc: "Extends 2-opt by considering triple-edge reconnections.",
                 complexity: "O(n\u{00b3}) / pass", has_options: false, exact: false },
    SolverInfo { name: "Simulated Annealing",   alias: "sa",              category: "Metaheuristic",
                 desc: "Accepts worse moves with decreasing probability to escape local optima.",
                 complexity: "O(epochs \u{00b7} n)", has_options: true, exact: false },
    SolverInfo { name: "Genetic Algorithm",     alias: "ga",              category: "Metaheuristic",
                 desc: "Evolves a population of tours via crossover and mutation operators.",
                 complexity: "O(epochs \u{00b7} pop \u{00b7} n)", has_options: true, exact: false },
    SolverInfo { name: "Particle Swarm",        alias: "pso",             category: "Metaheuristic",
                 desc: "Discrete PSO with velocity-capped particles guided by a global best.",
                 complexity: "O(epochs \u{00b7} swarm \u{00b7} n)", has_options: true, exact: false },
    SolverInfo { name: "Cuckoo Search",         alias: "cs",              category: "Metaheuristic",
                 desc: "L\u{00e9}vy-flight search with probabilistic nest abandonment.",
                 complexity: "O(epochs \u{00b7} nests \u{00b7} n)", has_options: true, exact: false },
    SolverInfo { name: "Flower Pollination",    alias: "fpa",             category: "Metaheuristic",
                 desc: "Global L\u{00e9}vy-flight toward best tour; local \u{03b5}-scaled cross-pollination.",
                 complexity: "O(epochs \u{00b7} pop \u{00b7} n)", has_options: true, exact: false },
    SolverInfo { name: "Lin-Kernighan",         alias: "lk",              category: "Local Search",
                 desc: "Lin-Kernighan style ILS: 2-opt with candidate lists + double-bridge kicks.",
                 complexity: "O(epochs \u{00b7} n\u{00b2})", has_options: true, exact: false },
    SolverInfo { name: "Or-opt",                alias: "or_opt",          category: "Local Search",
                 desc: "Relocates segments of 1\u{2013}3 cities to better positions (best-improvement).",
                 complexity: "O(n\u{00b2}) / pass", has_options: false, exact: false },
    SolverInfo { name: "Stochastic Hill Climb", alias: "stochastic_hill", category: "Metaheuristic",
                 desc: "Random-restart hill climbing to escape local optima.",
                 complexity: "O(epochs \u{00b7} n)", has_options: false, exact: false },
    SolverInfo { name: "Tabu Search",           alias: "tabu_search",     category: "Metaheuristic",
                 desc: "Local search with a memory structure to avoid revisiting solutions.",
                 complexity: "O(epochs \u{00b7} n)", has_options: false, exact: false },
    SolverInfo { name: "Random Shuffle",        alias: "shuffle",         category: "Utility",
                 desc: "Baseline random tour. Useful as a warm-start seed for pipelines.",
                 complexity: "O(n)", has_options: false, exact: false },
];

pub fn list_solvers() -> &'static [SolverInfo] {
    &SOLVER_LIST
}

impl FromStr for Solvers {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bhk" | "bellman_karp" => Ok(Solvers::BellmanKarp),
            "branch_bound" => Ok(Solvers::BranchBound),
            "cs" | "cuckoo_search" => Ok(Solvers::CuckooSearch),
            "fpa" | "flower_pollination" => Ok(Solvers::FlowerPollination),
            "lk" | "lin_kernighan" => Ok(Solvers::LinKernighan),
            "nn" | "nearest_neighbor" => Ok(Solvers::NearestNeighbor),
            "ga" | "genetic_algorithm" => Ok(Solvers::GeneticAlgorithm),
            "pso" | "particle_swarm" => Ok(Solvers::ParticleSwarmOptimization),
            "shuffle" | "random_shuffle" => Ok(Solvers::RandomShuffle),
            "sa" | "simulated_annealing" => Ok(Solvers::SimulatedAnnealing),
            "stochastic_hill" => Ok(Solvers::StochasticHill),
            "tabu_search" => Ok(Solvers::TabuSearch),
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
    let solution = match solver {
        Solvers::BellmanKarp => bellman_karp::solve(problem, &h, tx, init_tour),
        Solvers::BranchBound => branch_bound::solve(problem, &h, tx, init_tour),
        Solvers::CuckooSearch => {
            let cs = opts.cs.as_ref().cloned().unwrap_or_default();
            cuckoo_search::solve(problem, &cs, tx, init_tour)
        }
        Solvers::FlowerPollination => {
            let fpa = opts.fpa.as_ref().cloned().unwrap_or_default();
            flower_pollination::solve(problem, &fpa, tx, init_tour)
        }
        Solvers::LinKernighan => {
            let lk = opts.lk.as_ref().cloned().unwrap_or_default();
            lin_kernighan::solve(problem, &lk, tx, init_tour)
        }
        Solvers::NearestNeighbor => nearest_neighbor::solve(problem, &h, tx, init_tour),
        Solvers::GeneticAlgorithm => {
            let ga = opts.ga.as_ref().cloned().unwrap_or_default();
            genetic_algorithm::solve(problem, &ga, tx, init_tour)
        }
        Solvers::ParticleSwarmOptimization => particle_swarm::solve(problem, &h, tx, init_tour),
        Solvers::RandomShuffle => random_shuffle::solve(problem, &h, tx, init_tour),
        Solvers::SimulatedAnnealing => {
            let sa = opts.sa.as_ref().cloned().unwrap_or_default();
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
    pub point: KDPoint,
    pub distance: f32,
    pub n: usize,
    results: Vec<NearestResultItem>,
}

impl NearestResult {
    pub fn new(point: KDPoint, distance: f32, n: usize) -> Self {
        let results = Vec::with_capacity(n);

        NearestResult {
            target: point,
            point,
            distance,
            n,
            results,
        }
    }

    fn add(&mut self, pt: KDPoint, new_distance: f32) {
        if self.n == 0 || pt.id == self.target.id {
            return;
        }

        if new_distance < self.closest_distance() {
            self.distance = new_distance;
            self.point = pt;
        }

        // Use search_radius so we always fill the buffer before applying the
        // farthest-distance gate. Without this, farthest_distance() returns the
        // distance of the current last item (not INFINITY) and subsequent farther
        // candidates are rejected even when the buffer is not yet full.
        if new_distance < self.search_radius() {
            if self.results.len() >= self.n {
                self.results.pop();
            }

            let new_result = NearestResultItem::new(pt, new_distance);
            self.results.push(new_result);
            self.results.sort_by(|a, b| a.partial_cmp(b).unwrap());
        }
    }

    pub fn nearest(&self) -> Vec<&NearestResultItem> {
        self.results.iter().collect::<Vec<&NearestResultItem>>()
    }

    pub fn closest_distance(&self) -> f32 {
        self.distance
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

    #[test]
    fn lk_solver_can_be_parsed_from_string() {
        use std::str::FromStr;
        assert!(Solvers::from_str("lk").is_ok());
        assert!(Solvers::from_str("lin_kernighan").is_ok());
    }

    #[test]
    fn lk_options_default_is_valid() {
        LKOptions::default().validate().expect("default LKOptions must be valid");
    }

    #[test]
    fn lk_options_validate_rejects_zero_n_nearest() {
        let opts = LKOptions {
            heuristic: HeuristicOptions { n_nearest: 0, ..HeuristicOptions::default() },
            max_depth: 5,
        };
        assert!(opts.validate().is_err(), "n_nearest=0 must be rejected");
    }

    #[test]
    fn lk_options_validate_rejects_zero_max_depth() {
        let opts = LKOptions { max_depth: 0, ..LKOptions::default() };
        assert!(opts.validate().is_err(), "max_depth=0 must be rejected");
    }

    #[test]
    fn lk_options_from_toml_parses_all_fields() {
        let t: toml::Table =
            toml::from_str("epochs=50\nn_nearest=7\nmax_depth=3").unwrap();
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
            .arg(Arg::new("platoo_epochs").long("platoo_epochs").action(ArgAction::Set))
            .arg(Arg::new("n_nearest").long("n_nearest").action(ArgAction::Set))
            .arg(Arg::new("verbose").long("verbose").action(ArgAction::SetTrue))
            .arg(Arg::new("max_depth").long("max-depth").action(ArgAction::Set));  // hyphen matches production CLI
        let args = cmd.get_matches_from(["t", "--max-depth", "3"]);  // hyphen matches production CLI
        let opts = LKOptions::from_cli(&args).unwrap();
        assert_eq!(opts.max_depth, 3);
    }
}

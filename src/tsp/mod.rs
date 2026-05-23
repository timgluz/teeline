pub mod bellman_karp;
pub mod pipeline;
pub mod branch_bound;
pub mod random_shuffle;
pub mod convert;
pub mod cuckoo_search;
pub mod flower_pollination;
pub mod distance_matrix;
pub mod genetic_algorithm;
pub mod kdtree;
pub mod nearest_neighbor;
pub mod progress;
#[cfg(feature = "gui")]
pub mod progress_eframe;
pub mod route;
pub mod simulated_annealing;
pub mod stochastic_hill;
pub mod tabu_search;
pub mod opt_tour;
pub mod particle_swarm;
pub mod probability;
pub mod tsplib;
pub mod three_opt;
pub mod two_opt;

use crate::tsp::distance_matrix::DistanceMatrix;
use crate::tsp::kdtree::KDPoint;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::mpsc;

pub const VERSION: &str = "0.6.1";
pub const AUTHOR: &str = "Timo Sulg <timo@sulg.dev>";

use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Solvers {
    BellmanKarp,
    BranchBound,
    CuckooSearch,
    FlowerPollination,
    NearestNeighbor,
    GeneticAlgorithm,
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
            "two_opt",
            "2opt",
            "classic",
            "fast",
            "thorough",
        ]
    }

    /// Deterministic local-search solvers: monotone hill-climbers that can only reach
    /// solutions reachable from their starting tour. A better start always means a better end.
    pub fn auto_expand_with_nn(&self) -> bool {
        matches!(self, Solvers::TwoOpt | Solvers::ThreeOpt | Solvers::TabuSearch)
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

impl FromStr for Solvers {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bhk" | "bellman_karp" => Ok(Solvers::BellmanKarp),
            "branch_bound" => Ok(Solvers::BranchBound),
            "cs" | "cuckoo_search" => Ok(Solvers::CuckooSearch),
            "fpa" | "flower_pollination" => Ok(Solvers::FlowerPollination),
            "nn" | "nearest_neighbor" => Ok(Solvers::NearestNeighbor),
            "ga" | "genetic_algorithm" => Ok(Solvers::GeneticAlgorithm),
            "pso" | "particle_swarm" => Ok(Solvers::ParticleSwarmOptimization),
            "shuffle" | "random_shuffle" => Ok(Solvers::RandomShuffle),
            "sa" | "simulated_annealing" => Ok(Solvers::SimulatedAnnealing),
            "stochastic_hill" => Ok(Solvers::StochasticHill),
            "tabu_search" => Ok(Solvers::TabuSearch),
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
        HeuristicOptions { epochs: 10_000, platoo_epochs: 500, n_nearest: 3, verbose: false }
    }
}

impl HeuristicOptions {
    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut h = HeuristicOptions::default();
        for (k, v) in table.iter() {
            match k.as_str() {
                "epochs" => {
                    h.epochs = v.as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))? as usize;
                }
                "platoo_epochs" => {
                    h.platoo_epochs = v.as_integer()
                        .ok_or_else(|| format!("config: `platoo_epochs` must be an integer, got {v}"))? as usize;
                }
                "n_nearest" => {
                    h.n_nearest = v.as_integer()
                        .ok_or_else(|| format!("config: `n_nearest` must be an integer, got {v}"))? as usize;
                }
                "verbose" => {
                    h.verbose = v.as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                other => return Err(format!(
                    "config: unknown field `{other}` in [heuristic] — valid: epochs, platoo_epochs, n_nearest, verbose"
                )),
            }
        }
        Ok(h)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Self {
        let mut h = HeuristicOptions::default();
        if let Some(v) = args.get_one::<String>("epochs") {
            h.epochs = v.parse().unwrap_or(h.epochs);
        }
        if let Some(v) = args.get_one::<String>("platoo_epochs") {
            h.platoo_epochs = v.parse().unwrap_or(h.platoo_epochs);
        }
        if let Some(v) = args.get_one::<String>("n_nearest") {
            h.n_nearest = v.parse().unwrap_or(h.n_nearest);
        }
        if args.get_flag("verbose") {
            h.verbose = true;
        }
        h
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
        if self.cooling_rate <= 0.0 {
            return Err(format!("cooling_rate must be > 0 (got {})", self.cooling_rate));
        }
        if self.cooling_rate >= 1.0 {
            return Err(format!("cooling_rate must be < 1 (got {})", self.cooling_rate));
        }
        if self.max_temperature <= 0.0 {
            return Err(format!("max_temperature must be > 0 (got {})", self.max_temperature));
        }
        if self.min_temperature < 0.0 {
            return Err(format!("min_temperature must be >= 0 (got {})", self.min_temperature));
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
                    sa.heuristic.epochs = v.as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))? as usize;
                }
                "platoo_epochs" => {
                    sa.heuristic.platoo_epochs = v.as_integer()
                        .ok_or_else(|| format!("config: `platoo_epochs` must be an integer, got {v}"))? as usize;
                }
                "n_nearest" => {
                    sa.heuristic.n_nearest = v.as_integer()
                        .ok_or_else(|| format!("config: `n_nearest` must be an integer, got {v}"))? as usize;
                }
                "verbose" => {
                    sa.heuristic.verbose = v.as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                "cooling_rate" => { sa.cooling_rate = parse_f32(v, "sa.cooling_rate")?; }
                "max_temperature" => { sa.max_temperature = parse_f32(v, "sa.max_temperature")?; }
                "min_temperature" => { sa.min_temperature = parse_f32(v, "sa.min_temperature")?; }
                other => return Err(format!(
                    "config: unknown field `{other}` in [sa] — valid: epochs, platoo_epochs, n_nearest, verbose, cooling_rate, max_temperature, min_temperature"
                )),
            }
        }
        sa.validate()?;
        Ok(sa)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Self {
        let mut sa = SAOptions::default();
        if let Some(v) = args.get_one::<String>("epochs") {
            sa.heuristic.epochs = v.parse().unwrap_or(sa.heuristic.epochs);
        }
        if let Some(v) = args.get_one::<String>("platoo_epochs") {
            sa.heuristic.platoo_epochs = v.parse().unwrap_or(sa.heuristic.platoo_epochs);
        }
        if let Some(v) = args.get_one::<String>("n_nearest") {
            sa.heuristic.n_nearest = v.parse().unwrap_or(sa.heuristic.n_nearest);
        }
        if args.get_flag("verbose") { sa.heuristic.verbose = true; }
        if let Some(v) = args.get_one::<String>("cooling_rate") {
            sa.cooling_rate = v.parse().unwrap_or(sa.cooling_rate);
        }
        if let Some(v) = args.get_one::<String>("min_temperature") {
            sa.min_temperature = v.parse().unwrap_or(sa.min_temperature);
        }
        if let Some(v) = args.get_one::<String>("max_temperature") {
            sa.max_temperature = v.parse().unwrap_or(sa.max_temperature);
        }
        sa
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
                    ga.heuristic.epochs = v.as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))? as usize;
                }
                "platoo_epochs" => {
                    ga.heuristic.platoo_epochs = v.as_integer()
                        .ok_or_else(|| format!("config: `platoo_epochs` must be an integer, got {v}"))? as usize;
                }
                "n_nearest" => {
                    ga.heuristic.n_nearest = v.as_integer()
                        .ok_or_else(|| format!("config: `n_nearest` must be an integer, got {v}"))? as usize;
                }
                "verbose" => {
                    ga.heuristic.verbose = v.as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                "mutation_probability" => { ga.mutation_probability = parse_f32(v, "ga.mutation_probability")?; }
                "n_elite" => {
                    ga.n_elite = v.as_integer()
                        .ok_or_else(|| format!("config: `ga.n_elite` must be an integer, got {v}"))? as usize;
                }
                other => return Err(format!(
                    "config: unknown field `{other}` in [ga] — valid: epochs, platoo_epochs, n_nearest, verbose, mutation_probability, n_elite"
                )),
            }
        }
        ga.validate()?;
        Ok(ga)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Self {
        let mut ga = GAOptions::default();
        if let Some(v) = args.get_one::<String>("epochs") {
            ga.heuristic.epochs = v.parse().unwrap_or(ga.heuristic.epochs);
        }
        if let Some(v) = args.get_one::<String>("platoo_epochs") {
            ga.heuristic.platoo_epochs = v.parse().unwrap_or(ga.heuristic.platoo_epochs);
        }
        if let Some(v) = args.get_one::<String>("n_nearest") {
            ga.heuristic.n_nearest = v.parse().unwrap_or(ga.heuristic.n_nearest);
        }
        if args.get_flag("verbose") { ga.heuristic.verbose = true; }
        if let Some(v) = args.get_one::<String>("mutation_probability") {
            ga.mutation_probability = v.parse().unwrap_or(ga.mutation_probability);
        }
        if let Some(v) = args.get_one::<String>("n_elite") {
            ga.n_elite = v.parse().unwrap_or(ga.n_elite);
        }
        ga
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CSOptions {
    pub heuristic: HeuristicOptions,
    pub mutation_probability: f32,
}

impl Default for CSOptions {
    fn default() -> Self {
        CSOptions { heuristic: HeuristicOptions::default(), mutation_probability: 0.001 }
    }
}

impl CSOptions {
    pub fn validate(&self) -> Result<(), String> {
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
                    cs.heuristic.epochs = v.as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))? as usize;
                }
                "platoo_epochs" => {
                    cs.heuristic.platoo_epochs = v.as_integer()
                        .ok_or_else(|| format!("config: `platoo_epochs` must be an integer, got {v}"))? as usize;
                }
                "n_nearest" => {
                    cs.heuristic.n_nearest = v.as_integer()
                        .ok_or_else(|| format!("config: `n_nearest` must be an integer, got {v}"))? as usize;
                }
                "verbose" => {
                    cs.heuristic.verbose = v.as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                "mutation_probability" => { cs.mutation_probability = parse_f32(v, "cs.mutation_probability")?; }
                other => return Err(format!(
                    "config: unknown field `{other}` in [cs] — valid: epochs, platoo_epochs, n_nearest, verbose, mutation_probability"
                )),
            }
        }
        cs.validate()?;
        Ok(cs)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Self {
        let mut cs = CSOptions::default();
        if let Some(v) = args.get_one::<String>("epochs") {
            cs.heuristic.epochs = v.parse().unwrap_or(cs.heuristic.epochs);
        }
        if let Some(v) = args.get_one::<String>("platoo_epochs") {
            cs.heuristic.platoo_epochs = v.parse().unwrap_or(cs.heuristic.platoo_epochs);
        }
        if let Some(v) = args.get_one::<String>("n_nearest") {
            cs.heuristic.n_nearest = v.parse().unwrap_or(cs.heuristic.n_nearest);
        }
        if args.get_flag("verbose") { cs.heuristic.verbose = true; }
        if let Some(v) = args.get_one::<String>("mutation_probability") {
            cs.mutation_probability = v.parse().unwrap_or(cs.mutation_probability);
        }
        cs
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FPAOptions {
    pub heuristic: HeuristicOptions,
    pub mutation_probability: f32,
}

impl Default for FPAOptions {
    fn default() -> Self {
        FPAOptions { heuristic: HeuristicOptions::default(), mutation_probability: 0.001 }
    }
}

impl FPAOptions {
    pub fn validate(&self) -> Result<(), String> {
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
                    fpa.heuristic.epochs = v.as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {v}"))? as usize;
                }
                "platoo_epochs" => {
                    fpa.heuristic.platoo_epochs = v.as_integer()
                        .ok_or_else(|| format!("config: `platoo_epochs` must be an integer, got {v}"))? as usize;
                }
                "n_nearest" => {
                    fpa.heuristic.n_nearest = v.as_integer()
                        .ok_or_else(|| format!("config: `n_nearest` must be an integer, got {v}"))? as usize;
                }
                "verbose" => {
                    fpa.heuristic.verbose = v.as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {v}"))?;
                }
                "mutation_probability" => { fpa.mutation_probability = parse_f32(v, "fpa.mutation_probability")?; }
                other => return Err(format!(
                    "config: unknown field `{other}` in [fpa] — valid: epochs, platoo_epochs, n_nearest, verbose, mutation_probability"
                )),
            }
        }
        fpa.validate()?;
        Ok(fpa)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Self {
        let mut fpa = FPAOptions::default();
        if let Some(v) = args.get_one::<String>("epochs") {
            fpa.heuristic.epochs = v.parse().unwrap_or(fpa.heuristic.epochs);
        }
        if let Some(v) = args.get_one::<String>("platoo_epochs") {
            fpa.heuristic.platoo_epochs = v.parse().unwrap_or(fpa.heuristic.platoo_epochs);
        }
        if let Some(v) = args.get_one::<String>("n_nearest") {
            fpa.heuristic.n_nearest = v.parse().unwrap_or(fpa.heuristic.n_nearest);
        }
        if args.get_flag("verbose") { fpa.heuristic.verbose = true; }
        if let Some(v) = args.get_one::<String>("mutation_probability") {
            fpa.mutation_probability = v.parse().unwrap_or(fpa.mutation_probability);
        }
        fpa
    }
}

// ---------------------------------------------------------------------------
// AppOptions — pure config shell; no runtime state
// ---------------------------------------------------------------------------

/// Pure config container. No progress channel, no initial tour — those are runtime concerns.
/// Each field is populated only for the solver that uses it.
#[derive(Clone, Debug, Default)]
pub struct AppOptions {
    pub sa:        Option<SAOptions>,
    pub ga:        Option<GAOptions>,
    pub cs:        Option<CSOptions>,
    pub fpa:       Option<FPAOptions>,
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
        return Err(format!("tour length {} != cities length {}", tour.len(), cities.len()));
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

pub(crate) fn solve_with_context(
    solver: Solvers,
    problem: &TspProblem,
    opts: &AppOptions,
    progress_tx: Option<mpsc::Sender<progress::ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Result<Solution, String> {
    let tx = progress_tx.as_ref();
    let h = opts.heuristic.as_ref().cloned().unwrap_or_default();
    let solution = match solver {
        Solvers::BellmanKarp               => bellman_karp::solve(problem, &h, tx, init_tour),
        Solvers::BranchBound               => branch_bound::solve(problem, &h, tx, init_tour),
        Solvers::CuckooSearch              => { let cs = opts.cs.as_ref().cloned().unwrap_or_default(); cuckoo_search::solve(problem, &cs, tx, init_tour) }
        Solvers::FlowerPollination         => { let fpa = opts.fpa.as_ref().cloned().unwrap_or_default(); flower_pollination::solve(problem, &fpa, tx, init_tour) }
        Solvers::NearestNeighbor           => nearest_neighbor::solve(problem, &h, tx, init_tour),
        Solvers::GeneticAlgorithm          => { let ga = opts.ga.as_ref().cloned().unwrap_or_default(); genetic_algorithm::solve(problem, &ga, tx, init_tour) }
        Solvers::ParticleSwarmOptimization => particle_swarm::solve(problem, &h, tx, init_tour),
        Solvers::RandomShuffle             => random_shuffle::solve(problem, &h, tx, init_tour),
        Solvers::SimulatedAnnealing        => { let sa = opts.sa.as_ref().cloned().unwrap_or_default(); simulated_annealing::solve(problem, &sa, tx, init_tour) }
        Solvers::StochasticHill            => stochastic_hill::solve(problem, &h, tx, init_tour),
        Solvers::TabuSearch                => tabu_search::solve(problem, &h, tx, init_tour),
        Solvers::ThreeOpt                  => three_opt::solve(problem, &h, tx, init_tour),
        Solvers::TwoOpt                    => two_opt::solve(problem, &h, tx, init_tour),
        Solvers::Unspecified               => return Err("solver not specified".to_string()),
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
    cities.iter().map(|c| (c.id, c.clone())).collect()
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
        let cities_idx = cities
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i))
            .collect();

        Solution {
            total: distances.tour_length(route),
            route: route.to_vec(),
            cities: cities.to_vec(),
            cities_idx,
        }
    }

    /// Convenience constructor for solver functions that already hold separate `cities`
    /// and `distances` slices and do not need to construct a full [`TspProblem`].
    pub(crate) fn from_parts(route: &[usize], cities: &[kdtree::KDPoint], distances: &DistanceMatrix) -> Self {
        let cities_idx = cities
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i))
            .collect();

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
            target: point.clone(),
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
            self.point = pt.clone();
        }

        if new_distance < self.farthest_distance() {
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
        let h = HeuristicOptions { epochs: 100, platoo_epochs: 10, n_nearest: 3, verbose: false };
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
        let a = AppOptions { sa: None, ga: None, cs: None, fpa: None, heuristic: None };
        drop(a);
    }

    #[test]
    fn test_heuristic_options_from_toml() {
        let t: toml::Table = toml::from_str(
            "epochs=5000\nplatoo_epochs=200\nn_nearest=5\nverbose=true"
        ).unwrap();
        let h = HeuristicOptions::from_toml(&t).unwrap();
        assert_eq!(h.epochs, 5000);
        assert_eq!(h.n_nearest, 5);
        assert!(h.verbose);
    }

    #[test]
    fn test_sa_options_from_toml_parses_all_fields() {
        let t: toml::Table = toml::from_str(
            "epochs=5000\ncooling_rate=0.0005\nmax_temperature=200.0\nmin_temperature=0.001"
        ).unwrap();
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
        let t: toml::Table = toml::from_str(
            "mutation_probability=0.05\nn_elite=5\nepochs=2000"
        ).unwrap();
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
        assert!(!Solvers::BranchBound.auto_expand_with_nn());
        assert!(!Solvers::Unspecified.auto_expand_with_nn());
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
}

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

// -- SolverOptions

#[derive(Clone)]
pub struct SolverOptions {
    pub epochs: usize,
    pub platoo_epochs: usize,
    pub verbose: bool,
    pub n_nearest: usize,
    pub mutation_probability: f32,
    pub n_elite: usize,
    pub cooling_rate: f32,
    pub max_temperature: f32,
    pub min_temperature: f32,
    pub progress_tx: Option<mpsc::Sender<progress::ProgressMessage>>,
    pub initial_tour: Option<Vec<usize>>,
}

impl std::fmt::Debug for SolverOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SolverOptions")
            .field("epochs", &self.epochs)
            .field("platoo_epochs", &self.platoo_epochs)
            .field("verbose", &self.verbose)
            .field("n_nearest", &self.n_nearest)
            .field("mutation_probability", &self.mutation_probability)
            .field("n_elite", &self.n_elite)
            .field("cooling_rate", &self.cooling_rate)
            .field("max_temperature", &self.max_temperature)
            .field("min_temperature", &self.min_temperature)
            .field("progress_tx", &self.progress_tx.is_some())
            .field("initial_tour", &self.initial_tour.is_some())
            .finish()
    }
}

impl Default for SolverOptions {
    fn default() -> Self {
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
            progress_tx: None,
            initial_tour: None,
        }
    }
}

impl SolverOptions {
    pub fn send_progress(&self, msg: progress::ProgressMessage) {
        if let Some(ref tx) = self.progress_tx {
            let _ = tx.send(msg);
        }
    }

    pub fn for_internal_seed(&self) -> SolverOptions {
        SolverOptions {
            progress_tx: None,
            initial_tour: None,
            ..self.clone()
        }
    }
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

pub fn solve(
    solver: Solvers,
    cities: &[KDPoint],
    distances: &DistanceMatrix,
    opts: &SolverOptions,
) -> Result<Solution, String> {
    let solution = match solver {
        Solvers::BellmanKarp => bellman_karp::solve(cities, distances, opts),
        Solvers::BranchBound => branch_bound::solve(cities, distances, opts),
        Solvers::CuckooSearch => cuckoo_search::solve(cities, distances, opts),
        Solvers::FlowerPollination => flower_pollination::solve(cities, distances, opts),
        Solvers::NearestNeighbor => nearest_neighbor::solve(cities, distances, opts),
        Solvers::GeneticAlgorithm => genetic_algorithm::solve(cities, distances, opts),
        Solvers::ParticleSwarmOptimization => particle_swarm::solve(cities, distances, opts),
        Solvers::RandomShuffle => random_shuffle::solve(cities, distances, opts),
        Solvers::SimulatedAnnealing => simulated_annealing::solve(cities, distances, opts),
        Solvers::StochasticHill => stochastic_hill::solve(cities, distances, opts),
        Solvers::TabuSearch => tabu_search::solve(cities, distances, opts),
        Solvers::ThreeOpt => three_opt::solve(cities, distances, opts),
        Solvers::TwoOpt => two_opt::solve(cities, distances, opts),
        Solvers::Unspecified => return Err("solver not specified".to_string()),
    };
    Ok(solution)
}

// -- solution implementation
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
    pub fn new(route: &[usize], cities: &[kdtree::KDPoint], distances: &DistanceMatrix) -> Self {
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
    fn test_send_progress_with_none_is_noop() {
        let options = SolverOptions::default();
        options.send_progress(progress::ProgressMessage::Done);
    }

    #[test]
    fn test_send_progress_with_channel_delivers_message() {
        use std::sync::mpsc;
        let (tx, rx) = mpsc::channel();
        let mut options = SolverOptions::default();
        options.progress_tx = Some(tx);
        options.send_progress(progress::ProgressMessage::EpochUpdate(99));
        match rx.recv().unwrap() {
            progress::ProgressMessage::EpochUpdate(n) => assert_eq!(n, 99),
            other => panic!("unexpected message: {:?}", other),
        }
    }

    #[test]
    fn test_solver_options_initial_tour_defaults_to_none() {
        assert!(SolverOptions::default().initial_tour.is_none());
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
        // Stochastic solvers use shuffle expansion, not NN
        assert!(!Solvers::SimulatedAnnealing.auto_expand_with_nn());
        assert!(!Solvers::StochasticHill.auto_expand_with_nn());
        assert!(!Solvers::GeneticAlgorithm.auto_expand_with_nn());
        assert!(!Solvers::ParticleSwarmOptimization.auto_expand_with_nn());
        assert!(!Solvers::CuckooSearch.auto_expand_with_nn());
        assert!(!Solvers::FlowerPollination.auto_expand_with_nn());
        // Constructors: no expansion
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
    fn test_for_internal_seed_clears_initial_tour_and_progress() {
        use std::sync::mpsc;
        let (tx, _rx) = mpsc::channel();
        let mut opts = SolverOptions::default();
        opts.initial_tour = Some(vec![1, 2, 3]);
        opts.progress_tx = Some(tx);
        let inner = opts.for_internal_seed();
        assert!(inner.initial_tour.is_none());
        assert!(inner.progress_tx.is_none());
        assert_eq!(inner.epochs, opts.epochs); // other fields preserved
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
        let sol = Solution::new(&route, &cities, &dm);
        assert_approx(4.0, sol.total);
    }
}

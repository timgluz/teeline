pub mod bellman_karp;
pub mod branch_bound;
pub mod distance_matrix;
pub mod genetic_algorithm;
pub mod kdtree;
pub mod nearest_neighbor;
pub mod progress;
pub mod route;
pub mod simulated_annealing;
pub mod stochastic_hill;
pub mod tabu_search;
pub mod tsplib;
pub mod two_opt;

use crate::tsp::kdtree::KDPoint;
use std::cmp::Ordering;
use std::collections::HashMap;

pub const VERSION: &'static str = "0.6.1";
pub const AUTHOR: &'static str = "Timo Sulg <timo@sulg.dev>";

use std::fmt;
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

#[derive(Clone, Debug)]
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
    pub show_progress: bool, // should we show and print progress
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
            show_progress: true,
        }
    }
}

// -- solution implementation
pub type CityTable = HashMap<usize, KDPoint>;

pub fn total_distance(cities: &[KDPoint], route: &[usize]) -> f32 {
    let mut total = 0.0;
    let last_idx = route.len() - 1;

    let cities_table = city_table_from_vec(cities);
    for i in 0..last_idx {
        let distance = cities_table[&route[i]].distance(&cities_table[&route[i + 1]]);
        total += distance
    }

    total += cities_table[&route[last_idx]].distance(&cities_table[&route[0]]);

    total
}

pub fn city_table_from_vec(cities: &[kdtree::KDPoint]) -> CityTable {
    let table: CityTable = cities.iter().map(|c| (c.id, c.clone())).collect();

    return table;
}

pub struct Solution {
    pub total: f32,
    route: Vec<usize>,
    cities: Vec<KDPoint>,
    cities_idx: HashMap<usize, usize>, // it maps city.id to internal vector_id
}

impl Solution {
    pub fn new(route: &[usize], cities: &[kdtree::KDPoint]) -> Self {
        let idx: HashMap<usize, usize> =
            cities.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

        let mut solution = Solution {
            total: 0.0,
            route: route.to_vec(),
            cities: cities.to_vec(),
            cities_idx: idx,
        };

        solution.update_total();

        solution
    }

    pub fn len(&self) -> usize {
        self.route.len()
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

    pub fn update_total(&mut self) {
        self.total = total_distance(self.cities(), self.route());
    }
}

#[derive(Debug, Clone)]
pub struct NearestResult {
    pub target: KDPoint,
    pub point: KDPoint, // the best result, may be the exact match
    pub distance: f32,  // best distance
    pub n: usize,       // how many nearest items to keep
    // we expect that we look only small number of items
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

        // we only keep the best results
        if new_distance < self.farthest_distance() {
            // if stack is full, then remove the weakest result
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
    fn test_total_distance_for_tsp_5_1() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);

        let route = vec![0, 1, 2, 3, 4];

        assert_approx(4.0, total_distance(&cities, &route));
    }
}

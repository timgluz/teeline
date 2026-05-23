use std::sync::mpsc;

use rand::Rng;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::{HeuristicOptions, Solution};

/// Returns a uniformly random permutation of the city IDs.
/// Used as a lightweight first pipeline stage for stochastic solvers that rely on
/// broad high-temperature exploration — a random start outperforms a greedy NN start
/// for those algorithms because the temperature schedule is calibrated for cold starts.
pub fn solve(
    cities: &[KDPoint],
    distances: &DistanceMatrix,
    _opts: &HeuristicOptions,
    _progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _initial_tour: Option<&[usize]>,
) -> Solution {
    let mut rng = rand::rng();
    let mut path: Vec<usize> = cities.iter().map(|c| c.id).collect();
    for i in (1..path.len()).rev() {
        let j = rng.random_range(0..=i);
        path.swap(i, j);
    }
    Solution::new(&path, cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, HeuristicOptions};

    fn five_cities() -> Vec<KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
            vec![0.5, 0.5],
        ])
    }

    #[test]
    fn test_random_shuffle_produces_valid_tour() {
        let cities = five_cities();
        let dm = distance_matrix::from_cities(&cities);
        let result = solve(&cities, &dm, &HeuristicOptions::default(), None, None);

        assert_eq!(result.len(), cities.len());
        let mut visited = result.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected);
    }

    #[test]
    fn test_random_shuffle_tour_length_is_positive() {
        let cities = five_cities();
        let dm = distance_matrix::from_cities(&cities);
        let result = solve(&cities, &dm, &HeuristicOptions::default(), None, None);
        assert!(result.total > 0.0);
    }
}

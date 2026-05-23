use std::sync::mpsc;

use rand::Rng;

use super::progress::ProgressMessage;
use super::{HeuristicOptions, Solution, TspProblem};

/// Returns a uniformly random permutation of the city IDs.
/// Used as a lightweight first pipeline stage for stochastic solvers that rely on
/// broad high-temperature exploration — a random start outperforms a greedy NN start
/// for those algorithms because the temperature schedule is calibrated for cold starts.
pub fn solve(
    problem: &TspProblem,
    _opts: &HeuristicOptions,
    _progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let mut rng = rand::rng();
    let mut path: Vec<usize> = cities.iter().map(|c| c.id).collect();
    for i in (1..path.len()).rev() {
        let j = rng.random_range(0..=i);
        path.swap(i, j);
    }
    Solution::from_parts(&path, cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, HeuristicOptions, TspProblem};

    fn five_city_problem() -> TspProblem {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
            vec![0.5, 0.5],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        TspProblem::new(cities, dm)
    }

    #[test]
    fn test_random_shuffle_produces_valid_tour() {
        let problem = five_city_problem();
        let n = problem.cities.len();
        let result = solve(&problem, &HeuristicOptions::default(), None);

        assert_eq!(result.len(), n);
        let mut visited = result.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = problem.cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected);
    }

    #[test]
    fn test_random_shuffle_tour_length_is_positive() {
        let problem = five_city_problem();
        let result = solve(&problem, &HeuristicOptions::default(), None);
        assert!(result.total > 0.0);
    }
}

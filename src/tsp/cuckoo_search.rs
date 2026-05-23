use std::sync::mpsc;

use rand::Rng;

use super::probability::levy_step;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{CSOptions, Solution, TspProblem};

const DEFAULT_N_NESTS: usize = 25;


fn apply_k_random_2opt(tour: &[usize], k: usize, rng: &mut impl Rng) -> Vec<usize> {
    let n = tour.len();
    let mut result = tour.to_vec();
    if n < 2 {
        return result;
    }
    for _ in 0..k {
        let i = rng.random_range(0..n - 1);
        let j = rng.random_range(i + 1..n);
        result[i..=j].reverse();
    }
    result
}

pub fn solve(
    problem: &TspProblem,
    opts: &CSOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n_nests = opts.heuristic.n_nearest.max(DEFAULT_N_NESTS);
    let pa = (opts.mutation_probability as f64).clamp(0.01, 0.99);
    let n_cities = cities.len();

    let mut rng = rand::rng();
    let city_ids: Vec<usize> = cities.iter().map(|c| c.id).collect();

    let mut nests: Vec<Vec<usize>> = (0..n_nests)
        .map(|idx| {
            if idx == 0 {
                problem.initial_tour.as_deref().map(|t| t.to_vec()).unwrap_or_else(|| {
                    let mut t = city_ids.clone();
                    for i in (1..n_cities).rev() {
                        let j = rng.random_range(0..=i);
                        t.swap(i, j);
                    }
                    t
                })
            } else {
                let mut t = city_ids.clone();
                for i in (1..n_cities).rev() {
                    let j = rng.random_range(0..=i);
                    t.swap(i, j);
                }
                t
            }
        })
        .collect();

    let mut costs: Vec<f32> = nests.iter().map(|t| distances.tour_length(t)).collect();

    let (best_idx, _) = costs
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    let mut best: Vec<usize> = nests[best_idx].clone();
    let mut best_cost: f32 = costs[best_idx];

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&best), best_cost));
    }

    for epoch in 0..opts.heuristic.epochs {
        for cuckoo_idx in 0..n_nests {
            let levy = levy_step(&mut rng).abs();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
            let k = ((levy * (n_cities as f64 * 0.1)).ceil() as usize).clamp(1, n_cities / 2);

            let new_tour = apply_k_random_2opt(&nests[cuckoo_idx], k, &mut rng);
            let new_cost = distances.tour_length(&new_tour);

            let target_idx = rng.random_range(0..n_nests);
            if new_cost < costs[target_idx] {
                nests[target_idx] = new_tour;
                costs[target_idx] = new_cost;

                if new_cost < best_cost {
                    best = nests[target_idx].clone();
                    best_cost = new_cost;
                    if let Some(tx) = progress_tx {
                        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&best), best_cost));
                    }
                }
            }
        }

        for idx in 0..n_nests {
            let r: f64 = rng.random();
            if r < pa {
                let mut t = city_ids.clone();
                for i in (1..n_cities).rev() {
                    let j = rng.random_range(0..=i);
                    t.swap(i, j);
                }
                let cost = distances.tour_length(&t);
                nests[idx] = t;
                costs[idx] = cost;

                if cost < best_cost {
                    best = nests[idx].clone();
                    best_cost = cost;
                    if let Some(tx) = progress_tx {
                        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&best), best_cost));
                    }
                }
            }
        }

        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::EpochUpdate(epoch));
        }
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&best, cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, CSOptions, HeuristicOptions, TspProblem};

    #[test]
    fn test_cs_respects_initial_tour() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let optimal: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let optimal_cost = dm.tour_length(&optimal);
        let opts = CSOptions {
            heuristic: HeuristicOptions { epochs: 0, ..HeuristicOptions::default() },
            ..CSOptions::default()
        };
        let problem = TspProblem { cities: cities.clone(), distances: dm, initial_tour: Some(optimal) };
        let result = solve(&problem, &opts, None);
        assert!((result.total - optimal_cost).abs() < 1e-4);
        let mut visited = result.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected);
    }

    #[test]
    fn test_apply_k_random_2opt_preserves_cities() {
        let tour = vec![0usize, 1, 2, 3, 4];
        let mut rng = rand::rng();
        let result = apply_k_random_2opt(&tour, 3, &mut rng);
        let mut sorted = result.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_apply_k_0_returns_clone() {
        let tour = vec![2usize, 0, 4, 1, 3];
        let mut rng = rand::rng();
        let result = apply_k_random_2opt(&tour, 0, &mut rng);
        assert_eq!(result, tour);
    }

    #[test]
    fn test_solve_returns_valid_tour_on_small_instance() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
        ]);
        let distances = distance_matrix::from_cities(&cities);
        let opts = CSOptions {
            heuristic: HeuristicOptions { epochs: 30, n_nearest: 5, ..HeuristicOptions::default() },
            mutation_probability: 0.25,
        };
        let problem = TspProblem::new(cities.clone(), distances);
        let sol = solve(&problem, &opts, None);
        let mut visited = sol.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected, "CS tour does not visit all cities exactly once");
        assert!(sol.total > 0.0);
    }
}

use rand::Rng;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::probability::levy_step;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{Solution, SolverOptions};
const DEFAULT_N_NESTS: usize = 25;

/// Greedy nearest-neighbour tour starting from the first city.
/// Seeds nest 0 so the population starts from a reasonable neighbourhood.
fn nn_seed(city_ids: &[usize], distances: &DistanceMatrix) -> Vec<usize> {
    let n = city_ids.len();
    let mut unvisited: Vec<usize> = city_ids.to_vec();
    let mut tour = Vec::with_capacity(n);
    let mut current = unvisited.swap_remove(0);
    tour.push(current);
    while !unvisited.is_empty() {
        let best_pos = unvisited
            .iter()
            .enumerate()
            .min_by(|(_, &a), (_, &b)| {
                let da = distances.distance_between(current, a).unwrap_or(f32::MAX);
                let db = distances.distance_between(current, b).unwrap_or(f32::MAX);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap();
        current = unvisited.swap_remove(best_pos);
        tour.push(current);
    }
    tour
}

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

pub fn solve(cities: &[KDPoint], distances: &DistanceMatrix, options: &SolverOptions) -> Solution {
    let n_nests = options.n_nearest.max(DEFAULT_N_NESTS);
    let pa = (options.mutation_probability as f64).clamp(0.01, 0.99);
    let n_cities = cities.len();

    let mut rng = rand::rng();
    let city_ids: Vec<usize> = cities.iter().map(|c| c.id).collect();

    // Nest 0 is seeded with a greedy NN tour so the population starts from a good
    // neighbourhood; the rest are random Fisher-Yates shuffles for diversity.
    let mut nests: Vec<Vec<usize>> = (0..n_nests)
        .map(|idx| {
            if idx == 0 {
                nn_seed(&city_ids, distances)
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

    options.send_progress(ProgressMessage::PathUpdate(Route::new(&best), best_cost));

    for epoch in 0..options.epochs {
        // 1. Each nest generates one cuckoo via Lévy flight (Yang & Deb 2009: n cuckoos/epoch).
        //    |levy_step| is used as step magnitude; sign is irrelevant for a discrete mapping.
        for cuckoo_idx in 0..n_nests {
            let levy = levy_step(&mut rng).abs();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
            let k = ((levy * (n_cities as f64 * 0.1)).ceil() as usize).clamp(1, n_cities / 2);

            let new_tour = apply_k_random_2opt(&nests[cuckoo_idx], k, &mut rng);
            let new_cost = distances.tour_length(&new_tour);

            // Replace a randomly chosen nest if cuckoo is better.
            // cuckoo_idx may equal target_idx; harmless (nest replaces itself only if better).
            let target_idx = rng.random_range(0..n_nests);
            if new_cost < costs[target_idx] {
                nests[target_idx] = new_tour;
                costs[target_idx] = new_cost;

                if new_cost < best_cost {
                    best = nests[target_idx].clone();
                    best_cost = new_cost;
                    options.send_progress(ProgressMessage::PathUpdate(Route::new(&best), best_cost));
                }
            }
        }

        // 2. Per-nest Bernoulli abandonment (Yang & Deb 2009): each nest independently
        //    re-seeded with probability pa, preserving discovered information per epoch.
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
                    options.send_progress(ProgressMessage::PathUpdate(Route::new(&best), best_cost));
                }
            }
        }

        options.send_progress(ProgressMessage::EpochUpdate(epoch));
    }

    options.send_progress(ProgressMessage::Done);
    Solution::new(&best, cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree};

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
        let options = SolverOptions {
            epochs: 30,
            n_nearest: 5,
            mutation_probability: 0.25,
            ..SolverOptions::default()
        };
        let sol = solve(&cities, &distances, &options);
        let mut visited = sol.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected, "CS tour does not visit all cities exactly once");
        assert!(sol.total > 0.0);
    }
}

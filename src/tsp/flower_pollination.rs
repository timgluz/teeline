use rand::Rng;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::probability::{bernoulli, levy_step, sample_with_exclude};
use super::progress::ProgressMessage;
use super::route::{apply_swaps, swap_sequence, Route};
use super::{Solution, SolverOptions};

const DEFAULT_N_FLOWERS: usize = 25;

/// Greedy nearest-neighbour tour starting from the first city.
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

/// Global pollination: move `flower` toward `gbest` by a Lévy-scaled fraction of the swap
/// sequence between them — the permutation analogue of `x + γ·L·(g* − x)` (Yang 2012).
/// Returns the current flower unchanged if it already equals `gbest`.
fn global_pollination(flower: &[usize], gbest: &[usize], rng: &mut impl Rng) -> Vec<usize> {
    let seq = swap_sequence(flower, gbest);
    if seq.is_empty() {
        return flower.to_vec();
    }
    let levy = levy_step(rng).abs();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
    let n_swaps = ((levy * seq.len() as f64 * 0.5).ceil() as usize).clamp(1, seq.len());
    apply_swaps(flower, &seq[..n_swaps])
}

/// Local pollination: apply an ε-scaled fraction of the displacement between two randomly
/// chosen flowers `j` and `k` to `flower` — the permutation analogue of `x + ε·(x_j − x_k)`.
/// Returns the current flower unchanged when `n_flowers < 3` or `flowers[j] == flowers[k]`.
fn local_pollination(
    flower: &[usize],
    flowers: &[Vec<usize>],
    flower_idx: usize,
    rng: &mut impl Rng,
) -> Vec<usize> {
    let n_flowers = flowers.len();
    if n_flowers < 3 {
        return flower.to_vec();
    }
    let j = sample_with_exclude(rng, n_flowers, &[flower_idx]);
    let k = sample_with_exclude(rng, n_flowers, &[flower_idx, j]);
    let seq = swap_sequence(&flowers[j], &flowers[k]);
    if seq.is_empty() {
        return flower.to_vec();
    }
    let epsilon: f64 = rng.random();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
    let n_swaps = ((epsilon * seq.len() as f64).ceil() as usize).clamp(1, seq.len());
    apply_swaps(flower, &seq[..n_swaps])
}

pub fn solve(cities: &[KDPoint], distances: &DistanceMatrix, options: &SolverOptions) -> Solution {
    let n_flowers = options.n_nearest.max(DEFAULT_N_FLOWERS);
    let n_cities = cities.len();
    // default mutation_probability 0.001 → 99.9% local, which defeats global search;
    // floor to 0.8 so a bare `bin fpa` run is meaningful without extra flags.
    let switch_prob = if options.mutation_probability < 0.01 {
        0.8_f64
    } else {
        options.mutation_probability as f64
    };

    tracing::info!(
        n_flowers,
        switch_prob,
        epochs = options.epochs,
        "FPA starting"
    );

    let mut rng = rand::rng();
    let city_ids: Vec<usize> = cities.iter().map(|c| c.id).collect();

    // Flower 0 seeded with greedy NN for a warm start; rest are random shuffles.
    let mut flowers: Vec<Vec<usize>> = (0..n_flowers)
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

    let mut costs: Vec<f32> = flowers.iter().map(|t| distances.tour_length(t)).collect();

    let (best_idx, _) = costs
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    let mut gbest: Vec<usize> = flowers[best_idx].clone();
    let mut gbest_cost: f32 = costs[best_idx];

    options.send_progress(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));

    for epoch in 0..options.epochs {
        for i in 0..n_flowers {
            let new_x = if bernoulli(&mut rng, switch_prob) {
                global_pollination(&flowers[i], &gbest, &mut rng)
            } else {
                local_pollination(&flowers[i], &flowers, i, &mut rng)
            };

            let new_cost = distances.tour_length(&new_x);
            if new_cost < costs[i] {
                flowers[i] = new_x;
                costs[i] = new_cost;

                if new_cost < gbest_cost {
                    gbest = flowers[i].clone();
                    gbest_cost = new_cost;
                    options.send_progress(ProgressMessage::PathUpdate(
                        Route::new(&gbest),
                        gbest_cost,
                    ));
                    tracing::info!(epoch, tour_length = gbest_cost, "FPA: new best");
                }
            }
        }

        options.send_progress(ProgressMessage::EpochUpdate(epoch));
    }

    options.send_progress(ProgressMessage::Done);
    Solution::new(&gbest, cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree};

    fn four_city_setup() -> (Vec<KDPoint>, DistanceMatrix) {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        (cities, dm)
    }

    #[test]
    fn test_nn_seed_visits_all_cities() {
        let (cities, dm) = four_city_setup();
        let city_ids: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let tour = nn_seed(&city_ids, &dm);
        let mut sorted = tour.clone();
        sorted.sort();
        assert_eq!(sorted, city_ids);
    }

    #[test]
    fn test_solve_returns_valid_tour() {
        let (cities, dm) = four_city_setup();
        let options = SolverOptions {
            epochs: 30,
            n_nearest: 5,
            mutation_probability: 0.8,
            ..SolverOptions::default()
        };
        let sol = solve(&cities, &dm, &options);
        let mut visited = sol.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected, "FPA tour does not visit all cities exactly once");
        assert!(sol.total > 0.0);
        assert!(sol.total.is_finite());
    }
}

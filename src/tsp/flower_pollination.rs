use std::sync::mpsc;

use rand::Rng;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::probability::{bernoulli, levy_step, sample_without_replacement};
use super::progress::ProgressMessage;
use super::route::{apply_swaps, swap_sequence, Route};
use super::{FPAOptions, Solution};

const DEFAULT_N_FLOWERS: usize = 25;


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
    let j = sample_without_replacement(rng, n_flowers, &[flower_idx]);
    let k = sample_without_replacement(rng, n_flowers, &[flower_idx, j]);
    let seq = swap_sequence(&flowers[j], &flowers[k]);
    if seq.is_empty() {
        return flower.to_vec();
    }
    let epsilon: f64 = rng.random();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
    let n_swaps = ((epsilon * seq.len() as f64).ceil() as usize).clamp(1, seq.len());
    apply_swaps(flower, &seq[..n_swaps])
}

pub fn solve(
    cities: &[KDPoint],
    distances: &DistanceMatrix,
    opts: &FPAOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    initial_tour: Option<&[usize]>,
) -> Solution {
    let n_flowers = opts.heuristic.n_nearest.max(DEFAULT_N_FLOWERS);
    let n_cities = cities.len();
    // default mutation_probability 0.001 → 99.9% local, which defeats global search;
    // floor to 0.8 so a bare `bin fpa` run is meaningful without extra flags.
    let switch_prob = if opts.mutation_probability < 0.01 {
        0.8_f64
    } else {
        opts.mutation_probability as f64
    };

    tracing::info!(
        n_flowers,
        switch_prob,
        epochs = opts.heuristic.epochs,
        "FPA starting"
    );

    let mut rng = rand::rng();
    let city_ids: Vec<usize> = cities.iter().map(|c| c.id).collect();

    let mut flowers: Vec<Vec<usize>> = (0..n_flowers)
        .map(|idx| {
            if idx == 0 {
                initial_tour.map(|t| t.to_vec()).unwrap_or_else(|| {
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

    let mut costs: Vec<f32> = flowers.iter().map(|t| distances.tour_length(t)).collect();

    let (best_idx, _) = costs
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    let mut gbest: Vec<usize> = flowers[best_idx].clone();
    let mut gbest_cost: f32 = costs[best_idx];

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));
    }

    for epoch in 0..opts.heuristic.epochs {
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
                    if let Some(tx) = progress_tx {
                        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));
                    }
                    tracing::info!(epoch, tour_length = gbest_cost, "FPA: new best");
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
    Solution::from_parts(&gbest, cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, FPAOptions, HeuristicOptions};

    #[test]
    fn test_fpa_respects_initial_tour() {
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
        let opts = FPAOptions {
            heuristic: HeuristicOptions { epochs: 0, ..HeuristicOptions::default() },
            ..FPAOptions::default()
        };
        let result = solve(&cities, &dm, &opts, None, Some(&optimal));
        assert!((result.total - optimal_cost).abs() < 1e-4);
        let mut visited = result.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected);
    }

    fn four_city_setup() -> (Vec<KDPoint>, super::DistanceMatrix) {
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
    fn test_solve_returns_valid_tour() {
        let (cities, dm) = four_city_setup();
        let opts = FPAOptions {
            heuristic: HeuristicOptions { epochs: 30, n_nearest: 5, ..HeuristicOptions::default() },
            mutation_probability: 0.8,
        };
        let sol = solve(&cities, &dm, &opts, None, None);
        let mut visited = sol.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected, "FPA tour does not visit all cities exactly once");
        assert!(sol.total > 0.0);
        assert!(sol.total.is_finite());
    }
}

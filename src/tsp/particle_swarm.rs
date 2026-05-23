use rand::Rng;
use std::sync::mpsc;

use super::progress::ProgressMessage;
use super::route::{apply_swaps, swap_sequence, Route};
use super::{HeuristicOptions, Solution, TspProblem};

type Swap = (usize, usize);
type Velocity = Vec<Swap>;

const W_MAX: f64 = 0.9;
const W_MIN: f64 = 0.4;
const C1: f64 = 1.5;
const C2: f64 = 1.5;
const DEFAULT_N_PARTICLES: usize = 30;
const V_MAX_FACTOR: f64 = 0.35;


fn trim_velocity(v: &[Swap], keep: usize) -> Velocity {
    v.iter().take(keep).copied().collect()
}

pub fn solve(
    problem: &TspProblem,
    opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n_cities = cities.len();
    let n_particles = opts.n_nearest.max(DEFAULT_N_PARTICLES);
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let v_max = ((n_cities as f64 * V_MAX_FACTOR).ceil() as usize).max(1);

    let mut rng = rand::rng();
    let city_ids: Vec<usize> = cities.iter().map(|c| c.id).collect();

    let mut positions: Vec<Vec<usize>> = (0..n_particles)
        .map(|idx| {
            if idx == 0 {
                problem.initial_tour.as_deref().map(|t| t.to_vec()).unwrap_or_else(|| {
                    let mut p = city_ids.clone();
                    for i in (1..n_cities).rev() {
                        let j = rng.random_range(0..=i);
                        p.swap(i, j);
                    }
                    p
                })
            } else {
                let mut p = city_ids.clone();
                for i in (1..n_cities).rev() {
                    let j = rng.random_range(0..=i);
                    p.swap(i, j);
                }
                p
            }
        })
        .collect();

    let mut velocities: Vec<Velocity> = vec![Vec::new(); n_particles];
    let mut pbest: Vec<Vec<usize>> = positions.clone();
    let mut pbest_cost: Vec<f32> = pbest.iter().map(|p| distances.tour_length(p)).collect();

    let (best_idx, _) = pbest_cost
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap();
    let mut gbest: Vec<usize> = pbest[best_idx].clone();
    let mut gbest_cost: f32 = pbest_cost[best_idx];

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));
    }

    let epochs = opts.epochs;
    for epoch in 0..epochs {
        #[allow(clippy::cast_precision_loss)]
        let w = W_MAX - (W_MAX - W_MIN) * (epoch as f64 / epochs.max(1) as f64);

        for i in 0..n_particles {
            let r1: f64 = rng.random();
            let r2: f64 = rng.random();

            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let inertia_keep = (w * velocities[i].len() as f64).round() as usize;

            let cog_diff = swap_sequence(&positions[i], &pbest[i]);
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let cog_keep = (C1 * r1 * cog_diff.len() as f64).round() as usize;

            let soc_diff = swap_sequence(&positions[i], &gbest);
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let soc_keep = (C2 * r2 * soc_diff.len() as f64).round() as usize;

            let mut new_vel = trim_velocity(&velocities[i], inertia_keep);
            new_vel.extend(trim_velocity(&cog_diff, cog_keep));
            new_vel.extend(trim_velocity(&soc_diff, soc_keep));
            new_vel.truncate(v_max);

            let new_pos = apply_swaps(&positions[i], &new_vel);
            positions[i] = new_pos;
            velocities[i] = new_vel;

            let cost = distances.tour_length(&positions[i]);

            if cost < pbest_cost[i] {
                pbest[i] = positions[i].clone();
                pbest_cost[i] = cost;
            }
            if cost < gbest_cost {
                gbest = positions[i].clone();
                gbest_cost = cost;
                if let Some(tx) = progress_tx {
                    let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));
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
    use crate::tsp::{distance_matrix, kdtree, HeuristicOptions, TspProblem};

    #[test]
    fn test_pso_respects_initial_tour() {
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
        let opts = HeuristicOptions { epochs: 0, ..HeuristicOptions::default() };
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
    fn test_trim_velocity_truncates_and_clamps() {
        let v: Velocity = vec![(0, 1), (1, 2), (2, 3)];
        assert_eq!(trim_velocity(&v, 2), vec![(0, 1), (1, 2)]);
        assert_eq!(trim_velocity(&v, 0), vec![]);
        assert_eq!(trim_velocity(&v, 10), v);
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
        let opts = HeuristicOptions { epochs: 20, n_nearest: 5, ..HeuristicOptions::default() };
        let problem = TspProblem::new(cities.clone(), distances);
        let sol = solve(&problem, &opts, None);
        assert_eq!(sol.route().len(), cities.len());
        assert!(sol.total > 0.0);
    }
}

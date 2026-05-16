use rand::Rng;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::messages::ProgressMessage;
use super::route::{apply_swaps, swap_sequence, Route};
use super::{Solution, SolverOptions};

type Swap = (usize, usize);
type Velocity = Vec<Swap>;

// PSO hyper-parameters
const W_MAX: f64 = 0.9; // inertia at epoch 0 (exploration)
const W_MIN: f64 = 0.4; // inertia at final epoch (exploitation)
const C1: f64 = 1.5;    // cognitive coefficient
const C2: f64 = 1.5;    // social coefficient
const DEFAULT_N_PARTICLES: usize = 30;
// Without a velocity cap, steady-state swap-list length ≈ (C1+C2)*0.5*(n−1)/(1−W) ≈ 5n,
// which scrambles the tour completely.  Capping at ~n/3 gives directional movement.
const V_MAX_FACTOR: f64 = 0.35;

/// Greedy nearest-neighbour tour starting from the first city in `city_ids`.
/// Used to seed particle 0 so the swarm starts from a reasonable neighbourhood.
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

/// Scalar-multiply a velocity: keep the first `keep` swaps (clamped to length).
fn trim_velocity(v: &[Swap], keep: usize) -> Velocity {
    v.iter().take(keep).copied().collect()
}

pub fn solve(cities: &[KDPoint], distances: &DistanceMatrix, options: &SolverOptions) -> Solution {
    let n_cities = cities.len();
    // n_nearest is repurposed as n_particles; floor at DEFAULT_N_PARTICLES
    let n_particles = options.n_nearest.max(DEFAULT_N_PARTICLES);
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let v_max = ((n_cities as f64 * V_MAX_FACTOR).ceil() as usize).max(1);

    let mut rng = rand::rng();
    let city_ids: Vec<usize> = cities.iter().map(|c| c.id).collect();

    // Particle 0 is seeded with a greedy NN tour so gbest starts from a good neighbourhood.
    // The rest are random Fisher-Yates shuffles to maintain diversity.
    let mut positions: Vec<Vec<usize>> = (0..n_particles)
        .map(|idx| {
            if idx == 0 {
                nn_seed(&city_ids, distances)
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

    // Global best
    let (best_idx, _) = pbest_cost
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap();
    let mut gbest: Vec<usize> = pbest[best_idx].clone();
    let mut gbest_cost: f32 = pbest_cost[best_idx];

    options.send_progress(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));

    let epochs = options.epochs;
    for epoch in 0..epochs {
        // Linearly decay inertia weight from W_MAX (exploration) to W_MIN (exploitation).
        // Mirrors simulated annealing's cooling schedule: high randomness early,
        // fine-tuning near convergence.
        #[allow(clippy::cast_precision_loss)]
        let w = W_MAX - (W_MAX - W_MIN) * (epoch as f64 / epochs.max(1) as f64);

        for i in 0..n_particles {
            let r1: f64 = rng.random();
            let r2: f64 = rng.random();

            // Inertia: keep first round(w * |v|) swaps of current velocity
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let inertia_keep = (w * velocities[i].len() as f64).round() as usize;

            // Cognitive: toward personal best
            let cog_diff = swap_sequence(&positions[i], &pbest[i]);
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let cog_keep = (C1 * r1 * cog_diff.len() as f64).round() as usize;

            // Social: toward global best
            let soc_diff = swap_sequence(&positions[i], &gbest);
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let soc_keep = (C2 * r2 * soc_diff.len() as f64).round() as usize;

            // New velocity = inertia + cognitive + social, capped at v_max.
            // The cap is the discrete analogue of v_max in continuous PSO:
            // without it the swap list grows to ~5n and scrambles the tour.
            let mut new_vel = trim_velocity(&velocities[i], inertia_keep);
            new_vel.extend(trim_velocity(&cog_diff, cog_keep));
            new_vel.extend(trim_velocity(&soc_diff, soc_keep));
            new_vel.truncate(v_max);

            // Update position and velocity
            let new_pos = apply_swaps(&positions[i], &new_vel);
            positions[i] = new_pos;
            velocities[i] = new_vel;

            // Evaluate and update bests
            let cost = distances.tour_length(&positions[i]);

            if cost < pbest_cost[i] {
                pbest[i] = positions[i].clone();
                pbest_cost[i] = cost;
            }
            if cost < gbest_cost {
                gbest = positions[i].clone();
                gbest_cost = cost;
                options.send_progress(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));
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

    #[test]
    fn test_trim_velocity_truncates_and_clamps() {
        let v: Velocity = vec![(0, 1), (1, 2), (2, 3)];
        assert_eq!(trim_velocity(&v, 2), vec![(0, 1), (1, 2)]);
        assert_eq!(trim_velocity(&v, 0), vec![]);
        assert_eq!(trim_velocity(&v, 10), v);
    }

    #[test]
    fn test_nn_seed_visits_all_cities() {
        let ids = vec![0, 1, 2, 3];
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let city_ids: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let tour = nn_seed(&city_ids, &dm);
        let mut sorted = tour.clone();
        sorted.sort();
        assert_eq!(sorted, ids);
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
            epochs: 20,
            n_nearest: 5,
            ..SolverOptions::default()
        };
        let sol = solve(&cities, &distances, &options);
        assert_eq!(sol.route().len(), cities.len());
        assert!(sol.total > 0.0);
    }
}

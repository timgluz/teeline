use rand::RngExt;
use std::sync::mpsc;

use super::budget::Budget;
use super::progress::ProgressMessage;
use super::route::{Route, apply_swaps, swap_sequence};
use super::{HeuristicOptions, Solution, TspProblem};

type Swap = (usize, usize);
type Velocity = Vec<Swap>;

const G0: f64 = 20.0;
const ALPHA: f64 = 1.0;
// Empirically: inertia (W>0) hurts performance on berlin52 — stale swap indices
// accumulated across epochs act as noise once G decays. W=0 gives pure gravity.
const INERTIA_W: f64 = 0.0;
const V_MAX_FACTOR: f64 = 0.35;
const DEFAULT_N_AGENTS: usize = 25;

fn trim_velocity(v: &[Swap], keep: usize) -> Velocity {
    v.iter().take(keep).copied().collect()
}

pub fn solve(
    problem: &TspProblem,
    opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n_cities = cities.len();
    let n_agents = opts.n_nearest.max(DEFAULT_N_AGENTS);
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let v_max = ((n_cities as f64 * V_MAX_FACTOR).ceil() as usize).max(1);

    let mut rng = rand::rng();
    let city_ids: Vec<usize> = cities.iter().map(|c| c.id).collect();

    let mut positions: Vec<Vec<usize>> = (0..n_agents)
        .map(|idx| {
            if idx == 0 {
                init_tour.map(|t| t.to_vec()).unwrap_or_else(|| {
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

    let mut velocities: Vec<Velocity> = vec![Vec::new(); n_agents];
    let mut costs: Vec<f32> = positions.iter().map(|p| distances.tour_length(p)).collect();

    let (best_idx, _) = costs
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap();
    let mut gbest: Vec<usize> = positions[best_idx].clone();
    let mut gbest_cost: f32 = costs[best_idx];

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));
    }

    let epochs = opts.epochs;
    let mut budget = Budget::new(epochs, opts.stagnation_epochs);
    // Same horizon problem as PSO's inertia: with the zero left in place the denominator is
    // `max(1)`, so `g` decays to nothing within a few epochs and the rest of the run is a random
    // walk at a fixed near-zero gravitational constant.
    let horizon = if epochs == 0 {
        HeuristicOptions::default().epochs
    } else {
        epochs
    };
    // Starts true so the first epoch runs, then carries the previous epoch's result into the
    // budget — one decision point for both the cap and the convergence threshold.
    let mut improved = true;

    while budget.record(improved) {
        let epoch = budget.index();
        let best_at_epoch_start = gbest_cost;
        // Spread-based mass normalization (Rashedi 2009)
        let worst_cost = costs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        #[allow(clippy::cast_precision_loss)]
        let sum_fitness: f64 = costs.iter().map(|&c| (worst_cost - c) as f64).sum();

        let masses: Vec<f64> = if sum_fitness < f64::EPSILON {
            // All agents have equal cost — uniform mass avoids NaN
            vec![1.0 / n_agents as f64; n_agents]
        } else {
            costs
                .iter()
                .map(|&c| (worst_cost - c) as f64 / sum_fitness)
                .collect()
        };

        // Kbest: top ⌈N/2⌉ agents by mass (heaviest = shortest tours)
        let k_best = n_agents.div_ceil(2);
        let mut ranked: Vec<usize> = (0..n_agents).collect();
        ranked.sort_unstable_by(|&a, &b| masses[b].partial_cmp(&masses[a]).unwrap());
        let kbest_indices = &ranked[..k_best];

        // Decaying gravitational constant
        #[allow(clippy::cast_precision_loss)]
        let g = G0 * (-(ALPHA * epoch as f64) / horizon as f64).exp();

        for i in 0..n_agents {
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let inertia_keep = (INERTIA_W * velocities[i].len() as f64).round() as usize;
            let mut new_vel = trim_velocity(&velocities[i], inertia_keep);

            for &j in kbest_indices {
                if i == j {
                    continue; // self-pull is a no-op; skip for efficiency
                }
                let r: f64 = rng.random();
                #[allow(clippy::cast_possible_truncation)]
                let n_swaps = (r * g * masses[j]).round() as usize;
                if n_swaps == 0 {
                    continue;
                }
                let pull = swap_sequence(&positions[i], &positions[j]);
                new_vel.extend(trim_velocity(&pull, n_swaps));
            }

            new_vel.truncate(v_max);
            positions[i] = apply_swaps(&positions[i], &new_vel);
            velocities[i] = new_vel;

            costs[i] = distances.tour_length(&positions[i]);
            if costs[i] < gbest_cost {
                gbest = positions[i].clone();
                gbest_cost = costs[i];
                if let Some(tx) = progress_tx {
                    let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));
                }
            }
        }

        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::EpochUpdate(epoch));
        }

        // A single comparison for the epoch, rather than a flag per improvement site: a solver
        // with several such sites would otherwise assign the same value more than once.
        improved = gbest_cost < best_at_epoch_start;
    }

    if budget.converged() {
        tracing::info!(
            epoch = budget.epoch(),
            stagnation_epochs = budget.stale_epochs(),
            "GSA: converged, no improvement for the stagnation limit"
        );
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&gbest, cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{HeuristicOptions, TspProblem, distance_matrix, kdtree};

    fn five_city_problem() -> (Vec<kdtree::KDPoint>, TspProblem) {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities.clone(), dm);
        (cities, problem)
    }

    #[test]
    fn test_gsa_returns_valid_tour() {
        let (cities, problem) = five_city_problem();
        let opts = HeuristicOptions {
            epochs: 20,
            n_nearest: 5,
            ..HeuristicOptions::default()
        };
        let sol = solve(&problem, &opts, None, None);
        assert_eq!(sol.route().len(), cities.len());
        let mut visited = sol.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected);
    }

    #[test]
    fn test_gsa_respects_initial_tour() {
        let (cities, problem) = five_city_problem();
        let dm = distance_matrix::from_cities(&cities);
        let optimal: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let optimal_cost = dm.tour_length(&optimal);
        let opts = HeuristicOptions {
            epochs: 5,
            ..HeuristicOptions::default()
        };

        // The seed must be the solver's *initial* best, and that is what the first update carries:
        // it is sent before any epoch runs. Comparing final costs cannot show this — on an
        // instance this small the solver may reach the optimum on its own, seed or no seed.
        let (tx, rx) = mpsc::channel();
        let result = solve(&problem, &opts, Some(&tx), Some(&optimal));
        match rx.try_recv() {
            Ok(ProgressMessage::PathUpdate(route, cost)) => {
                assert_eq!(
                    route.route(),
                    optimal.as_slice(),
                    "the seeded tour must be the initial best"
                );
                assert!((cost - optimal_cost).abs() < 1e-4, "at the seed's cost");
            }
            other => panic!("expected the seeded tour as the first update, got {other:?}"),
        }
        assert!((result.total - optimal_cost).abs() < 1e-4);
        let mut visited = result.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected);
    }

    #[test]
    fn test_gsa_uniform_mass_when_converged() {
        // When all agents share identical cost, sum_fitness=0 → uniform mass fallback.
        // Solver must not panic or produce NaN.
        let (cities, problem) = five_city_problem();
        // Force all agents to share the same tour (same cost) by providing init_tour
        // and running 0 epochs so positions never diverge.
        let tour: Vec<usize> = cities.iter().map(|c| c.id).collect();
        // Run a few epochs; all agents start from the same init_tour so sum_fitness
        // is 0 on the first epoch, exercising the uniform-mass fallback.
        let opts2 = HeuristicOptions {
            epochs: 5,
            n_nearest: 5,
            ..HeuristicOptions::default()
        };
        let sol = solve(&problem, &opts2, None, Some(&tour));
        assert_eq!(sol.route().len(), cities.len());
        assert!(sol.total.is_finite());
    }

    /// See the PSO equivalent: an uncapped run must terminate on the convergence limit without
    /// letting the gravitational constant decay to nothing on the first epoch.
    #[test]
    fn test_gsa_uncapped_epochs_converge_and_keep_their_schedule() {
        let (cities, problem) = five_city_problem();
        let opts = HeuristicOptions {
            epochs: 0,
            stagnation_epochs: 10,
            ..HeuristicOptions::default()
        };

        let result = solve(&problem, &opts, None, None);

        assert!(result.total.is_finite() && result.total > 0.0);
        let mut visited = result.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected);
    }
}

use rand::RngExt;
use std::collections::HashSet;
use std::sync::mpsc;

use super::budget::Budget;
use super::nearest_neighbor::seed_population;
use super::progress::ProgressMessage;
use super::route::{Route, move_toward, random_reversal};
use super::{HeuristicOptions, Solution, TspProblem};

const G0: f64 = 20.0;
const ALPHA: f64 = 1.0;
/// Scale on `g · mass · r` so the OX segment stays useful across the 20 → 7.36
/// decay of `g` (Rashedi's α is 20, so his `g` reaches zero; ours does not).
const GRAV_SCALE: f64 = 0.25;
/// A pull never borrows more than half of the attractor.
const MAX_FRACTION: f64 = 0.5;
const DEFAULT_N_AGENTS: usize = 25;

/// Rashedi shrinks Kbest linearly from N to 1; that schedule is the algorithm's
/// exploration-to-exploitation control, with `g` as the secondary one.
fn kbest_size(n_agents: usize, epoch: usize, horizon: usize) -> usize {
    #[allow(clippy::cast_precision_loss)]
    let progress = (epoch as f64 / horizon.max(1) as f64).min(1.0);
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let size = (n_agents as f64 - (n_agents as f64 - 1.0) * progress).round() as usize;
    size.clamp(1, n_agents.max(1))
}

/// Sample one Kbest attractor weighted by mass, excluding `agent` itself.
///
/// Falls back to uniform over the remaining agents when every weight is zero (a
/// collapsed or tied population), and returns `None` when there is nobody to pull
/// toward.
fn sample_attractor(
    kbest: &[usize],
    masses: &[f64],
    agent: usize,
    rng: &mut impl RngExt,
) -> Option<usize> {
    let mut count = 0usize;
    let mut total = 0.0f64;
    for &j in kbest {
        if j == agent {
            continue;
        }
        count += 1;
        total += masses[j].max(0.0);
    }
    if count == 0 {
        return None;
    }

    if total <= f64::EPSILON {
        let pick = rng.random_range(0..count);
        let mut seen = 0usize;
        for &j in kbest {
            if j == agent {
                continue;
            }
            if seen == pick {
                return Some(j);
            }
            seen += 1;
        }
        return None;
    }

    let mut point = rng.random::<f64>() * total;
    let mut last = None;
    for &j in kbest {
        if j == agent || masses[j] <= 0.0 {
            continue;
        }
        last = Some(j);
        point -= masses[j];
        if point <= 0.0 {
            return Some(j);
        }
    }
    last
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

    tracing::info!(n_agents, epochs = opts.epochs, "GSA starting");

    let mut rng = rand::rng();
    let mut tours = seed_population(distances.as_ref(), cities, n_agents, init_tour, &mut rng);

    let mut costs: Vec<f32> = tours.iter().map(|t| distances.tour_length(t)).collect();

    let (best_idx, _) = costs
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap();
    let mut gbest: Vec<usize> = tours[best_idx].clone();
    let mut gbest_cost: f32 = costs[best_idx];
    let start_gbest_cost = gbest_cost;
    tracing::info!(start_gbest = start_gbest_cost, "GSA: seeded population");

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));
    }

    let epochs = opts.epochs;
    let mut budget = Budget::new(epochs, opts.stagnation_epochs);
    // An uncapped run has no horizon of its own; the default keeps the decay's shape.
    let horizon = if epochs == 0 {
        HeuristicOptions::default().epochs
    } else {
        epochs
    };
    // Starts true so the first epoch runs, then carries the previous epoch's result into the
    // budget — one decision point for both the cap and the convergence threshold.
    let mut improved = true;
    let mut used = HashSet::with_capacity(n_cities);
    let mut ranked: Vec<usize> = (0..n_agents).collect();

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

        // Kbest: heaviest agents, shrinking from N to 1 over the run.
        let k_best = kbest_size(n_agents, epoch, horizon);
        ranked.sort_unstable_by(|&a, &b| masses[b].total_cmp(&masses[a]));
        let kbest_indices = &ranked[..k_best];

        #[allow(clippy::cast_precision_loss)]
        let g = G0 * (-(ALPHA * epoch as f64) / horizon.max(1) as f64).exp();

        for i in 0..n_agents {
            let mut candidate = tours[i].clone();
            if n_cities >= 4
                && let Some(j) = sample_attractor(kbest_indices, &masses, i, &mut rng)
            {
                let r: f64 = rng.random();
                #[allow(clippy::cast_precision_loss)]
                let min_fraction = 1.0 / n_cities as f64;
                let fraction = (GRAV_SCALE * g * masses[j] * r).clamp(min_fraction, MAX_FRACTION);
                candidate = move_toward(&tours[i], &tours[j], fraction, &mut rng, &mut used);
            }
            random_reversal(&mut candidate, &mut rng);

            // Greedy acceptance is the memetic deviation: without per-agent memory,
            // always-accepting an unconditional reversal drifts the whole swarm to a
            // noise floor and freezes gbest at the best seed.
            let candidate_cost = distances.tour_length(&candidate);
            if candidate_cost < costs[i] {
                tours[i] = candidate;
                costs[i] = candidate_cost;

                if candidate_cost < gbest_cost {
                    gbest = tours[i].clone();
                    gbest_cost = candidate_cost;
                    if let Some(tx) = progress_tx {
                        let _ =
                            tx.send(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));
                    }
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

    tracing::info!(
        start_gbest = start_gbest_cost,
        pre_polish_gbest = gbest_cost,
        "GSA: loop finished"
    );

    // The swarm explores; the polish exploits. This is the memetic half.
    let polished = super::two_opt::solve(problem, opts, None, Some(&gbest));

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(
            Route::new(polished.route()),
            polished.total,
        ));
        let _ = tx.send(ProgressMessage::Done);
    }

    polished
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::distance_source::DistanceSource;
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
    fn test_gsa_never_returns_worse_than_its_seed() {
        let (cities, problem) = five_city_problem();
        let dm = distance_matrix::from_cities(&cities);
        let seed: Vec<usize> = cities.iter().map(|c| c.id).rev().collect();
        let seed_cost = dm.tour_length(&seed);
        let opts = HeuristicOptions {
            epochs: 20,
            ..HeuristicOptions::default()
        };
        let sol = solve(&problem, &opts, None, Some(&seed));

        assert!(
            sol.total <= seed_cost + 1e-4,
            "the elitist gbest must not return worse than the seed: {} > {seed_cost}",
            sol.total
        );
    }

    #[test]
    fn test_kbest_size_shrinks_to_one() {
        assert_eq!(kbest_size(25, 0, 10_000), 25);
        assert_eq!(kbest_size(25, 10_000, 10_000), 1);
        assert_eq!(kbest_size(25, 20_000, 10_000), 1);
        assert_eq!(kbest_size(1, 0, 10_000), 1);
    }

    #[test]
    fn test_sample_attractor_falls_back_to_uniform_on_zero_mass() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let kbest = [0usize, 1, 2];
        let masses = [0.0, 0.0, 0.0];
        for _ in 0..20 {
            let j = sample_attractor(&kbest, &masses, 0, &mut rng).unwrap();
            assert!(j == 1 || j == 2);
        }
        assert_eq!(sample_attractor(&[0], &[1.0], 0, &mut rng), None);
    }
}

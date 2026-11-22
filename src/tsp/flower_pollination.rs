use rand::RngExt;
use std::collections::HashSet;
use std::sync::mpsc;

use super::budget::Budget;
use super::nearest_neighbor::seed_population;
use super::probability::{bernoulli, levy_step};
use super::progress::ProgressMessage;
use super::route::{Route, move_toward, random_reversal};
use super::{FPAOptions, Solution, TspProblem};

const DEFAULT_N_FLOWERS: usize = 25;
/// Yang's gamma on the global step. The raw Lévy median (~0.6) would otherwise saturate
/// the segment cap on most draws, turning global pollination into a fixed half-best
/// copy and collapsing the population within a few hundred epochs.
const LEVY_GAMMA: f64 = 0.1;
/// A global move never borrows more than half of the best tour.
const MAX_GLOBAL_FRACTION: f64 = 0.5;
/// Local pollination borrows at most 30% of a neighbour before the reversal.
const LOCAL_EPS_SCALE: f64 = 0.3;

fn global_pollination(
    flower: &[usize],
    gbest: &[usize],
    rng: &mut impl RngExt,
    used: &mut HashSet<usize>,
) -> Vec<usize> {
    let levy = levy_step(rng).abs();
    #[allow(clippy::cast_precision_loss)]
    let min_fraction = 1.0 / flower.len().max(1) as f64;
    let fraction = (LEVY_GAMMA * levy).clamp(min_fraction, MAX_GLOBAL_FRACTION);
    move_toward(flower, gbest, fraction, rng, used)
}

fn local_pollination(
    flower: &[usize],
    flowers: &[Vec<usize>],
    flower_idx: usize,
    rng: &mut impl RngExt,
    used: &mut HashSet<usize>,
) -> Vec<usize> {
    let n_flowers = flowers.len();
    if n_flowers < 2 {
        return flower.to_vec();
    }
    // A uniformly sampled offset keeps the neighbour distinct without a rejection loop.
    let j = (flower_idx + 1 + rng.random_range(0..n_flowers - 1)) % n_flowers;
    let epsilon: f64 = rng.random();

    let mut candidate = move_toward(flower, &flowers[j], epsilon * LOCAL_EPS_SCALE, rng, used);
    // Always reverse, even when the crossover reproduced the current tour: once the
    // population collapses, the reversal is the only move still exploring.
    random_reversal(&mut candidate, rng);
    candidate
}

pub fn solve(
    problem: &TspProblem,
    opts: &FPAOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n_cities = cities.len();
    let n_flowers = opts.heuristic.n_nearest.max(DEFAULT_N_FLOWERS);
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
    let mut flowers = seed_population(distances.as_ref(), cities, n_flowers, init_tour, &mut rng);

    let mut costs: Vec<f32> = flowers.iter().map(|t| distances.tour_length(t)).collect();

    let (best_idx, _) = costs
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    let mut gbest: Vec<usize> = flowers[best_idx].clone();
    let mut gbest_cost: f32 = costs[best_idx];
    let start_gbest_cost = gbest_cost;
    tracing::info!(start_gbest = start_gbest_cost, "FPA: seeded population");

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));
    }

    let mut budget = Budget::new(opts.heuristic.epochs, opts.heuristic.stagnation_epochs);
    // Starts true so the first epoch runs, then carries the previous epoch's result into the
    // budget — one decision point for both the cap and the convergence threshold.
    let mut improved = true;
    let mut used = HashSet::with_capacity(n_cities);

    while budget.record(improved) {
        let epoch = budget.index();
        let best_at_epoch_start = gbest_cost;
        for i in 0..n_flowers {
            let new_x = if bernoulli(&mut rng, switch_prob) {
                global_pollination(&flowers[i], &gbest, &mut rng, &mut used)
            } else {
                local_pollination(&flowers[i], &flowers, i, &mut rng, &mut used)
            };

            if new_x == flowers[i] {
                continue;
            }

            let new_cost = distances.tour_length(&new_x);
            if new_cost < costs[i] {
                flowers[i] = new_x;
                costs[i] = new_cost;

                if new_cost < gbest_cost {
                    gbest = flowers[i].clone();
                    gbest_cost = new_cost;
                    if let Some(tx) = progress_tx {
                        let _ =
                            tx.send(ProgressMessage::PathUpdate(Route::new(&gbest), gbest_cost));
                    }
                    tracing::info!(epoch, tour_length = gbest_cost, "FPA: new best");
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
            "FPA: converged, no improvement for the stagnation limit"
        );
    }

    tracing::info!(
        start_gbest = start_gbest_cost,
        pre_polish_gbest = gbest_cost,
        "FPA: loop finished"
    );

    // The swarm explores; the polish exploits. This is the memetic half, and it is what
    // turns the population's best tour into a locally optimal one.
    let polished = super::two_opt::solve(problem, &opts.heuristic, None, Some(&gbest));

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
    use std::sync::mpsc;

    use crate::tsp::progress::ProgressMessage;
    use crate::tsp::{FPAOptions, HeuristicOptions, TspProblem, distance_matrix, kdtree};

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
            heuristic: HeuristicOptions {
                epochs: 5,
                ..HeuristicOptions::default()
            },
            ..FPAOptions::default()
        };
        let problem = TspProblem::new(cities.clone(), dm);

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
    fn test_fpa_never_returns_worse_than_its_seed() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let seed: Vec<usize> = vec![0, 2, 1, 3];
        let seed_cost = dm.tour_length(&seed);
        let opts = FPAOptions {
            heuristic: HeuristicOptions {
                epochs: 20,
                ..HeuristicOptions::default()
            },
            mutation_probability: 0.8,
        };
        let problem = TspProblem::new(cities, dm);
        let sol = solve(&problem, &opts, None, Some(&seed));

        assert!(
            sol.total <= seed_cost + 1e-4,
            "the elitist gbest must not return worse than the seed: {} > {seed_cost}",
            sol.total
        );
    }

    fn four_city_setup() -> TspProblem {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        TspProblem::new(cities, dm)
    }

    #[test]
    fn test_solve_returns_valid_tour() {
        let problem = four_city_setup();
        let cities = problem.cities.clone();
        let opts = FPAOptions {
            heuristic: HeuristicOptions {
                epochs: 30,
                n_nearest: 5,
                ..HeuristicOptions::default()
            },
            mutation_probability: 0.8,
        };
        let sol = solve(&problem, &opts, None, None);
        let mut visited = sol.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(
            visited, expected,
            "FPA tour does not visit all cities exactly once"
        );
        assert!(sol.total > 0.0);
        assert!(sol.total.is_finite());
    }
}

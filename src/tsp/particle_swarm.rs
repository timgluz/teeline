use rand::RngExt;
use std::collections::HashSet;
use std::sync::mpsc;

use super::budget::Budget;
use super::candidate_list::{CandidateList, MIN_CANDIDATE_K};
use super::nearest_neighbor::randomized_tour;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem};

const W_MAX: f64 = 0.9;
const W_MIN: f64 = 0.4;
/// Segment fraction of the cognitive move (toward the particle's own best).
const C1: f64 = 0.5;
/// Segment fraction of the social move (toward the swarm's best).
const C2: f64 = 0.5;
const DEFAULT_N_PARTICLES: usize = 30;

fn random_tour(city_ids: &[usize], rng: &mut impl RngExt) -> Vec<usize> {
    let mut tour = city_ids.to_vec();
    for i in (1..tour.len()).rev() {
        let j = rng.random_range(0..=i);
        tour.swap(i, j);
    }
    tour
}

/// Order-crossover move toward `target`: copy a random contiguous segment of `target`
/// (wrapping around the ends) and fill the rest in `cur`'s relative order.
///
/// A transposition can break four edges at once and is blind to tour rotation; this move
/// keeps the target's edges inside the copied segment and breaks only its junctions.
fn move_toward(
    cur: &[usize],
    target: &[usize],
    fraction: f64,
    rng: &mut impl RngExt,
    used: &mut HashSet<usize>,
) -> Vec<usize> {
    let n = cur.len();
    if n < 4 {
        return target.to_vec();
    }
    let seg_len = ((fraction * n as f64).round() as usize).clamp(1, n - 1);
    let start = rng.random_range(0..n);

    let mut child = vec![usize::MAX; n];
    used.clear();
    for k in 0..seg_len {
        let idx = (start + k) % n;
        child[idx] = target[idx];
        used.insert(target[idx]);
    }

    let mut fill = (start + seg_len) % n;
    for k in 0..n {
        let city = cur[(start + seg_len + k) % n];
        if !used.contains(&city) {
            child[fill] = city;
            fill = (fill + 1) % n;
        }
    }

    child
}

/// Explore with one or two random 2-opt reversals; `w` decays from `W_MAX` to `W_MIN`.
fn perturb(tour: &mut [usize], w: f64, rng: &mut impl RngExt) {
    let n = tour.len();
    if n < 3 {
        return;
    }
    let moves = if w > 0.6 { 2 } else { 1 };
    for _ in 0..moves {
        let a = rng.random_range(0..n);
        let b = rng.random_range(0..n);
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        if hi - lo > 1 {
            tour[lo..=hi].reverse();
        }
    }
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
    let n_particles = opts.n_nearest.max(DEFAULT_N_PARTICLES);

    let mut rng = rand::rng();
    let city_ids: Vec<usize> = cities.iter().map(|c| c.id).collect();

    // One candidate list for all constructive seeds. Coordinate metrics rank by the
    // KD-tree; EXPLICIT inputs rank by the exact distance source, because their layout is
    // a display aid (or a derived embedding), not the objective.
    let candidates = if distances.is_coordinate_metric() {
        CandidateList::build(cities, MIN_CANDIDATE_K)
    } else {
        CandidateList::build_from_distances(distances.as_ref(), cities, MIN_CANDIDATE_K)
    }
    .ok();

    let mut tours: Vec<Vec<usize>> = Vec::with_capacity(n_particles);
    for idx in 0..n_particles {
        let tour = if idx == 0 {
            init_tour
                .map(|t| t.to_vec())
                .unwrap_or_else(|| random_tour(&city_ids, &mut rng))
        } else if idx < n_particles / 2 {
            // Half the swarm starts from diverse nearest-neighbour tours; the rest stays
            // random so the population does not collapse onto one basin.
            match &candidates {
                Some(cands) if n_cities >= 4 => {
                    let start = rng.random_range(0..n_cities);
                    randomized_tour(distances.as_ref(), cands, start, 2, &mut rng)
                        .into_iter()
                        .map(|pos| cities[pos].id)
                        .collect()
                }
                _ => random_tour(&city_ids, &mut rng),
            }
        } else {
            random_tour(&city_ids, &mut rng)
        };
        tours.push(tour);
    }

    let mut pbest: Vec<Vec<usize>> = tours.clone();
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
    let mut budget = Budget::new(epochs, opts.stagnation_epochs);
    // The inertia is interpolated across a horizon, and an uncapped run has none of its own.
    // Leaving the zero in place makes the denominator `max(1)`, so `w` collapses immediately
    // and the exploration term degenerates. The default horizon keeps the anneal's shape and
    // only drops the cap.
    let horizon = if epochs == 0 {
        HeuristicOptions::default().epochs
    } else {
        epochs
    };
    // Starts true so the first epoch runs, then carries the previous epoch's result into the
    // budget — one decision point for both the cap and the convergence threshold.
    let mut improved = true;
    // Reused by every move; the set's capacity survives across epochs, so the hot loop does
    // not allocate one per crossover.
    let mut used = HashSet::with_capacity(MIN_CANDIDATE_K.max(8));

    while budget.record(improved) {
        let epoch = budget.index();
        let best_at_epoch_start = gbest_cost;
        #[allow(clippy::cast_precision_loss)]
        let w = W_MAX - (W_MAX - W_MIN) * (epoch as f64 / horizon as f64);

        for i in 0..n_particles {
            let r1: f64 = rng.random();
            let r2: f64 = rng.random();

            // Cognitive, then social. Each move is recomputed from the current tour: a swap
            // list computed against the pre-cognitive tour would be applied at ranks the
            // first move had already shifted.
            let cog = move_toward(&tours[i], &pbest[i], C1 * r1, &mut rng, &mut used);
            let mut next = move_toward(&cog, &gbest, C2 * r2, &mut rng, &mut used);
            perturb(&mut next, w, &mut rng);
            tours[i] = next;

            let cost = distances.tour_length(&tours[i]);

            if cost < pbest_cost[i] {
                pbest[i] = tours[i].clone();
                pbest_cost[i] = cost;
            }
            if cost < gbest_cost {
                gbest = tours[i].clone();
                gbest_cost = cost;
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
            "PSO: converged, no improvement for the stagnation limit"
        );
    }

    // The swarm gets close, then a full 2-opt pass turns the best tour into a locally
    // optimal one. This is the memetic half: the PSO explores, the polish exploits.
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
    use std::sync::mpsc;

    use crate::tsp::progress::ProgressMessage;
    use crate::tsp::{HeuristicOptions, TspProblem, distance_matrix, kdtree};
    use rand::SeedableRng;

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
        let opts = HeuristicOptions {
            epochs: 5,
            ..HeuristicOptions::default()
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
    fn test_move_toward_returns_a_permutation() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let cur = vec![0, 1, 2, 3, 4, 5];
        let target = vec![3, 4, 5, 0, 1, 2];
        let mut used = HashSet::new();
        let child = move_toward(&cur, &target, 0.5, &mut rng, &mut used);

        let mut sorted = child.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_pso_never_returns_worse_than_its_seed() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let seed: Vec<usize> = vec![0, 2, 1, 3];
        let seed_cost = dm.tour_length(&seed);
        let opts = HeuristicOptions {
            epochs: 20,
            ..HeuristicOptions::default()
        };
        let problem = TspProblem::new(cities, dm);
        let sol = solve(&problem, &opts, None, Some(&seed));

        assert!(
            sol.total <= seed_cost + 1e-4,
            "the elitist gbest must not return worse than the seed: {} > {seed_cost}",
            sol.total
        );
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
        let opts = HeuristicOptions {
            epochs: 20,
            n_nearest: 5,
            ..HeuristicOptions::default()
        };
        let problem = TspProblem::new(cities.clone(), distances);
        let sol = solve(&problem, &opts, None, None);
        assert_eq!(sol.route().len(), cities.len());
        assert!(sol.total > 0.0);
    }
}

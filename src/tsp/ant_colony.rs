use std::sync::mpsc;

use rand::RngExt;

use super::progress::ProgressMessage;
use super::route::Route;
use super::{AcoOptions, Solution, TspProblem};

/// How far below tau0 the evaporation floor sits. Without a floor, an un-deposited edge
/// decays to exact f32 0.0 within a few hundred epochs at typical parameters, making it
/// permanently unreachable regardless of heuristic distance — a real bug, not a
/// theoretical edge case. This is a one-line, MMAS-influenced deviation from pure
/// textbook Ant System.
const TAU_MIN_RATIO: f32 = 1e-4;

/// Minimum distance used when computing the heuristic desirability term `1/distance`,
/// avoiding division by zero for coincident cities. Deliberately not `f32::EPSILON`
/// (~1.19e-7, the ULP at magnitude 1.0) — this codebase has two prior-bug comments
/// (kdtree.rs, bellman_karp.rs) about that exact tolerance being wrong away from 1.0.
const MIN_DIST: f32 = 1e-6;

/// Evaporates every pheromone value by `rate`, then clamps below `tau_min` back up to it.
fn evaporate_and_floor(pheromone: &mut [f32], rate: f32, tau_min: f32) {
    for t in pheromone.iter_mut() {
        *t *= 1.0 - rate;
        if *t < tau_min {
            *t = tau_min;
        }
    }
}

/// Deposits `amount` symmetrically on edge (u, v). Teeline only solves symmetric TSP, so
/// pheromone mirrors that symmetry (both `pheromone[u*n+v]` and `pheromone[v*n+u]` are written).
fn deposit_edge(pheromone: &mut [f32], n: usize, u: usize, v: usize, amount: f32) {
    pheromone[u * n + v] += amount;
    pheromone[v * n + u] += amount;
}

/// Deposits `1 / cost` along every edge of a closed tour given in position-space
/// (wraps last -> first). No-op for degenerate tours/costs.
fn deposit_tour(pheromone: &mut [f32], n: usize, tour_pos: &[usize], cost: f32) {
    if tour_pos.len() < 2 || cost <= 0.0 {
        return;
    }
    let amount = 1.0 / cost;
    let last = *tour_pos.last().expect("checked len >= 2 above");
    deposit_edge(pheromone, n, last, tour_pos[0], amount);
    for w in tour_pos.windows(2) {
        deposit_edge(pheromone, n, w[0], w[1], amount);
    }
}

/// Roulette-wheel selection over `weights`, given `sum` (the caller-provided total, which
/// may differ slightly from the true sum due to floating-point rounding over 50+ terms)
/// and `r` drawn uniformly from `[0, 1)`. If the accumulation loop exhausts without
/// crossing `r * sum` — a real, reachable path given f32 rounding, not a theoretical one —
/// falls back to the last candidate rather than panicking.
fn roulette_select(weights: &[(usize, f32)], sum: f32, r: f32) -> usize {
    let target = r * sum;
    let mut acc = 0.0f32;
    for &(pos, w) in weights {
        acc += w;
        if acc >= target {
            return pos;
        }
    }
    weights.last().expect("weights must be non-empty").0
}

/// Selects the next city with graceful degradation for numerically degenerate weight sets.
/// `primary` = (position, pheromone^alpha * eta^beta) pairs for the candidate set; `fallback`
/// = (position, eta^beta) pairs for the same candidates. If `primary`'s sum is non-positive
/// or non-finite (e.g. all products underflowed, or beta pushed a term to `inf`), falls back
/// to eta-only proportional selection — discarding distance information entirely (uniform
/// random) would be strictly worse than degrading toward greedy-by-distance behavior. If even
/// `fallback` is degenerate (fully pathological — every remaining distance non-finite), returns
/// the first fallback candidate as a last resort so callers never panic or stall.
/// `r1`/`r2` are independent uniform samples in `[0, 1)` for the primary/fallback rolls,
/// passed in rather than an RNG so this function stays pure and unit-testable.
fn select_next(primary: &[(usize, f32)], fallback: &[(usize, f32)], r1: f32, r2: f32) -> usize {
    let primary_sum: f32 = primary.iter().map(|&(_, w)| w).sum();
    if primary_sum > 0.0 && primary_sum.is_finite() {
        return roulette_select(primary, primary_sum, r1);
    }
    let fallback_sum: f32 = fallback.iter().map(|&(_, w)| w).sum();
    if fallback_sum > 0.0 && fallback_sum.is_finite() {
        return roulette_select(fallback, fallback_sum, r2);
    }
    fallback
        .first()
        .map(|&(pos, _)| pos)
        .unwrap_or_else(|| primary.first().expect("candidate set must be non-empty").0)
}

/// Ant System (Dorigo 1996): a colony of stateless ants probabilistically construct tours
/// biased by a shared pheromone matrix and heuristic desirability (1/distance), then all
/// ants deposit pheromone proportional to `1/tour_cost` along their edges after a uniform
/// evaporation pass. Unlike Elitist AS or Ant Colony System, every ant deposits every epoch
/// (not just the iteration-best), and there is no local pheromone decay during construction.
///
/// Ants carry no memory between epochs — only the pheromone matrix persists — so `init_tour`
/// is not replayed as a permanent "ant 0" the way CS/FPA's mutated population arrays do.
/// Instead it seeds the incumbent best/best_cost and biases the initial pheromone level
/// (tau0) and epoch-1 construction via one extra deposit pass.
pub fn solve(
    problem: &TspProblem,
    opts: &AcoOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n = cities.len();

    tracing::info!(cities = n, num_ants = opts.num_ants, "ACO starting");

    if n <= 2 {
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::Done);
        }
        return Solution::from_parts(&path, cities, distances);
    }

    let mut rng = rand::rng();

    let to_pos = |id: usize| -> usize {
        distances
            .city_id2pos(id)
            .expect("init_tour city id must exist in this problem")
    };
    let to_id = |pos: usize| -> usize {
        distances
            .pos2city_id(pos)
            .expect("position must map to a known city id")
    };

    let best_pos_seed: Vec<usize> = match init_tour {
        Some(t) => t.iter().map(|&id| to_pos(id)).collect(),
        None => {
            let mut positions: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                let j = rng.random_range(0..=i);
                positions.swap(i, j);
            }
            positions
        }
    };
    let mut best_cost = distances.tour_length_by_pos(&best_pos_seed);
    let mut best: Vec<usize> = best_pos_seed.clone();

    if let Some(tx) = progress_tx {
        let best_ids: Vec<usize> = best.iter().map(|&p| to_id(p)).collect();
        let _ = tx.send(ProgressMessage::PathUpdate(
            Route::new(&best_ids),
            best_cost,
        ));
    }

    let tau0 = if init_tour.is_some() && best_cost > 0.0 {
        opts.num_ants as f32 / best_cost
    } else {
        tracing::warn!(
            "ACO: no usable init_tour, using flat tau0=1.0 — pheromone differentiation may take longer to emerge"
        );
        1.0
    };
    let tau_min = tau0 * TAU_MIN_RATIO;

    let mut pheromone: Vec<f32> = vec![tau0; n * n];

    // Static per run since beta is fixed — precomputed once, not per ant-step.
    // Diagonal (self-loops) left at 0.0; they're never valid transitions.
    let mut eta_beta = vec![0.0f32; n * n];
    for u in 0..n {
        for v in 0..n {
            if u == v {
                continue;
            }
            let dist = distances.distance_by_pos(u, v).unwrap_or(0.0).max(MIN_DIST);
            eta_beta[u * n + v] = (1.0 / dist).powf(opts.beta);
        }
    }

    if init_tour.is_some() {
        deposit_tour(&mut pheromone, n, &best_pos_seed, best_cost);
    }

    for epoch in 0..opts.heuristic.epochs {
        // Precomputed once per epoch, not once per ant-step: with num_ants ants each
        // scanning up to n unvisited cities per step, per-step powf would cost roughly
        // num_ants * n^2 / 2 evaluations vs n^2 here.
        let tau_alpha: Vec<f32> = pheromone.iter().map(|&t| t.powf(opts.alpha)).collect();

        let mut ant_tours: Vec<Vec<usize>> = Vec::with_capacity(opts.num_ants);
        let mut ant_costs: Vec<f32> = Vec::with_capacity(opts.num_ants);

        for _ in 0..opts.num_ants {
            let start = rng.random_range(0..n);
            let mut visited = vec![false; n];
            visited[start] = true;
            let mut tour = Vec::with_capacity(n);
            tour.push(start);
            let mut current = start;

            for _ in 1..n {
                let mut primary: Vec<(usize, f32)> = Vec::with_capacity(n - tour.len());
                let mut fallback: Vec<(usize, f32)> = Vec::with_capacity(n - tour.len());
                for v in 0..n {
                    if visited[v] {
                        continue;
                    }
                    let eta = eta_beta[current * n + v];
                    primary.push((v, tau_alpha[current * n + v] * eta));
                    fallback.push((v, eta));
                }

                let r1: f32 = rng.random();
                let r2: f32 = rng.random();
                let next = select_next(&primary, &fallback, r1, r2);

                visited[next] = true;
                tour.push(next);
                current = next;
            }

            let cost = distances.tour_length_by_pos(&tour);
            if cost < best_cost {
                best = tour.clone();
                best_cost = cost;
                if let Some(tx) = progress_tx {
                    let best_ids: Vec<usize> = best.iter().map(|&p| to_id(p)).collect();
                    let _ = tx.send(ProgressMessage::PathUpdate(
                        Route::new(&best_ids),
                        best_cost,
                    ));
                }
            }
            ant_tours.push(tour);
            ant_costs.push(cost);
        }

        evaporate_and_floor(&mut pheromone, opts.evaporation_rate, tau_min);
        for (tour, &cost) in ant_tours.iter().zip(ant_costs.iter()) {
            deposit_tour(&mut pheromone, n, tour, cost);
        }

        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::EpochUpdate(epoch));
        }
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }

    let best_ids: Vec<usize> = best.iter().map(|&p| to_id(p)).collect();
    Solution::from_parts(&best_ids, cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{AcoOptions, HeuristicOptions, TspProblem, distance_matrix, kdtree};

    #[test]
    fn test_evaporate_and_floor_decays_above_floor() {
        let mut pheromone = vec![1.0, 1.0];
        evaporate_and_floor(&mut pheromone, 0.5, 0.0);
        assert!((pheromone[0] - 0.5).abs() < 1e-6);
        assert!((pheromone[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_evaporate_and_floor_clamps_below_floor() {
        let mut pheromone = vec![0.001, 1.0];
        evaporate_and_floor(&mut pheromone, 0.99, 0.001);
        // 0.001 * 0.01 = 0.00001, which is below the 0.001 floor -> clamped up.
        assert!((pheromone[0] - 0.001).abs() < 1e-9);
        // 1.0 * 0.01 = 0.01, which is above the floor -> left as decayed.
        assert!((pheromone[1] - 0.01).abs() < 1e-6);
    }

    #[test]
    fn test_deposit_edge_writes_both_directions() {
        let n = 3;
        let mut pheromone = vec![0.0; n * n];
        deposit_edge(&mut pheromone, n, 0, 2, 0.5);
        assert!((pheromone[0 * n + 2] - 0.5).abs() < 1e-6);
        assert!((pheromone[2 * n + 0] - 0.5).abs() < 1e-6);
        assert!((pheromone[0 * n + 1]).abs() < 1e-9);
    }

    #[test]
    fn test_deposit_tour_touches_only_tour_edges() {
        // n=4, but the tour only visits positions [0, 1, 2] (closed: 2 -> 0 too).
        // Edges touching position 3 must remain untouched.
        let n = 4;
        let mut pheromone = vec![0.0; n * n];
        let cost = 10.0;
        deposit_tour(&mut pheromone, n, &[0, 1, 2], cost);
        let amount = 1.0 / cost;
        for &(u, v) in &[(0usize, 1usize), (1, 2), (2, 0)] {
            assert!(
                (pheromone[u * n + v] - amount).abs() < 1e-6,
                "edge ({u},{v}) not deposited"
            );
            assert!(
                (pheromone[v * n + u] - amount).abs() < 1e-6,
                "edge ({v},{u}) not deposited"
            );
        }
        for pos in 0..n {
            assert!(
                (pheromone[pos * n + 3]).abs() < 1e-9,
                "edge touching position 3 must remain untouched"
            );
            assert!(
                (pheromone[3 * n + pos]).abs() < 1e-9,
                "edge touching position 3 must remain untouched"
            );
        }
    }

    #[test]
    fn test_deposit_tour_noop_on_degenerate_cost() {
        let n = 3;
        let mut pheromone = vec![0.0; n * n];
        deposit_tour(&mut pheromone, n, &[0, 1, 2], 0.0);
        assert!(pheromone.iter().all(|&t| t == 0.0));
    }

    #[test]
    fn test_roulette_select_picks_correct_bucket() {
        let weights = [(0usize, 1.0f32), (1usize, 3.0f32)];
        // sum=4.0, r=0.5 -> target=2.0; acc after (0,1.0)=1.0 < 2.0; acc after (1,3.0)=4.0 >= 2.0.
        assert_eq!(roulette_select(&weights, 4.0, 0.5), 1);
        // r close to 0 should pick the first bucket.
        assert_eq!(roulette_select(&weights, 4.0, 0.01), 0);
    }

    #[test]
    fn test_roulette_select_exhaustion_falls_back_to_last() {
        // Caller-provided sum (3.0) is inflated relative to the true weight total (2.0) —
        // simulates f32 rounding drift. With r close to 1, target (2.9997) exceeds the max
        // achievable accumulation (2.0), so the loop must exhaust and fall back to the last
        // candidate rather than panicking.
        let weights = [(0usize, 1.0f32), (1usize, 1.0f32)];
        assert_eq!(roulette_select(&weights, 3.0, 0.9999), 1);
    }

    #[test]
    fn test_select_next_uses_primary_when_healthy() {
        let primary = [(0usize, 1.0f32), (1usize, 3.0f32)];
        let fallback = [(0usize, 1.0f32), (1usize, 1.0f32)];
        assert_eq!(select_next(&primary, &fallback, 0.5, 0.5), 1);
    }

    #[test]
    fn test_select_next_falls_back_to_eta_when_primary_degenerate() {
        let primary = [(0usize, 0.0f32), (1usize, 0.0f32)];
        let fallback = [(0usize, 1.0f32), (1usize, 3.0f32)];
        assert_eq!(select_next(&primary, &fallback, 0.5, 0.5), 1);
    }

    #[test]
    fn test_select_next_falls_back_to_first_when_fully_degenerate() {
        let primary = [(0usize, 0.0f32), (1usize, f32::NAN)];
        let fallback = [(0usize, 0.0f32), (1usize, 0.0f32)];
        assert_eq!(select_next(&primary, &fallback, 0.5, 0.5), 0);
    }

    #[test]
    fn test_aco_respects_initial_tour() {
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
        let opts = AcoOptions {
            heuristic: HeuristicOptions {
                epochs: 0,
                ..HeuristicOptions::default()
            },
            ..AcoOptions::default()
        };
        let problem = TspProblem::new(cities.clone(), dm);
        let result = solve(&problem, &opts, None, Some(&optimal));
        assert!((result.total - optimal_cost).abs() < 1e-4);
        let mut visited = result.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected);
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
        let opts = AcoOptions {
            heuristic: HeuristicOptions {
                epochs: 20,
                ..HeuristicOptions::default()
            },
            num_ants: 8,
            ..AcoOptions::default()
        };
        let problem = TspProblem::new(cities.clone(), distances);
        let sol = solve(&problem, &opts, None, None);
        let mut visited = sol.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(
            visited, expected,
            "ACO tour does not visit all cities exactly once"
        );
        assert!(sol.total > 0.0 && sol.total.is_finite());
    }

    #[test]
    fn test_solve_returns_valid_tour_with_no_init_tour_uses_flat_tau0() {
        // Exercises the flat-tau0 + warn fallback (not reachable via a normal CLI run,
        // since `aco` auto-expands with shuffle, which always supplies an init_tour).
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![2.0, 0.0],
            vec![2.0, 2.0],
            vec![0.0, 2.0],
            vec![1.0, 3.0],
        ]);
        let distances = distance_matrix::from_cities(&cities);
        let opts = AcoOptions {
            heuristic: HeuristicOptions {
                epochs: 15,
                ..HeuristicOptions::default()
            },
            num_ants: 6,
            ..AcoOptions::default()
        };
        let problem = TspProblem::new(cities.clone(), distances);
        let sol = solve(&problem, &opts, None, None);
        let mut visited = sol.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected);
        assert!(sol.total > 0.0 && sol.total.is_finite());
    }

    #[test]
    fn test_solve_n_le_2_returns_trivial_tour() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 1.0]]);
        let distances = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities.clone(), distances);
        let sol = solve(&problem, &AcoOptions::default(), None, None);
        let mut visited = sol.route().to_vec();
        visited.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(visited, expected);
    }
}

//! POPMUSIC decomposition solver.
//!
//! Keeps the tour as a cyclic sequence and repeatedly re-optimizes short contiguous
//! windows with fixed endpoints, splicing the exact interior order back. The window
//! sub-solver is the shared [`held_karp_path`](super::held_karp::held_karp_path)
//! primitive.

use std::sync::mpsc;

use super::DistanceSource;
use super::held_karp::held_karp_path_with_scratch;
use super::nearest_neighbor;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Position, Solution, TspProblem};

/// Window length used when the instance is large enough. `R = 14` bounds each window DP
/// to `12 · 2^12` states.
const DEFAULT_WINDOW: usize = 14;
/// Maximum number of full passes over the tour; the solver stops earlier when a pass
/// makes no splice.
const MAX_PASSES: usize = 4;
/// Relative tolerance for accepting a window improvement. Two different interior orders
/// can differ by a rounding ulp even when their true costs tie, so a strict `<` would let
/// overlapping windows trade ulps and never settle.
const IMPROVEMENT_EPS: f64 = 1e-6;

pub fn solve(
    problem: &TspProblem,
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n = cities.len();
    let window = DEFAULT_WINDOW.min(n);

    // Fewer than four cities have no interior to reorder: every order is the same cycle.
    if n <= 2 || window < 4 {
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        return Solution::from_parts(&path, distances.as_ref());
    }

    let mut tour: Vec<usize> = match init_tour {
        Some(t) => t.to_vec(),
        // Reuse the greedy constructor rather than duplicating it. `None` keeps NN's
        // progress messages off the channel so POPMUSIC's own stream is the only one.
        None => nearest_neighbor::solve(problem, &HeuristicOptions::default(), None, None)
            .route()
            .to_vec(),
    };

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(
            Route::new(&tour),
            distances.tour_length(&tour),
        ));
    }

    let mut scratch = Scratch::new(window);
    let starts_per_offset = n.div_ceil(window);
    tracing::info!(cities = n, window, "POPMUSIC starting");

    for pass in 0..MAX_PASSES {
        let mut improved = false;
        // Offset 0 optimizes the non-overlapping windows; `window / 2` the overlapping
        // halves. Together they are one pass.
        for offset in [0, window / 2] {
            for k in 0..starts_per_offset {
                let start = (offset + k * window) % n;
                if optimize_window(&mut tour, distances.as_ref(), start, window, &mut scratch) {
                    improved = true;
                    if let Some(tx) = progress_tx {
                        let _ = tx.send(ProgressMessage::PathUpdate(
                            Route::new(&tour),
                            distances.tour_length(&tour),
                        ));
                    }
                }
            }
        }
        tracing::info!(pass, improved, "POPMUSIC pass complete");
        if !improved {
            break;
        }
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&tour, distances.as_ref())
}

/// Buffers reused across windows so the hot loop allocates only the DP's result order.
struct Scratch {
    positions: Vec<Position>,
    matrix: Vec<f32>,
    dp: Vec<f64>,
}

impl Scratch {
    fn new(window: usize) -> Self {
        Scratch {
            positions: Vec::with_capacity(window),
            matrix: vec![0.0; window * window],
            dp: Vec::new(),
        }
    }
}

/// Re-optimizes one window in place. Returns `true` when the interior order changed.
fn optimize_window(
    tour: &mut [usize],
    distances: &dyn DistanceSource,
    start: usize,
    window: usize,
    scratch: &mut Scratch,
) -> bool {
    let n = tour.len();
    let interior_len = window - 2;
    let stride = window;

    scratch.positions.clear();
    for k in 0..window {
        scratch.positions.push(
            distances
                .city_id2pos(tour[(start + k) % n])
                .expect("tour contains problem city ids"),
        );
    }

    // Local indices: 0..interior_len are the window interior, interior_len is the first
    // endpoint (start) and interior_len + 1 is the last (end).
    for a in 0..stride {
        let pa = local_position(&scratch.positions, a, interior_len);
        for b in 0..stride {
            let pb = local_position(&scratch.positions, b, interior_len);
            scratch.matrix[a * stride + b] = if pa == pb {
                // A window never traverses a self-edge, and the DP never reads the
                // diagonal; do not ask a custom source for `d(p, p)`.
                0.0
            } else {
                distances.distance_by_pos(pa, pb).unwrap_or(f32::INFINITY)
            };
        }
    }

    // Current open path, summed in the same left-to-right f64 order the DP uses so an
    // unchanged order compares bit-identically.
    let mut current = f64::from(scratch.matrix[interior_len * stride]);
    for i in 1..interior_len {
        current += f64::from(scratch.matrix[(i - 1) * stride + i]);
    }
    current += f64::from(scratch.matrix[(interior_len - 1) * stride + (interior_len + 1)]);

    let Some((candidate, order)) =
        held_karp_path_with_scratch(&scratch.matrix, interior_len, &mut scratch.dp)
    else {
        return false;
    };

    if order.iter().enumerate().all(|(i, &local)| local == i) {
        return false;
    }
    let epsilon = IMPROVEMENT_EPS * current.abs().max(1.0);
    if candidate >= current - epsilon {
        return false;
    }

    for (offset, &local) in order.iter().enumerate() {
        let slot = (start + 1 + offset) % n;
        tour[slot] = distances
            .pos2city_id(scratch.positions[1 + local])
            .expect("window position came from the tour");
    }
    true
}

fn local_position(positions: &[Position], local: usize, interior_len: usize) -> Position {
    if local < interior_len {
        positions[local + 1]
    } else if local == interior_len {
        positions[0]
    } else {
        positions[positions.len() - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree};

    fn line_problem(n: usize) -> TspProblem {
        let coords: Vec<Vec<f32>> = (0..n).map(|i| vec![i as f32 * 10.0, 0.0]).collect();
        let cities = kdtree::build_points(&coords);
        let dm = distance_matrix::from_cities_unrounded(&cities);
        TspProblem::new(cities, dm)
    }

    fn tiny_problem(n: usize) -> TspProblem {
        let cities: Vec<kdtree::KDPoint> = (0..n)
            .map(|i| kdtree::KDPoint::new_with_id(i, &[i as f32, 0.0]))
            .collect();
        let dm = if n < 2 {
            distance_matrix::DistanceMatrix::new(n, Vec::new(), cities.clone()).unwrap()
        } else {
            distance_matrix::from_cities_unrounded(&cities)
        };
        TspProblem::new(cities, dm)
    }

    fn is_permutation(route: &[usize], n: usize) -> bool {
        let mut sorted = route.to_vec();
        sorted.sort_unstable();
        sorted == (0..n).collect::<Vec<_>>()
    }

    #[test]
    fn tiny_instances_return_valid_tours() {
        for n in 0..=3 {
            let problem = tiny_problem(n);
            let solution = solve(&problem, &HeuristicOptions::default(), None, None);
            assert!(is_permutation(solution.route(), n), "n={n}");
        }
    }

    #[test]
    fn bad_seed_is_improved() {
        for (n, seed) in [
            (5, vec![0, 4, 1, 3, 2]),
            (6, vec![0, 5, 1, 4, 2, 3]),
            (7, vec![0, 6, 1, 5, 2, 4, 3]),
        ] {
            let problem = line_problem(n);
            let before = problem.distances.tour_length(&seed);
            let solution = solve(&problem, &HeuristicOptions::default(), None, Some(&seed));
            assert!(
                solution.total < before,
                "n={n}: POPMUSIC should improve the zigzag {before}, got {}",
                solution.total
            );
            assert!(is_permutation(solution.route(), n), "n={n}");
        }
    }

    #[test]
    fn warm_start_is_never_worse_than_the_seed() {
        let problem = line_problem(8);
        let seed = vec![0, 7, 1, 6, 2, 5, 3, 4];
        let before = problem.distances.tour_length(&seed);
        let solution = solve(&problem, &HeuristicOptions::default(), None, Some(&seed));
        assert!(
            solution.total <= before + 1e-3,
            "{} > {before}",
            solution.total
        );
    }

    #[test]
    fn idempotent_on_its_own_output() {
        let problem = line_problem(8);
        let seed = vec![0, 7, 1, 6, 2, 5, 3, 4];
        let first = solve(&problem, &HeuristicOptions::default(), None, Some(&seed));
        let second = solve(
            &problem,
            &HeuristicOptions::default(),
            None,
            Some(first.route()),
        );
        assert_eq!(first.route(), second.route());
        assert_eq!(first.total, second.total);
    }

    #[test]
    fn deterministic_without_a_seed() {
        let problem = line_problem(9);
        let first = solve(&problem, &HeuristicOptions::default(), None, None);
        let second = solve(&problem, &HeuristicOptions::default(), None, None);
        assert_eq!(first.route(), second.route());
        assert_eq!(first.total, second.total);
    }

    #[test]
    fn wrap_around_window_preserves_endpoints() {
        let problem = line_problem(10);
        let mut tour = vec![0, 9, 1, 8, 2, 7, 3, 6, 4, 5];
        let before = problem.distances.tour_length(&tour);
        let mut scratch = Scratch::new(6);
        let improved = optimize_window(&mut tour, problem.distances.as_ref(), 8, 6, &mut scratch);

        assert!(improved, "the wrapping window should improve the zigzag");
        assert!(is_permutation(&tour, 10));
        assert_eq!(tour[8], 4, "the window's first endpoint moved");
        assert_eq!(tour[3], 8, "the window's last endpoint moved");
        assert!(
            problem.distances.tour_length(&tour) < before,
            "the wrapping splice must lower the tour cost"
        );
    }

    #[test]
    fn progress_reports_updates_and_one_done() {
        let problem = line_problem(8);
        let seed = vec![0, 7, 1, 6, 2, 5, 3, 4];
        let (tx, rx) = mpsc::channel();
        let _ = solve(
            &problem,
            &HeuristicOptions::default(),
            Some(&tx),
            Some(&seed),
        );
        drop(tx);

        let mut updates = 0;
        let mut done = 0;
        for message in rx.iter() {
            match message {
                ProgressMessage::PathUpdate(..) => updates += 1,
                ProgressMessage::Done => done += 1,
                _ => {}
            }
        }
        assert!(updates >= 1, "expected at least one path update");
        assert_eq!(done, 1, "expected exactly one done signal");
    }
}

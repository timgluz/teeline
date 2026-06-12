use std::sync::mpsc;

use super::distance_matrix::DistanceMatrix;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem};

/// Or-opt local search: relocates segments of 1, 2, or 3 consecutive cities.
///
/// Each pass scans all possible relocations across all three segment sizes and
/// applies the globally best improving move (best-improvement), then repeats
/// until no relocation improves the tour.
///
/// Reversed insertions are included for Or-2 and Or-3 moves. These are only
/// equivalent to forward insertions under a symmetric distance matrix, which
/// all distance matrices in this project satisfy.
///
/// Complexity: O(n²) per pass (three segment sizes × n × n insertion positions).
pub fn solve(
    problem: &TspProblem,
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n = cities.len();

    tracing::info!(cities = n, "or-opt starting");

    if n < 4 {
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        return Solution::from_parts(&path, cities, distances);
    }

    let mut path: Vec<usize> = init_tour
        .map(|t| t.to_vec())
        .unwrap_or_else(|| cities.iter().map(|c| c.id).collect());

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&path), 0.0));
    }

    while let Some(best) = find_best_move(&path, distances) {
        let (_, i, j, seg_len, reversed) = best;

        #[cfg(debug_assertions)]
        let (old_len, expected_delta) = (distances.tour_length(&path), best.0);

        apply_relocation(&mut path, i, seg_len, j, reversed);

        #[cfg(debug_assertions)]
        {
            let new_len = distances.tour_length(&path);
            debug_assert!(
                (new_len - (old_len + expected_delta)).abs() < 1.0,
                "or-opt delta mismatch: expected Δ={expected_delta:.3}, got {:.3} (old={old_len:.3} new={new_len:.3})",
                new_len - old_len
            );
        }

        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::PathUpdate(
                Route::new(&path),
                distances.tour_length(&path),
            ));
        }
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&path, cities, distances)
}

/// Scan all Or-1/2/3 relocations and return the best improving move, or `None`.
///
/// Returns `(delta, seg_start, insert_after_j, seg_len, reversed)`.
/// `delta` is the change in total tour length (negative = improvement).
fn find_best_move(
    path: &[usize],
    distances: &DistanceMatrix,
) -> Option<(f32, usize, usize, usize, bool)> {
    let n = path.len();
    // -1e-3 threshold avoids infinite loops from f32 rounding at large coordinate scales.
    let mut best_delta = -1e-3_f32;
    let mut best: Option<(f32, usize, usize, usize, bool)> = None;

    for seg_len in 1_usize..=3 {
        if n <= seg_len + 1 {
            continue;
        }

        for i in 0..n {
            // Segments that wrap the array boundary are not considered (at most
            // one Or-2 and two Or-3 wrap-around cases per pass). Their exclusion
            // has negligible impact on quality in practice.
            if i + seg_len > n {
                continue;
            }

            let prev = if i == 0 { n - 1 } else { i - 1 };
            // after_seg may equal n for the last non-wrapping segment; wrap to 0.
            let after_seg = (i + seg_len) % n;

            let a = path[prev];
            let first_seg = path[i];
            let last_seg = path[i + seg_len - 1];
            let d = path[after_seg];

            // Gain from removing segment: the edge from prev→first and last→after
            // are replaced by prev→after.
            // remove_gain = d(prev,first) + d(last,after) - d(prev,after)
            // (positive = tour shortened at removal site)
            let remove_gain = distances.distance_between(a, first_seg).unwrap_or(f32::MAX)
                + distances.distance_between(last_seg, d).unwrap_or(f32::MAX)
                - distances.distance_between(a, d).unwrap_or(f32::MAX);

            // Try inserting the segment after every valid position j.
            // Forbidden: j == prev (no-op reinsertion) and j in [i, i+seg_len-1]
            // (overlaps with segment). This gives seg_len+1 forbidden positions.
            for j in 0..n {
                // Forbidden window: {prev, i, i+1, ..., i+seg_len-1}
                if j == prev || (j >= i && j < i + seg_len) {
                    continue;
                }
                // For non-wrapping forbidden window (prev < i), also OK:
                // j < prev or j >= i+seg_len are both valid.

                let x = path[j];
                let y = path[(j + 1) % n];
                let edge_xy = distances.distance_between(x, y).unwrap_or(f32::MAX);

                // delta = (new tour cost) - (old tour cost)
                //       = -remove_gain + d(x, first) + d(last, y) - d(x, y)
                // Negative delta = improvement.
                let fwd_delta = -remove_gain
                    + distances.distance_between(x, first_seg).unwrap_or(f32::MAX)
                    + distances.distance_between(last_seg, y).unwrap_or(f32::MAX)
                    - edge_xy;

                if fwd_delta < best_delta {
                    best_delta = fwd_delta;
                    best = Some((fwd_delta, i, j, seg_len, false));
                }

                // Reversed insertion for Or-2 and Or-3 (symmetric distance matrix).
                if seg_len > 1 {
                    let rev_delta = -remove_gain
                        + distances.distance_between(x, last_seg).unwrap_or(f32::MAX)
                        + distances.distance_between(first_seg, y).unwrap_or(f32::MAX)
                        - edge_xy;

                    if rev_delta < best_delta {
                        best_delta = rev_delta;
                        best = Some((rev_delta, i, j, seg_len, true));
                    }
                }
            }
        }
    }

    best
}

/// Remove the segment `path[i..i+seg_len]` and reinsert it after position `j`
/// (using the original, pre-removal index). Reverses the segment when `reversed`.
///
/// After draining `i..i+seg_len`, indices > i shift down by `seg_len`.
/// When original `j` was after the segment end (`j >= i + seg_len`), the
/// insertion point in the shortened vec is `j - seg_len + 1`; otherwise `j + 1`.
fn apply_relocation(tour: &mut Vec<usize>, i: usize, seg_len: usize, j: usize, reversed: bool) {
    let seg: Vec<usize> = tour.drain(i..i + seg_len).collect();
    let insert_at = if j >= i + seg_len {
        j - seg_len + 1
    } else {
        j + 1
    };
    if reversed {
        tour.splice(insert_at..insert_at, seg.into_iter().rev());
    } else {
        tour.splice(insert_at..insert_at, seg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{HeuristicOptions, TspProblem, distance_matrix, kdtree};

    fn make_problem(coords: &[[f32; 2]]) -> TspProblem {
        let cities =
            kdtree::build_points(&coords.iter().map(|c| vec![c[0], c[1]]).collect::<Vec<_>>());
        let dm = distance_matrix::from_cities(&cities);
        TspProblem::new(cities, dm)
    }

    // ------------------------------------------------------------------
    // apply_relocation unit tests
    // ------------------------------------------------------------------

    #[test]
    fn apply_relocation_or1_forward_move() {
        // Move city at index 1 to after index 3
        let mut tour = vec![0, 1, 2, 3, 4];
        apply_relocation(&mut tour, 1, 1, 3, false);
        assert_eq!(tour, vec![0, 2, 3, 1, 4]);
    }

    #[test]
    fn apply_relocation_or1_backward_move() {
        // Move city at index 3 to after index 0
        let mut tour = vec![0, 1, 2, 3, 4];
        apply_relocation(&mut tour, 3, 1, 0, false);
        assert_eq!(tour, vec![0, 3, 1, 2, 4]);
    }

    #[test]
    fn apply_relocation_or2_forward() {
        // Move pair [1,2] to after index 3
        let mut tour = vec![0, 1, 2, 3, 4];
        apply_relocation(&mut tour, 1, 2, 3, false);
        assert_eq!(tour, vec![0, 3, 1, 2, 4]);
    }

    #[test]
    fn apply_relocation_or2_reversed() {
        // Move pair [1,2] reversed (as [2,1]) to after index 3
        let mut tour = vec![0, 1, 2, 3, 4];
        apply_relocation(&mut tour, 1, 2, 3, true);
        assert_eq!(tour, vec![0, 3, 2, 1, 4]);
    }

    #[test]
    fn apply_relocation_or3_forward() {
        // Move triple [1,2,3] to after index 4
        let mut tour = vec![0, 1, 2, 3, 4, 5];
        apply_relocation(&mut tour, 1, 3, 4, false);
        assert_eq!(tour, vec![0, 4, 1, 2, 3, 5]);
    }

    // ------------------------------------------------------------------
    // find_best_move unit tests (verify delta sign and direction)
    // ------------------------------------------------------------------

    #[test]
    fn find_best_move_detects_or1_improvement() {
        // Tour: 0→1→2→3→4 (closed cycle). City 2 is a detour.
        // Layout: 0=(0,0), 1=(1,0), 2=(5,5) [far detour], 3=(2,0), 4=(3,0)
        // Moving city 2 elsewhere should be profitable.
        let problem = make_problem(&[
            [0.0, 0.0],
            [1.0, 0.0],
            [5.0, 5.0],
            [2.0, 0.0],
            [3.0, 0.0],
        ]);
        let path: Vec<usize> = problem.cities.iter().map(|c| c.id).collect();
        let result = find_best_move(&path, &problem.distances);
        assert!(result.is_some(), "expected an improving move");
        let (delta, _, _, _, _) = result.unwrap();
        assert!(delta < 0.0, "delta should be negative (improvement), got {delta}");
    }

    #[test]
    fn find_best_move_returns_none_for_optimal_tour() {
        // Square: [0,1,2,3] is already optimal (perimeter tour).
        let problem = make_problem(&[
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ]);
        let path = vec![0, 1, 2, 3];
        let result = find_best_move(&path, &problem.distances);
        assert!(
            result.is_none(),
            "expected no improvement on optimal square tour, got {result:?}"
        );
    }

    // ------------------------------------------------------------------
    // solve() quality tests
    // ------------------------------------------------------------------

    #[test]
    fn or_opt_improves_detour_tour() {
        // City 2 is a far detour; Or-opt should relocate it.
        let problem = make_problem(&[
            [0.0, 0.0],
            [1.0, 0.0],
            [5.0, 5.0],
            [2.0, 0.0],
            [3.0, 0.0],
        ]);
        let detour_tour: Vec<usize> = problem.cities.iter().map(|c| c.id).collect();
        let detour_cost = problem.distances.tour_length(&detour_tour);
        let sol = solve(
            &problem,
            &HeuristicOptions::default(),
            None,
            Some(&detour_tour),
        );
        assert!(
            sol.total < detour_cost,
            "or-opt should improve the detour tour: {} vs {}",
            sol.total,
            detour_cost
        );
    }

    #[test]
    fn or_opt_preserves_optimal_tour() {
        // Already-optimal 4-city square; Or-opt must not worsen it.
        let problem = make_problem(&[
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ]);
        let optimal = vec![0, 1, 2, 3];
        let sol = solve(
            &problem,
            &HeuristicOptions::default(),
            None,
            Some(&optimal),
        );
        assert!(
            (sol.total - 4.0).abs() < 1e-2,
            "tour cost changed from optimal: {}",
            sol.total
        );
    }

    #[test]
    fn or_opt_short_tour_returns_valid() {
        // n < 4 guard: should return a valid 3-city tour without panicking.
        let problem = make_problem(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        let sol = solve(&problem, &HeuristicOptions::default(), None, None);
        assert_eq!(sol.route().len(), 3);
    }

    #[test]
    fn or_opt_all_cities_visited_once() {
        // Validate tour structure: each city visited exactly once.
        let problem = make_problem(&[
            [0.0, 0.0],
            [1.0, 0.0],
            [5.0, 5.0],
            [2.0, 0.0],
            [3.0, 0.0],
            [4.0, 1.0],
        ]);
        let sol = solve(&problem, &HeuristicOptions::default(), None, None);
        let mut visited = sol.route().to_vec();
        visited.sort_unstable();
        let expected: Vec<usize> = (0..problem.cities.len()).collect();
        assert_eq!(visited, expected, "each city must appear exactly once");
    }
}

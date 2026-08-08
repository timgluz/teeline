use std::sync::mpsc;

use super::graph::{hamiltonian_cycle_to_path, select_edges, sorted_edges};
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem};

/// Greedy edge construction: sorts all pairwise edges shortest-first and greedily
/// accepts each one unless it would give a city degree 3+, or close a sub-cycle
/// before all n cities are covered (Kruskal-style construction). Unlike
/// nearest-neighbor's city-to-city walk, this defers hard decisions — cheap edges
/// are grabbed regardless of tour position, so the expensive edges it's eventually
/// forced into tend to be smaller and more evenly distributed.
///
/// Deterministic and parameter-free: correctness relies on scanning the *complete*
/// pairwise edge set (see `select_edges`'s doc comment for why), so
/// `HeuristicOptions` — including `n_nearest` — is accepted but entirely ignored,
/// the same as `christofides::solve`. Restricting to a k-nearest candidate set
/// would break the termination guarantee and could strand a city with no valid
/// partner late in the scan.
pub fn solve(
    problem: &TspProblem,
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n = cities.len();

    tracing::info!(cities = n, "greedy_edge starting");

    if n <= 2 {
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::Done);
        }
        return Solution::from_parts(&path, cities, distances);
    }

    // Step 1: all pairwise edges, sorted shortest-first.
    let edges = sorted_edges(n, distances);

    // Emit a placeholder progress update so the viz window doesn't appear frozen —
    // this solver has no meaningful intermediate state to stream (mirrors christofides).
    if let Some(tx) = progress_tx {
        let identity: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&identity), 0.0));
    }

    // Step 2: greedily accept edges subject to degree <= 2 and no-premature-cycle.
    let selected = select_edges(n, &edges);

    // Step 3: walk the single resulting cycle into an ordered path.
    let pos_path = hamiltonian_cycle_to_path(n, &selected);
    let path: Vec<usize> = pos_path.iter().map(|&pos| cities[pos].id).collect();

    if let Some(tx) = progress_tx {
        let total = distances.tour_length(&path);
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&path), total));
        let _ = tx.send(ProgressMessage::Done);
    }

    Solution::from_parts(&path, cities, distances)
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
    // select_edges tests live in graph.rs (the primitive's home).
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // solve() end-to-end
    // ------------------------------------------------------------------

    // Note: n=1 is not separately tested — `DistanceMatrix::build` (and therefore
    // `TspProblem`) requires at least 2 points, so a 1-city problem can't be
    // constructed through the normal path. The `n <= 2` guard in `solve()` still
    // covers it defensively, matching `christofides::solve`'s `n < 4` guard, which
    // similarly covers cases below its own testable minimum (n=3).

    #[test]
    fn greedy_edge_n2_returns_both_cities() {
        let problem = make_problem(&[[0., 0.], [1., 0.]]);
        let sol = solve(&problem, &HeuristicOptions::default(), None, None);
        assert_eq!(sol.route().len(), 2);
    }

    #[test]
    fn greedy_edge_n3_valid() {
        let problem = make_problem(&[[0., 0.], [1., 0.], [0., 1.]]);
        let sol = solve(&problem, &HeuristicOptions::default(), None, None);
        let mut visited = sol.route().to_vec();
        visited.sort_unstable();
        assert_eq!(visited, vec![0, 1, 2]);
    }

    #[test]
    fn greedy_edge_all_cities_visited_once() {
        let problem = make_problem(&[[0., 0.], [1., 0.], [5., 5.], [2., 0.], [3., 0.], [4., 1.]]);
        let sol = solve(&problem, &HeuristicOptions::default(), None, None);
        let mut visited = sol.route().to_vec();
        visited.sort_unstable();
        let expected: Vec<usize> = (0..problem.cities.len()).collect();
        assert_eq!(visited, expected);
    }

    #[test]
    fn greedy_edge_tour_cost_matches_reported() {
        let problem = make_problem(&[[0., 0.], [1., 0.], [2., 0.], [3., 0.], [4., 0.]]);
        let sol = solve(&problem, &HeuristicOptions::default(), None, None);
        let route = sol.route();
        let n = route.len();
        let recomputed: f32 = (0..n)
            .map(|i| {
                problem
                    .distances
                    .distance_between(route[i], route[(i + 1) % n])
                    .unwrap_or(f32::MAX)
            })
            .sum();
        assert!(
            (sol.total - recomputed).abs() < 1.0,
            "reported total ({:.2}) must match recomputed distance ({:.2})",
            sol.total,
            recomputed
        );
    }
}

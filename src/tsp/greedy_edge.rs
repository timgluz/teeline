use std::sync::mpsc;

use super::graph::{UnionFind, hamiltonian_cycle_to_path, sorted_edges};
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

// ---------------------------------------------------------------------------
// Step 2: greedy accept/reject with union-find
// ---------------------------------------------------------------------------

/// Walks `edges` (sorted ascending by weight) and greedily accepts each one unless
/// it would give either endpoint degree 3+, or close a sub-cycle before all `n`
/// edges have been placed.
///
/// Always terminates with exactly `n` accepted edges on a complete graph: both
/// rejection reasons are monotone — degree never decreases, and union-find
/// components never split — so by scan end, any two positions that still have
/// degree < 2 must already share a component (otherwise the edge between them
/// would have been accepted when scanned). That means exactly one path fragment
/// survives before the closing edge, which is why `accepted.len() == n - 1` is
/// the correct — and only — point at which a same-component edge may be accepted.
fn select_edges(n: usize, edges: &[(f32, u32, u32)]) -> Vec<(usize, usize)> {
    let mut uf = UnionFind::new(n);
    let mut degree = vec![0u8; n];
    let mut accepted: Vec<(usize, usize)> = Vec::with_capacity(n);

    for &(_, uu, vv) in edges {
        if accepted.len() == n {
            break;
        }
        let u = uu as usize;
        let v = vv as usize;
        if degree[u] >= 2 || degree[v] >= 2 {
            continue;
        }
        if uf.connected(u, v) && accepted.len() != n - 1 {
            continue;
        }
        uf.union(u, v);
        degree[u] += 1;
        degree[v] += 1;
        accepted.push((u, v));
    }

    assert_eq!(
        accepted.len(),
        n,
        "greedy edge selection must always place exactly n edges on a complete graph \
         (see select_edges doc comment); got {}",
        accepted.len()
    );

    accepted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{HeuristicOptions, TspProblem, distance_matrix, kdtree};
    use rand::RngExt;

    fn make_problem(coords: &[[f32; 2]]) -> TspProblem {
        let cities =
            kdtree::build_points(&coords.iter().map(|c| vec![c[0], c[1]]).collect::<Vec<_>>());
        let dm = distance_matrix::from_cities(&cities);
        TspProblem::new(cities, dm)
    }

    // ------------------------------------------------------------------
    // select_edges
    // ------------------------------------------------------------------

    #[test]
    fn select_edges_returns_exactly_n_edges() {
        let problem = make_problem(&[[0., 0.], [1., 0.], [1., 1.], [0., 1.], [2., 0.5]]);
        let n = problem.cities.len();
        let edges = sorted_edges(n, &problem.distances);
        let selected = select_edges(n, &edges);
        assert_eq!(selected.len(), n);
    }

    #[test]
    fn select_edges_never_exceeds_degree_2() {
        // A "star" layout where the center is closest to every other point — naive
        // greedy-by-weight would want to give the center degree 4 without the guard.
        let problem = make_problem(&[[0., 0.], [1., 0.], [-1., 0.], [0., 1.], [0., -1.]]);
        let n = problem.cities.len();
        let edges = sorted_edges(n, &problem.distances);
        let selected = select_edges(n, &edges);

        let mut degree = vec![0u8; n];
        for &(u, v) in &selected {
            degree[u] += 1;
            degree[v] += 1;
        }
        for (pos, &d) in degree.iter().enumerate() {
            assert_eq!(d, 2, "position {pos} must have degree exactly 2, got {d}");
        }
    }

    #[test]
    fn select_edges_rejects_premature_cycle() {
        // 5 collinear-ish points where the two cheapest edges plus a third cheap
        // edge would close a 3-cycle before all 5 cities are covered.
        let problem = make_problem(&[
            [0., 0.],
            [1., 0.],
            [2., 0.],
            [1., 1.], // close to (1,0) and (2,0) — tempts a premature triangle
            [10., 0.],
        ]);
        let n = problem.cities.len();
        let edges = sorted_edges(n, &problem.distances);
        let selected = select_edges(n, &edges);

        assert_eq!(selected.len(), n);
        // The result must form a single cycle over all 5 positions — this call
        // panics if select_edges ever let a premature sub-cycle through.
        let path = hamiltonian_cycle_to_path(n, &selected);
        let mut sorted = path.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..n).collect::<Vec<_>>());
    }

    #[test]
    fn select_edges_accepts_final_cycle_closing_edge() {
        // Small square: after 3 edges are placed on a 4-city cycle, the 4th
        // (cheapest remaining) edge necessarily closes the cycle and must be
        // accepted, not rejected as "premature".
        let problem = make_problem(&[[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
        let n = problem.cities.len();
        let edges = sorted_edges(n, &problem.distances);
        let selected = select_edges(n, &edges);
        assert_eq!(selected.len(), 4);

        let mut degree = vec![0u8; n];
        for &(u, v) in &selected {
            degree[u] += 1;
            degree[v] += 1;
        }
        assert!(degree.iter().all(|&d| d == 2));
    }

    #[test]
    fn select_edges_property_random_instances_always_form_one_cycle() {
        let mut rng = rand::rng();
        for n in 4..12 {
            let mut coords: Vec<[f32; 2]> = (0..n)
                .map(|_| [rng.random_range(0.0..50.0), rng.random_range(0.0..50.0)])
                .collect();
            // Force some duplicate/tied-distance points.
            if n >= 2 {
                coords[1] = coords[0];
            }
            let problem = make_problem(&coords);
            let edges = sorted_edges(n, &problem.distances);
            let selected = select_edges(n, &edges);

            assert_eq!(selected.len(), n, "n={n}: must select exactly n edges");
            let mut degree = vec![0u8; n];
            for &(u, v) in &selected {
                degree[u] += 1;
                degree[v] += 1;
            }
            assert!(
                degree.iter().all(|&d| d == 2),
                "n={n}: every position must have degree exactly 2, got {degree:?}"
            );
            // Single component: this panics if selected forms disjoint cycles.
            let path = hamiltonian_cycle_to_path(n, &selected);
            let mut sorted = path.clone();
            sorted.sort_unstable();
            assert_eq!(
                sorted,
                (0..n).collect::<Vec<_>>(),
                "n={n}: must visit all positions"
            );
        }
    }

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

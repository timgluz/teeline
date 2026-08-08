use std::sync::mpsc;

use super::distance_matrix::DistanceMatrix;
use super::graph::{hamiltonian_cycle_to_path, select_edges};
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem};

/// Savings construction: a constructive solver that ranks pairwise edges by the
/// *savings* of linking two cities directly instead of routing both through a
/// reference "hub", then greedily accepts each on the same Kruskal-style
/// scaffolding as `greedy_edge` (degree ≤ 2, no premature sub-cycle).
///
/// Inspired by the Clarke-Wright savings heuristic, but **not** canonical
/// Clarke-Wright route merging: there is no hub-edge removal or route-endpoint
/// merge step. The hub is used *only* to compute the savings sort key —
/// mechanically this is a savings-ordered greedy-edge construction. Reusing
/// `graph::select_edges` unchanged is what makes that distinction precise.
///
/// The hub is chosen as the city nearest the coordinate centroid, which gives a
/// balanced savings distribution and is order-independent (unlike a fixed
/// city-0 hub, whose merge ordering would shift if the input were permuted). The
/// hub is still visited like every other city — it only biases the *ordering* of
/// candidate merges, not the set of cities toured.
///
/// Mechanically it differs from `greedy_edge` only in its sort key (savings,
/// descending, vs raw distance, ascending) and its hub reference. Both reuse
/// `graph::select_edges` and `graph::hamiltonian_cycle_to_path`.
///
/// Deterministic and parameter-free: like `greedy_edge`, correctness relies on
/// scanning the complete pairwise edge set, so `HeuristicOptions` is accepted
/// but entirely ignored.
pub fn solve(
    problem: &TspProblem,
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n = cities.len();

    tracing::info!(cities = n, "savings starting");

    if n <= 2 {
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::Done);
        }
        return Solution::from_parts(&path, cities, distances);
    }

    // Step 1: pick the hub as the city nearest the coordinate centroid.
    let hub = hub_position(cities);

    // Emit a placeholder progress update so the viz window doesn't appear frozen —
    // this solver has no meaningful intermediate state to stream (mirrors greedy_edge).
    if let Some(tx) = progress_tx {
        let identity: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&identity), 0.0));
    }

    // Step 2: all pairwise edges, ranked by savings (descending).
    let edges = sorted_edges_by_savings(n, hub, distances);

    // Step 3: greedily accept edges subject to degree <= 2 and no-premature-cycle
    // (shared with greedy_edge — only the sort order above differs).
    let selected = select_edges(n, &edges);

    // Step 4: walk the single resulting cycle into an ordered path.
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
// Step 1: hub selection — centroid-nearest
// ---------------------------------------------------------------------------

/// Returns the position index of the city nearest the coordinate centroid.
///
/// Uses squared Euclidean distance for the comparison (the argmin is invariant
/// under the monotone sqrt, so skipping it avoids n square roots). A single O(n)
/// pass — cheaper than building a KD-tree for one query, and `TspProblem` carries
/// no pre-built tree to reuse.
fn hub_position(cities: &[KDPoint]) -> usize {
    let n = cities.len();
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    for c in cities {
        cx += c.coords[0];
        cy += c.coords[1];
    }
    cx /= n as f32;
    cy /= n as f32;

    let mut best = 0usize;
    let mut best_d2 = f32::MAX;
    for (i, c) in cities.iter().enumerate() {
        let dx = c.coords[0] - cx;
        let dy = c.coords[1] - cy;
        let d2 = dx * dx + dy * dy;
        if d2 < best_d2 {
            best_d2 = d2;
            best = i;
        }
    }
    best
}

// ---------------------------------------------------------------------------
// Step 2: savings-ranked candidate edges
// ---------------------------------------------------------------------------

/// All `n*(n-1)/2` pairwise edges among position indices `0..n`, sorted
/// **descending** by savings relative to `hub`: pairs that save the
/// most by being linked directly (rather than both via the hub) sort first.
///
/// Hub-involving pairs (`i == hub` or `j == hub`) have savings `0.0` (since
/// `d(hub,hub)=0`). Under EUC_2D the triangle inequality keeps every non-hub
/// savings ≥ 0, so hub pairs sort last; under GEO (floored great-circle
/// distances) flooring can break the triangle inequality and produce negative
/// non-hub savings, so hub pairs may not be last. This is irrelevant to
/// termination: `select_edges` always places exactly `n` edges forming one
/// Hamiltonian cycle regardless of sort order (its spanning-forest argument is
/// order-independent), so the hub always ends up with degree 2 like every other
/// position.
fn sorted_edges_by_savings(
    n: usize,
    hub: usize,
    distances: &DistanceMatrix,
) -> Vec<(f32, u32, u32)> {
    let mut edges = Vec::with_capacity(n * (n.saturating_sub(1)) / 2);
    for i in 0..n {
        for j in (i + 1)..n {
            let d_hub_i = distances
                .distance_by_pos(hub, i)
                .expect("hub, i are within 0..n by construction");
            let d_hub_j = distances
                .distance_by_pos(hub, j)
                .expect("hub, j are within 0..n by construction");
            let d_ij = distances
                .distance_by_pos(i, j)
                .expect("i, j are within 0..n by construction");
            // s(i,j) = d(hub,i) + d(hub,j) - d(i,j); 0 when either endpoint is the hub.
            let savings = d_hub_i + d_hub_j - d_ij;
            edges.push((savings, i as u32, j as u32));
        }
    }
    // Descending by savings. `total_cmp` gives a real total order over all f32 bit
    // patterns (including NaN), matching `sorted_edges`'s NaN-safety rationale —
    // never panics on malformed input the way `partial_cmp(...).unwrap()` would.
    edges.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
    edges
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
    // hub_position
    // ------------------------------------------------------------------

    #[test]
    fn hub_position_picks_centroid_nearest() {
        // Centroid is (0,0); the city at the origin is the nearest to it.
        let problem = make_problem(&[[0., 0.], [10., 0.], [0., 10.], [10., 10.]]);
        let hub = hub_position(&problem.cities);
        assert_eq!(hub, 0);
    }

    #[test]
    fn hub_position_is_within_bounds() {
        let problem = make_problem(&[[1., 2.], [3., 4.], [5., 6.], [7., 8.]]);
        let hub = hub_position(&problem.cities);
        assert!(hub < problem.cities.len());
    }

    // ------------------------------------------------------------------
    // sorted_edges_by_savings
    // ------------------------------------------------------------------

    #[test]
    fn savings_list_has_n_choose_2_entries() {
        let problem = make_problem(&[[0., 0.], [1., 0.], [2., 0.], [3., 0.]]);
        let n = problem.cities.len();
        let edges = sorted_edges_by_savings(n, 0, &problem.distances);
        assert_eq!(edges.len(), n * (n - 1) / 2);
    }

    #[test]
    fn savings_list_is_sorted_descending() {
        let problem = make_problem(&[[0., 0.], [1., 0.], [2., 0.], [3., 0.]]);
        let edges = sorted_edges_by_savings(4, 0, &problem.distances);
        for pair in edges.windows(2) {
            assert!(
                pair[0].0 >= pair[1].0,
                "savings must be sorted descending, got {} before {}",
                pair[0].0,
                pair[1].0
            );
        }
    }

    #[test]
    fn savings_for_hub_pair_is_zero() {
        // Any pair involving the hub has savings 0.
        let problem = make_problem(&[[0., 0.], [5., 0.], [10., 0.]]);
        let edges = sorted_edges_by_savings(3, 0, &problem.distances);
        for &(_s, i, j) in &edges {
            if i as usize == 0 || j as usize == 0 {
                // hub-involving pair
                let s = edges
                    .iter()
                    .find(|&&(_, ii, jj)| ii == i && jj == j)
                    .unwrap()
                    .0;
                assert!(
                    (s - 0.0).abs() < 1e-6,
                    "hub-pair savings must be 0, got {s}"
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // solve() end-to-end
    // ------------------------------------------------------------------

    // Note: n=1 is not separately tested — `DistanceMatrix::build` (and therefore
    // `TspProblem`) requires at least 2 points. The `n <= 2` guard covers it
    // defensively, matching `greedy_edge::solve`'s guard.

    #[test]
    fn savings_n2_returns_both_cities() {
        let problem = make_problem(&[[0., 0.], [1., 0.]]);
        let sol = solve(&problem, &HeuristicOptions::default(), None, None);
        assert_eq!(sol.route().len(), 2);
    }

    #[test]
    fn savings_n3_valid() {
        let problem = make_problem(&[[0., 0.], [1., 0.], [0., 1.]]);
        let sol = solve(&problem, &HeuristicOptions::default(), None, None);
        let mut visited = sol.route().to_vec();
        visited.sort_unstable();
        assert_eq!(visited, vec![0, 1, 2]);
    }

    #[test]
    fn savings_all_cities_visited_once() {
        let problem = make_problem(&[[0., 0.], [1., 0.], [5., 5.], [2., 0.], [3., 0.], [4., 1.]]);
        let sol = solve(&problem, &HeuristicOptions::default(), None, None);
        let mut visited = sol.route().to_vec();
        visited.sort_unstable();
        let expected: Vec<usize> = (0..problem.cities.len()).collect();
        assert_eq!(visited, expected);
    }

    #[test]
    fn savings_tour_cost_matches_reported() {
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

    #[test]
    fn savings_property_random_instances_form_one_cycle() {
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
            let sol = solve(&problem, &HeuristicOptions::default(), None, None);

            let mut visited = sol.route().to_vec();
            visited.sort_unstable();
            assert_eq!(
                visited,
                (0..n).collect::<Vec<_>>(),
                "n={n}: must visit all positions exactly once"
            );
        }
    }
}

use std::sync::mpsc;

use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem};
use crate::tsp::kdtree::KDPoint;

/// Christofides approximation algorithm for the metric TSP.
///
/// Produces a tour guaranteed to be within **1.5× the optimal length** when the
/// distance matrix satisfies the triangle inequality (EUC_2D instances do; arbitrary
/// FULL_MATRIX instances may not — the algorithm still produces a valid tour, but the
/// bound no longer applies).
///
/// The six steps of the algorithm:
/// 1. Build a minimum spanning tree (Prim's, O(n²)).
/// 2. Find all odd-degree vertices in the MST (handshaking lemma: always an even count).
/// 3. Compute a minimum-weight perfect matching on the odd-degree set (greedy approximation).
/// 4. Form a multigraph: MST edges ∪ matching edges.
/// 5. Find an Eulerian circuit in the multigraph (Hierholzer's algorithm).
/// 6. Shortcut repeated cities to obtain a Hamiltonian tour.
///
/// All intermediate work uses **position indices** (0..n into `problem.cities`);
/// city IDs are only materialised in the final output step.
pub fn solve(
    problem: &TspProblem,
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n = cities.len();

    tracing::info!(cities = n, "christofides starting");

    if n < 4 {
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::Done);
        }
        return Solution::from_parts(&path, cities, distances);
    }

    // Step 1: Minimum Spanning Tree via Prim's.
    let mst_edges = prim_mst(n, cities, distances);

    // Emit a placeholder progress update so the viz window doesn't appear frozen.
    if let Some(tx) = progress_tx {
        let identity: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&identity), 0.0));
    }

    // Step 2: Identify odd-degree vertices.
    let odd = odd_degree_nodes(&mst_edges, n);
    debug_assert!(odd.len().is_multiple_of(2), "handshaking lemma: odd-degree count must be even");

    // Step 3: Greedy minimum-weight perfect matching on odd-degree nodes.
    let matching = greedy_matching(&odd, cities, distances, n);

    // Step 4: Build multigraph (MST edges ∪ matching edges).
    let mut adj = build_multigraph(n, &mst_edges, &matching);

    // Step 5: Eulerian circuit via Hierholzer's algorithm.
    let circuit = hierholzer(&mut adj, 0);

    // Step 6: Shortcut repeated cities → Hamiltonian tour.
    let hamiltonian = shortcut(&circuit, n);
    let path: Vec<usize> = hamiltonian.iter().map(|&pos| cities[pos].id).collect();

    if let Some(tx) = progress_tx {
        let total = distances.tour_length(&path);
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&path), total));
        let _ = tx.send(ProgressMessage::Done);
    }

    Solution::from_parts(&path, cities, distances)
}

// ---------------------------------------------------------------------------
// Step 1: Prim's MST — O(n²)
// ---------------------------------------------------------------------------

/// Returns MST edges as `(pos_a, pos_b)` position-index pairs.
fn prim_mst(n: usize, cities: &[KDPoint], distances: &super::distance_matrix::DistanceMatrix) -> Vec<(usize, usize)> {
    let mut in_mst = vec![false; n];
    let mut key = vec![f32::MAX; n];   // minimum edge weight to bring each vertex into the tree
    let mut parent = vec![usize::MAX; n];
    key[0] = 0.0;

    let mut edges = Vec::with_capacity(n - 1);

    for _ in 0..n {
        // Minimum-key vertex not yet in MST.
        let u = (0..n)
            .filter(|&i| !in_mst[i])
            .min_by(|&a, &b| key[a].partial_cmp(&key[b]).unwrap_or(std::cmp::Ordering::Equal))
            .expect("at least one non-MST vertex remains");

        in_mst[u] = true;

        if parent[u] != usize::MAX {
            edges.push((u, parent[u]));
        }

        for v in 0..n {
            if !in_mst[v] {
                let d = distances
                    .distance_between(cities[u].id, cities[v].id)
                    .expect("city IDs are valid — distance matrix was built from the same cities");
                if d < key[v] {
                    key[v] = d;
                    parent[v] = u;
                }
            }
        }
    }

    edges
}

// ---------------------------------------------------------------------------
// Step 2: Odd-degree vertices
// ---------------------------------------------------------------------------

fn odd_degree_nodes(mst_edges: &[(usize, usize)], n: usize) -> Vec<usize> {
    let mut degree = vec![0u32; n];
    for &(u, v) in mst_edges {
        degree[u] += 1;
        degree[v] += 1;
    }
    (0..n).filter(|&i| degree[i] % 2 == 1).collect()
}

// ---------------------------------------------------------------------------
// Step 3: Greedy minimum-weight perfect matching
// ---------------------------------------------------------------------------

/// Sort-then-match greedy: collect all C(k,2) pairs of odd-degree positions,
/// sort by edge weight, and greedily match shortest-first.
///
/// `matched` is indexed by position (0..n), not by index into `odd`, so it is
/// sized to `n` even though only `odd.len()` positions are relevant.
fn greedy_matching(
    odd: &[usize],
    cities: &[KDPoint],
    distances: &super::distance_matrix::DistanceMatrix,
    n: usize,
) -> Vec<(usize, usize)> {
    let k = odd.len();
    let mut pairs: Vec<(f32, usize, usize)> = Vec::with_capacity(k * k / 2);
    for i in 0..k {
        for j in i + 1..k {
            let d = distances
                .distance_between(cities[odd[i]].id, cities[odd[j]].id)
                .expect("city IDs are valid");
            pairs.push((d, odd[i], odd[j]));
        }
    }
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // matched[pos] = true once that position has been paired.
    let mut matched = vec![false; n];
    let mut result = Vec::with_capacity(k / 2);
    for (_, u, v) in pairs {
        if !matched[u] && !matched[v] {
            matched[u] = true;
            matched[v] = true;
            result.push((u, v));
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Step 4: Build multigraph adjacency list
// ---------------------------------------------------------------------------

fn build_multigraph(
    n: usize,
    mst_edges: &[(usize, usize)],
    matching: &[(usize, usize)],
) -> Vec<Vec<usize>> {
    let mut adj = vec![vec![]; n];
    for &(u, v) in mst_edges.iter().chain(matching.iter()) {
        adj[u].push(v);
        adj[v].push(u);
    }
    adj
}

// ---------------------------------------------------------------------------
// Step 5: Hierholzer's Eulerian circuit (iterative)
// ---------------------------------------------------------------------------

/// Returns the Eulerian circuit as a sequence of position indices, including
/// the return to the start vertex (length = |edges| + 1).
///
/// Correctness relies on the multigraph being connected and all vertices having
/// even degree — both guaranteed by the Christofides construction.
fn hierholzer(adj: &mut [Vec<usize>], start: usize) -> Vec<usize> {
    let mut stack = vec![start];
    let mut circuit = Vec::new();

    while let Some(&v) = stack.last() {
        if !adj[v].is_empty() {
            let u = adj[v].pop().unwrap();
            // Remove the reverse half-edge (one occurrence only — multi-edges are valid).
            if let Some(pos) = adj[u].iter().position(|&x| x == v) {
                adj[u].swap_remove(pos);
            }
            stack.push(u);
        } else {
            circuit.push(stack.pop().unwrap());
        }
    }

    circuit.reverse();

    debug_assert!(
        adj.iter().all(Vec::is_empty),
        "Eulerian circuit must consume all edges — connectivity or parity invariant broken"
    );

    circuit
}

// ---------------------------------------------------------------------------
// Step 6: Shortcut to Hamiltonian tour
// ---------------------------------------------------------------------------

/// Remove repeated position indices from the Eulerian circuit, keeping the
/// first occurrence of each.  Returns a permutation of 0..n.
fn shortcut(circuit: &[usize], n: usize) -> Vec<usize> {
    let mut seen = vec![false; n];
    circuit
        .iter()
        .filter(|&&v| {
            let fresh = !seen[v];
            seen[v] = true;
            fresh
        })
        .copied()
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
    // Step 1: Prim's MST
    // ------------------------------------------------------------------

    #[test]
    fn prim_mst_has_n_minus_1_edges() {
        let problem = make_problem(&[[0., 0.], [1., 0.], [2., 0.], [1., 1.]]);
        let n = problem.cities.len();
        let edges = prim_mst(n, &problem.cities, &problem.distances);
        assert_eq!(edges.len(), n - 1, "MST must have exactly n-1 edges");
        // All positions reachable
        let mut reachable = vec![false; n];
        reachable[0] = true;
        for &(u, v) in &edges {
            reachable[u] = true;
            reachable[v] = true;
        }
        assert!(reachable.iter().all(|&r| r), "MST must span all nodes");
    }

    #[test]
    fn prim_mst_weight_is_minimal_on_chain() {
        // Linear layout: (0,0)-(1,0)-(2,0)-(3,0)-(4,0)
        // Optimal MST is the chain itself; weight = 4.0.
        let problem = make_problem(&[[0., 0.], [1., 0.], [2., 0.], [3., 0.], [4., 0.]]);
        let n = problem.cities.len();
        let edges = prim_mst(n, &problem.cities, &problem.distances);
        let weight: f32 = edges
            .iter()
            .map(|&(u, v)| {
                problem
                    .distances
                    .distance_between(problem.cities[u].id, problem.cities[v].id)
                    .unwrap()
            })
            .sum();
        assert!((weight - 4.0).abs() < 1e-3, "chain MST weight must be 4.0, got {weight}");
    }

    // ------------------------------------------------------------------
    // Step 2: Odd-degree nodes
    // ------------------------------------------------------------------

    #[test]
    fn odd_degree_count_is_even() {
        // Star at position 1 + one more branch: degrees are 1,3,1,1,1 → odd: {0,1,2,3,4} = 5? No.
        // Let's use a known MST: (0-1), (1-2), (1-3) → degrees: 0→1, 1→3, 2→1, 3→1.
        // Odd: {0,1,2,3} → count = 4, even. ✓
        let mst = vec![(0, 1), (1, 2), (1, 3)];
        let odd = odd_degree_nodes(&mst, 4);
        assert_eq!(odd.len() % 2, 0, "odd-degree count must be even (handshaking lemma)");
    }

    #[test]
    fn odd_degree_nodes_correct_values() {
        // Path graph: (0-1), (1-2), (2-3)
        // Degrees: 0→1, 1→2, 2→2, 3→1 → odd: {0, 3}
        let mst = vec![(0, 1), (1, 2), (2, 3)];
        let mut odd = odd_degree_nodes(&mst, 4);
        odd.sort_unstable();
        assert_eq!(odd, vec![0, 3], "endpoints of a path are odd-degree");
    }

    // ------------------------------------------------------------------
    // Step 3: Greedy matching
    // ------------------------------------------------------------------

    #[test]
    fn greedy_matching_covers_all_odd_nodes() {
        // 5-node star+chain: MST has odd nodes {0,1,3,4} (see planning notes)
        let problem =
            make_problem(&[[0., 0.], [1., 0.], [2., 0.], [1., 1.], [3., 0.]]);
        let n = problem.cities.len();
        let mst = prim_mst(n, &problem.cities, &problem.distances);
        let odd = odd_degree_nodes(&mst, n);
        let matching = greedy_matching(&odd, &problem.cities, &problem.distances, n);

        // Each odd-degree position must appear exactly once in the matching.
        let mut count = vec![0u32; n];
        for &(u, v) in &matching {
            count[u] += 1;
            count[v] += 1;
        }
        for &pos in &odd {
            assert_eq!(count[pos], 1, "position {pos} must be matched exactly once");
        }
    }

    // ------------------------------------------------------------------
    // Step 5: Hierholzer's Eulerian circuit
    // ------------------------------------------------------------------

    #[test]
    fn eulerian_circuit_length_equals_edges_plus_one() {
        let problem =
            make_problem(&[[0., 0.], [1., 0.], [2., 0.], [1., 1.], [3., 0.]]);
        let n = problem.cities.len();
        let mst = prim_mst(n, &problem.cities, &problem.distances);
        let odd = odd_degree_nodes(&mst, n);
        let matching = greedy_matching(&odd, &problem.cities, &problem.distances, n);
        let total_edges = mst.len() + matching.len();
        let mut adj = build_multigraph(n, &mst, &matching);
        let circuit = hierholzer(&mut adj, 0);
        assert_eq!(
            circuit.len(),
            total_edges + 1,
            "Eulerian circuit must traverse every edge once (length = edges + 1)"
        );
    }

    // ------------------------------------------------------------------
    // Step 6: Shortcut
    // ------------------------------------------------------------------

    #[test]
    fn shortcut_removes_duplicates() {
        // Circuit visits position 1 twice — shortcut keeps first occurrence.
        let circuit = vec![0, 1, 3, 4, 2, 1, 0];
        let n = 5;
        let tour = shortcut(&circuit, n);
        assert_eq!(tour.len(), n, "shortcut must produce n-city tour");
        let mut sorted = tour.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4], "tour must be a permutation of 0..n");
    }

    // ------------------------------------------------------------------
    // solve() end-to-end
    // ------------------------------------------------------------------

    #[test]
    fn christofides_n3_early_exit_valid() {
        // n < 4 guard: must return a valid 3-city tour without panic.
        let problem = make_problem(&[[0., 0.], [1., 0.], [1., 1.]]);
        let sol = solve(&problem, &HeuristicOptions::default(), None, None);
        assert_eq!(sol.route().len(), 3, "tour must visit all 3 cities");
    }

    #[test]
    fn christofides_all_cities_visited_once() {
        // 6-city problem: validate tour is a valid permutation.
        let problem = make_problem(&[
            [0., 0.],
            [1., 0.],
            [5., 5.],
            [2., 0.],
            [3., 0.],
            [4., 1.],
        ]);
        let sol = solve(&problem, &HeuristicOptions::default(), None, None);
        let mut visited = sol.route().to_vec();
        visited.sort_unstable();
        let expected: Vec<usize> = (0..problem.cities.len()).collect();
        assert_eq!(visited, expected, "each city must appear exactly once");
    }

    #[test]
    fn christofides_tour_cost_matches_reported() {
        // Reported total must match the sum of edge distances in the tour.
        let problem = make_problem(&[
            [0., 0.],
            [1., 0.],
            [2., 0.],
            [3., 0.],
            [4., 0.],
        ]);
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

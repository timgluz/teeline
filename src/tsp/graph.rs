//! Reusable graph-construction primitives for Kruskal-style tour builders.
//!
//! Kept separate from `greedy_edge` (its first caller) so a future MST-via-Kruskal
//! rewrite of `christofides`'s matching step, or a savings-algorithm solver, can
//! reuse `UnionFind`/`sorted_edges` without duplicating them.

use super::distance_matrix::DistanceMatrix;

/// Disjoint-set over position indices `0..n`, with path compression and union by rank.
pub(crate) struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    pub(crate) fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    pub(crate) fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    /// Merges the sets containing `a` and `b`. Returns `true` if a merge happened,
    /// `false` if they were already in the same set.
    pub(crate) fn union(&mut self, a: usize, b: usize) -> bool {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return false;
        }
        match self.rank[ra].cmp(&self.rank[rb]) {
            std::cmp::Ordering::Less => self.parent[ra] = rb,
            std::cmp::Ordering::Greater => self.parent[rb] = ra,
            std::cmp::Ordering::Equal => {
                self.parent[rb] = ra;
                self.rank[ra] += 1;
            }
        }
        true
    }

    pub(crate) fn connected(&mut self, a: usize, b: usize) -> bool {
        self.find(a) == self.find(b)
    }
}

/// All `n*(n-1)/2` pairwise edges among position indices `0..n`, sorted ascending
/// by weight. Indices are packed as `u32` (not `usize`) to roughly halve the
/// working-set size of this O(n²) list — meaningful at the few-thousand-city scale
/// where this list's memory dominates (e.g. ~1.2GB as `(f32, usize, usize)` at
/// n=10,000 on a 64-bit target, vs ~600MB as `(f32, u32, u32)`).
pub(crate) fn sorted_edges(n: usize, distances: &DistanceMatrix) -> Vec<(f32, u32, u32)> {
    let mut edges = Vec::with_capacity(n * (n.saturating_sub(1)) / 2);
    for i in 0..n {
        for j in (i + 1)..n {
            let d = distances
                .distance_by_pos(i, j)
                .expect("i, j are within 0..n by construction");
            edges.push((d, i as u32, j as u32));
        }
    }
    // `total_cmp` gives a real total order over all f32 bit patterns (including NaN),
    // so this never panics even on malformed input (e.g. a TSPLIB file with a "nan"
    // coordinate token, which `f32::from_str` parses successfully) — unlike
    // `partial_cmp(...).unwrap_or(Equal)`, whose fabricated "Equal" for NaN pairs is
    // not transitive and can make the sort's internal ordering checks panic with
    // "user-provided comparison function does not correctly implement a total order".
    // No caller depends on stability among tied distances (the property test below
    // forces duplicate points), so the allocation-free unstable sort is strictly better.
    edges.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    edges
}

/// Walks `edges` (sorted by some weight the caller chooses) and greedily accepts
/// each one unless it would give either endpoint degree 3+, or close a sub-cycle
/// before all `n` edges have been placed.
///
/// Shared by the Kruskal-style constructive solvers (`greedy_edge` sorts edges
/// ascending by distance; `savings` sorts descending by Clarke-Wright savings).
/// The leading `f32` weight is unused inside the loop — only the sort order the
/// caller established before calling matters — so this primitive is agnostic to
/// the sort key's meaning.
///
/// Always terminates with exactly `n` accepted edges on a complete graph: both
/// rejection reasons are monotone — degree never decreases, and union-find
/// components never split — so by scan end, any two positions that still have
/// degree < 2 must already share a component (otherwise the edge between them
/// would have been accepted when scanned). That means exactly one path fragment
/// survives before the closing edge, which is why `accepted.len() == n - 1` is
/// the correct — and only — point at which a same-component edge may be accepted.
pub(crate) fn select_edges(n: usize, edges: &[(f32, u32, u32)]) -> Vec<(usize, usize)> {
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
        "kruskal-style edge selection must always place exactly n edges on a complete graph \
         (see select_edges doc comment); got {}",
        accepted.len()
    );

    accepted
}

/// Walks a degree-exactly-2 edge set forming a single cycle over `0..n` into an
/// ordered path, starting at position 0.
///
/// Precondition: `edges` must contain exactly `n` edges, every position must have
/// degree exactly 2, and the edges must form *one* cycle covering all n positions
/// (not several disjoint cycles). This is guaranteed by `greedy_edge::select_edges`'s
/// construction, not re-derived here — violating it panics rather than silently
/// returning a corrupt/duplicated path.
pub(crate) fn hamiltonian_cycle_to_path(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    assert_eq!(
        edges.len(),
        n,
        "hamiltonian_cycle_to_path requires exactly n edges, got {}",
        edges.len()
    );

    let mut adj: Vec<Vec<usize>> = vec![Vec::with_capacity(2); n];
    for &(u, v) in edges {
        adj[u].push(v);
        adj[v].push(u);
    }
    for (pos, neighbors) in adj.iter().enumerate() {
        assert_eq!(
            neighbors.len(),
            2,
            "hamiltonian_cycle_to_path requires every position to have degree exactly 2; \
             position {pos} has degree {}",
            neighbors.len()
        );
    }

    let mut path = Vec::with_capacity(n);
    let mut seen = vec![false; n];
    let mut prev = usize::MAX;
    let mut cur = 0usize;
    for _ in 0..n {
        assert!(
            !seen[cur],
            "hamiltonian_cycle_to_path: edges do not form a single cycle covering all \
             positions (revisited position {cur} before all {n} were seen) — likely \
             several disjoint cycles"
        );
        seen[cur] = true;
        path.push(cur);
        let next = adj[cur]
            .iter()
            .copied()
            .find(|&x| x != prev)
            .unwrap_or_else(|| {
                panic!(
                    "hamiltonian_cycle_to_path: position {cur}'s neighbors are both equal to the \
                 previously-visited position {prev} — this is not a simple cycle (a parallel \
                 edge between {cur} and {prev}?)"
                )
            });
        prev = cur;
        cur = next;
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree};
    use rand::RngExt;

    // -----------------------------------------------------------------
    // UnionFind
    // -----------------------------------------------------------------

    #[test]
    fn union_find_singletons_are_disjoint() {
        let mut uf = UnionFind::new(4);
        assert!(!uf.connected(0, 1));
        assert!(!uf.connected(2, 3));
    }

    #[test]
    fn union_find_union_merges_two_sets() {
        let mut uf = UnionFind::new(4);
        assert!(uf.union(0, 1));
        assert!(uf.connected(0, 1));
        assert!(!uf.connected(0, 2));
    }

    #[test]
    fn union_find_union_on_already_connected_returns_false() {
        let mut uf = UnionFind::new(3);
        assert!(uf.union(0, 1));
        assert!(
            !uf.union(0, 1),
            "second union of the same pair must return false"
        );
        assert!(
            !uf.union(1, 0),
            "order-reversed re-union must also return false"
        );
    }

    #[test]
    fn union_find_path_compression_preserves_correctness() {
        // Chain unions: 0-1, 1-2, 2-3, 3-4 — forces a deep find() chain pre-compression.
        let mut uf = UnionFind::new(5);
        uf.union(0, 1);
        uf.union(1, 2);
        uf.union(2, 3);
        uf.union(3, 4);
        for a in 0..5 {
            for b in 0..5 {
                assert!(
                    uf.connected(a, b),
                    "{a} and {b} must be connected after chain unions"
                );
            }
        }
    }

    #[test]
    fn union_find_multi_union_forest_scenario() {
        // 6 elements, 5 unions forming one tree: all pairs must be connected.
        let mut uf = UnionFind::new(6);
        assert!(uf.union(0, 1));
        assert!(uf.union(2, 3));
        assert!(uf.union(4, 5));
        assert!(uf.union(1, 2));
        assert!(uf.union(3, 4));
        for a in 0..6 {
            for b in 0..6 {
                assert!(uf.connected(a, b));
            }
        }
    }

    // -----------------------------------------------------------------
    // sorted_edges
    // -----------------------------------------------------------------

    fn square_problem() -> (Vec<kdtree::KDPoint>, DistanceMatrix) {
        // Unit square: 0=(0,0) 1=(1,0) 2=(1,1) 3=(0,1)
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        (cities, dm)
    }

    #[test]
    fn sorted_edges_has_n_choose_2_entries() {
        let (cities, dm) = square_problem();
        let n = cities.len();
        let edges = sorted_edges(n, &dm);
        assert_eq!(edges.len(), n * (n - 1) / 2);
    }

    #[test]
    fn sorted_edges_is_sorted_ascending() {
        let (cities, dm) = square_problem();
        let edges = sorted_edges(cities.len(), &dm);
        for pair in edges.windows(2) {
            assert!(
                pair[0].0 <= pair[1].0,
                "edges must be sorted ascending by weight"
            );
        }
        // Cheapest edges on a unit square are the 4 sides (length 1.0); diagonals
        // (length sqrt(2)) must sort after them.
        assert!((edges[0].0 - 1.0).abs() < 1e-6);
    }

    // -----------------------------------------------------------------
    // select_edges
    // -----------------------------------------------------------------

    #[test]
    fn select_edges_returns_exactly_n_edges() {
        let (_, dm) = square_problem();
        let n = 4;
        let edges = sorted_edges(n, &dm);
        let selected = select_edges(n, &edges);
        assert_eq!(selected.len(), n);
    }

    #[test]
    fn select_edges_never_exceeds_degree_2() {
        // A "star" layout where the center is closest to every other point — naive
        // greedy-by-weight would want to give the center degree 4 without the guard.
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![-1.0, 0.0],
            vec![0.0, 1.0],
            vec![0.0, -1.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let n = cities.len();
        let edges = sorted_edges(n, &dm);
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
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![2.0, 0.0],
            vec![1.0, 1.0], // close to (1,0) and (2,0) — tempts a premature triangle
            vec![10.0, 0.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let n = cities.len();
        let edges = sorted_edges(n, &dm);
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
        let (_, dm) = square_problem();
        let n = 4;
        let edges = sorted_edges(n, &dm);
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
            let mut rows: Vec<Vec<f32>> = (0..n)
                .map(|_| vec![rng.random_range(0.0..50.0), rng.random_range(0.0..50.0)])
                .collect();
            // Force some duplicate/tied-distance points.
            if n >= 2 {
                rows[1] = rows[0].clone();
            }
            let cities = kdtree::build_points(&rows);
            let dm = distance_matrix::from_cities(&cities);
            let edges = sorted_edges(n, &dm);
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

    // -----------------------------------------------------------------
    // hamiltonian_cycle_to_path
    // -----------------------------------------------------------------

    #[test]
    fn hamiltonian_cycle_to_path_hand_built_4_cycle() {
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 0)];
        let path = hamiltonian_cycle_to_path(4, &edges);
        assert_eq!(path, vec![0, 1, 2, 3]);
    }

    #[test]
    fn hamiltonian_cycle_to_path_is_a_permutation() {
        let edges = vec![(0, 2), (2, 4), (4, 1), (1, 3), (3, 0)];
        let mut path = hamiltonian_cycle_to_path(5, &edges);
        path.sort_unstable();
        assert_eq!(path, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    #[should_panic(expected = "degree exactly 2")]
    fn hamiltonian_cycle_to_path_panics_on_wrong_degree() {
        // Exactly n=4 edges, but position 0 has degree 3 and position 3 has degree 1.
        let edges = vec![(0, 1), (0, 2), (0, 3), (1, 2)];
        hamiltonian_cycle_to_path(4, &edges);
    }

    #[test]
    #[should_panic(expected = "disjoint cycles")]
    fn hamiltonian_cycle_to_path_panics_on_disjoint_cycles() {
        // Two disjoint triangles: {0,1,2} and {3,4,5}. Every position has degree 2,
        // and there are exactly n=6 edges, but they do not form one Hamiltonian cycle.
        let edges = vec![(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)];
        hamiltonian_cycle_to_path(6, &edges);
    }
}

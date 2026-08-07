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

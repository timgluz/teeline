//! KD-tree k-nearest candidate lists for local search.
//!
//! A candidate list is a pruning heuristic, not a distance source: every candidate move is still
//! scored with the problem's exact distances. Memory is O(n·k), versus O(n²) for a full
//! neighbour table.
//!
//! KD-tree neighbours rank by raw Euclidean distance, which matches EUC_2D but not GEO
//! latitude/longitude. The list is therefore approximate for GEO/ATT/CEIL_2D; exact scoring
//! keeps tours valid, and the quality tests bound the resulting gap.

use super::kdtree::{self, KDPoint};

/// The generic `n_nearest` default (3) is a branch-and-bound beam width, not a useful
/// local-search neighbourhood. Two-opt/or-opt raise it to this floor; a caller can ask for more.
///
/// Measured on a280/att532, 16 keeps or-opt within 1.3% of the full-neighbourhood tour while
/// running about 25x faster; two-opt is no worse than the full scan at the same size.
pub(crate) const MIN_CANDIDATE_K: usize = 16;

pub(crate) fn build(cities: &[KDPoint], k: usize) -> Vec<Vec<usize>> {
    let n = cities.len();
    let k = k.min(n.saturating_sub(1));
    let max_id = cities.iter().map(|c| c.id).max().unwrap_or(0);
    let mut candidates = vec![Vec::new(); max_id + 1];
    if n < 2 || k == 0 {
        return candidates;
    }

    let tree = kdtree::from_cities(cities);
    for city in cities {
        candidates[city.id] = tree
            .nearest(city, k)
            .nearest()
            .iter()
            .map(|item| item.point.id)
            .collect();
    }
    candidates
}

/// City id → route position, with `usize::MAX` for ids not in the route.
pub(crate) fn positions(path: &[usize]) -> Vec<usize> {
    let max_id = *path.iter().max().unwrap_or(&0);
    let mut pos = vec![usize::MAX; max_id + 1];
    for (rank, &id) in path.iter().enumerate() {
        pos[id] = rank;
    }
    pos
}

/// Candidate positions adjacent to either `a` or `b`.
///
/// A 2-opt/or-opt endpoint at position `p` uses either `path[p]` or `path[p + 1]`, so both `p`
/// and `p - 1` are returned. The caller filters its own valid range and forbidden window.
pub(crate) fn candidate_positions(
    candidates: &[Vec<usize>],
    pos: &[usize],
    a: usize,
    b: usize,
) -> Vec<usize> {
    let mut positions = Vec::new();
    for &city in candidates[a].iter().chain(candidates[b].iter()) {
        let p = pos.get(city).copied().unwrap_or(usize::MAX);
        if p == usize::MAX {
            continue;
        }
        positions.push(p);
        if let Some(prev) = p.checked_sub(1) {
            positions.push(prev);
        }
    }
    positions.sort_unstable();
    positions.dedup();
    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_are_distinct_neighbours() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![10.0, 0.0],
            vec![2.0, 0.0],
        ]);
        let candidates = build(&cities, 2);
        assert_eq!(candidates[cities[0].id].len(), 2);
        assert!(!candidates[cities[0].id].contains(&cities[0].id));
        assert!(candidates[cities[0].id].contains(&cities[1].id));
        assert!(candidates[cities[0].id].contains(&cities[3].id));
    }

    #[test]
    fn positions_round_trip() {
        let path = vec![10, 30, 20];
        let pos = positions(&path);
        assert_eq!(pos[10], 0);
        assert_eq!(pos[30], 1);
        assert_eq!(pos[20], 2);
    }
}

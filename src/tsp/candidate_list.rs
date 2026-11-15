//! KD-tree k-nearest candidate lists for local search.
//!
//! A candidate list is a pruning heuristic, not a distance source: every candidate move is still
//! scored with the problem's exact distances. Memory is O(n·k), versus O(n²) for a full
//! neighbour table.
//!
//! KD-tree neighbours rank by raw Euclidean distance, which matches EUC_2D but not GEO
//! latitude/longitude. The list is therefore approximate for GEO/ATT/CEIL_2D; exact scoring
//! keeps tours valid, and the quality tests bound the resulting gap.
//!
//! Rows are indexed by the city's dense position in the problem, never by its external ID:
//! sparse IDs (10, 20, 30, …) would otherwise size the table by the largest ID instead of by
//! the city count.

use std::collections::HashMap;

use super::kdtree::{self, KDPoint};

/// The generic `n_nearest` default (3) is a branch-and-bound beam width, not a useful
/// local-search neighbourhood. Two-opt/or-opt raise it to this floor; a caller can ask for more.
///
/// Measured on a280/att532: 16 keeps or-opt within 1.3% of the full-neighbourhood tour while
/// running 9–21x faster; two-opt is slightly faster and finds no worse tours.
pub(crate) const MIN_CANDIDATE_K: usize = 16;

/// City ID → dense position, using a vector for the usual compact IDs and a map only when the
/// ID range would make that vector disproportionately large.
enum IdIndex {
    Dense(Vec<usize>),
    Sparse(HashMap<usize, usize>),
}

impl IdIndex {
    fn build(cities: &[KDPoint]) -> Self {
        let max_id = cities.iter().map(|c| c.id).max().unwrap_or(0);
        if max_id < cities.len().saturating_mul(4).max(1) {
            let mut dense = vec![usize::MAX; max_id + 1];
            for (pos, city) in cities.iter().enumerate() {
                dense[city.id] = pos;
            }
            Self::Dense(dense)
        } else {
            Self::Sparse(
                cities
                    .iter()
                    .enumerate()
                    .map(|(pos, city)| (city.id, pos))
                    .collect(),
            )
        }
    }

    fn get(&self, city_id: usize) -> Option<usize> {
        match self {
            Self::Dense(positions) => positions
                .get(city_id)
                .copied()
                .filter(|&pos| pos != usize::MAX),
            Self::Sparse(positions) => positions.get(&city_id).copied(),
        }
    }
}

pub(crate) struct CandidateList {
    /// Neighbour city IDs, indexed by the city's dense position in `cities`.
    rows: Vec<Vec<usize>>,
    id_index: IdIndex,
}

impl CandidateList {
    pub(crate) fn build(cities: &[KDPoint], k: usize) -> Self {
        let n = cities.len();
        let k = k.min(n.saturating_sub(1));
        let mut rows = vec![Vec::new(); n];
        let id_index = IdIndex::build(cities);
        if n < 2 || k == 0 {
            return Self { rows, id_index };
        }

        let tree = kdtree::from_cities(cities);
        for (pos, city) in cities.iter().enumerate() {
            rows[pos] = tree
                .nearest(city, k)
                .nearest()
                .iter()
                .map(|item| item.point.id)
                .collect();
        }
        Self { rows, id_index }
    }

    #[cfg(test)]
    pub(crate) fn num_cities(&self) -> usize {
        self.rows.len()
    }

    /// Neighbour city IDs for `city_id`; empty for an ID that is not part of the problem.
    pub(crate) fn neighbours(&self, city_id: usize) -> &[usize] {
        self.id_index
            .get(city_id)
            .map_or(&[], |pos| self.rows[pos].as_slice())
    }

    pub(crate) fn city_position(&self, city_id: usize) -> Option<usize> {
        self.id_index.get(city_id)
    }

    /// Route rank for every dense city position; `usize::MAX` for cities not in `path`.
    pub(crate) fn route_positions(&self, path: &[usize]) -> Vec<usize> {
        let mut positions = vec![usize::MAX; self.rows.len()];
        for (rank, &city_id) in path.iter().enumerate() {
            if let Some(city_pos) = self.id_index.get(city_id) {
                positions[city_pos] = rank;
            }
        }
        positions
    }
}

/// Append candidate route positions adjacent to either endpoint `a` or `b`.
///
/// A 2-opt/or-opt endpoint at route position `p` uses either `path[p]` or `path[p + 1]`, so
/// both `p` and its cyclic predecessor are returned. `n` is the route length, used to map the
/// predecessor of position 0 to the closing position `n - 1`. The caller filters its own valid
/// range and forbidden window.
pub(crate) fn candidate_positions_into(
    candidates: &CandidateList,
    route_positions: &[usize],
    n: usize,
    a: usize,
    b: usize,
    out: &mut Vec<usize>,
) {
    out.clear();
    if n == 0 {
        return;
    }

    for &city in candidates
        .neighbours(a)
        .iter()
        .chain(candidates.neighbours(b))
    {
        let Some(city_pos) = candidates.city_position(city) else {
            continue;
        };
        let p = route_positions.get(city_pos).copied().unwrap_or(usize::MAX);
        if p == usize::MAX {
            continue;
        }
        out.push(p);
        out.push(if p == 0 { n - 1 } else { p - 1 });
    }
    out.sort_unstable();
    out.dedup();
}

#[cfg(test)]
pub(crate) fn candidate_positions(
    candidates: &CandidateList,
    route_positions: &[usize],
    n: usize,
    a: usize,
    b: usize,
) -> Vec<usize> {
    let mut out = Vec::new();
    candidate_positions_into(candidates, route_positions, n, a, b, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cities_at(ids: &[usize]) -> Vec<KDPoint> {
        ids.iter()
            .enumerate()
            .map(|(i, &id)| KDPoint::new_with_id(id, &[i as f32, 0.0]))
            .collect()
    }

    #[test]
    fn candidates_are_distinct_neighbours() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![10.0, 0.0],
            vec![2.0, 0.0],
        ]);
        let candidates = CandidateList::build(&cities, 2);
        let row = candidates.neighbours(cities[0].id);
        assert_eq!(row.len(), 2);
        assert!(!row.contains(&cities[0].id));
        assert!(row.contains(&cities[1].id));
        assert!(row.contains(&cities[3].id));
    }

    #[test]
    fn rows_are_indexed_by_dense_position() {
        let cities = cities_at(&[10, 20, 30, 40]);
        let candidates = CandidateList::build(&cities, 2);

        assert_eq!(candidates.num_cities(), 4);
        assert_eq!(candidates.city_position(10), Some(0));
        assert_eq!(candidates.city_position(40), Some(3));
        assert!(candidates.neighbours(99).is_empty());
        assert_eq!(candidates.route_positions(&[30, 10, 40, 20]), [1, 3, 0, 2]);
    }

    #[test]
    fn sparse_ids_do_not_size_the_table_by_the_largest_id() {
        let cities = cities_at(&[1_000_000, 2_000_000, 3_000_000]);
        let candidates = CandidateList::build(&cities, 2);

        assert_eq!(candidates.num_cities(), 3);
        assert_eq!(candidates.city_position(1_000_000), Some(0));
        assert_eq!(
            candidates.route_positions(&[3_000_000, 1_000_000, 2_000_000]),
            [1, 2, 0]
        );
    }

    #[test]
    fn candidate_positions_wrap_the_predecessor_of_position_zero() {
        // City ids 0..4 at increasing x. Route starts at city 2, so city 2 is route position
        // 0 and its cyclic predecessor is route position 4.
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![2.0, 0.0],
            vec![3.0, 0.0],
            vec![4.0, 0.0],
        ]);
        let candidates = CandidateList::build(&cities, 2);
        let path = vec![2, 0, 1, 3, 4];
        let route_positions = candidates.route_positions(&path);

        let positions = candidate_positions(&candidates, &route_positions, path.len(), 0, 1);
        assert!(positions.contains(&0), "candidate at route 0 is missing");
        assert!(
            positions.contains(&4),
            "the wrap-around predecessor of route 0 is missing"
        );
    }

    #[test]
    fn candidate_positions_dedup_overlapping_rows_and_skip_unknown_cities() {
        let cities = cities_at(&[10, 20, 30, 40]);
        let candidates = CandidateList::build(&cities, 3);
        let path = vec![10, 20, 30, 40];
        let route_positions = candidates.route_positions(&path);

        let mut out = Vec::new();
        candidate_positions_into(&candidates, &route_positions, path.len(), 10, 20, &mut out);
        let mut sorted = out.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(out, sorted, "positions must be sorted and deduplicated");
        assert!(out.iter().all(|&p| p < path.len()));

        let mut empty = Vec::new();
        candidate_positions_into(
            &candidates,
            &route_positions,
            path.len(),
            999,
            998,
            &mut empty,
        );
        assert!(empty.is_empty());
    }

    #[test]
    fn candidate_positions_handles_an_empty_route() {
        let cities = cities_at(&[10, 20]);
        let candidates = CandidateList::build(&cities, 1);
        let mut out = vec![7];
        candidate_positions_into(&candidates, &[], 0, 10, 20, &mut out);
        assert!(out.is_empty());
    }
}

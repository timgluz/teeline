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

use super::Position;
use super::distance_source::DistanceSource;
use super::id_index::IdIndex;
use super::kdtree::{self, KDPoint};

/// The generic `n_nearest` default (3) is a branch-and-bound beam width, not a useful
/// local-search neighbourhood. Two-opt/or-opt raise it to this floor; a caller can ask for more.
///
/// Measured on a280/att532: 16 keeps or-opt within 1.3% of the full-neighbourhood tour while
/// running 9–21x faster; two-opt is slightly faster and finds no worse tours.
pub(crate) const MIN_CANDIDATE_K: usize = 16;

pub(crate) struct CandidateList {
    /// Neighbour positions, indexed by the city's dense position in `cities`.
    rows: Vec<Vec<Position>>,
    id_index: IdIndex,
}

impl CandidateList {
    pub(crate) fn build(cities: &[KDPoint], k: usize) -> Result<Self, &'static str> {
        let n = cities.len();
        let k = k.min(n.saturating_sub(1));
        let mut rows = vec![Vec::new(); n];
        let id_index = IdIndex::build(cities)?;
        if n < 2 || k == 0 {
            return Ok(Self { rows, id_index });
        }

        let tree = kdtree::from_cities(cities);
        for (pos, city) in cities.iter().enumerate() {
            let result = tree.nearest(city, k);
            let row: Vec<Position> = result
                .nearest()
                .iter()
                .map(|item| {
                    Position::new(
                        id_index
                            .get(item.point.id)
                            .expect("every KD-tree point must be in the city index"),
                    )
                })
                .collect();
            rows[pos] = row;
        }
        Ok(Self { rows, id_index })
    }

    /// Build candidate rows from the exact distance matrix rather than the coordinates.
    ///
    /// EXPLICIT instances may have no coordinates, or coordinates that are display data rather
    /// than the objective. Ranking by `distance_by_pos` keeps the neighbourhood tied to the
    /// distances the solver is scored on; non-finite pairs are skipped.
    pub(crate) fn build_from_distances(
        distances: &dyn DistanceSource,
        cities: &[KDPoint],
        k: usize,
    ) -> Result<Self, &'static str> {
        let n = cities.len();
        debug_assert_eq!(
            distances.num_cities(),
            n,
            "distance source and city list disagree on the city count"
        );
        let k = k.min(n.saturating_sub(1));
        let mut rows = vec![Vec::new(); n];
        let id_index = IdIndex::build(cities)?;
        if n < 2 || k == 0 {
            return Ok(Self { rows, id_index });
        }

        for (pos, row) in rows.iter_mut().enumerate() {
            // Keep the city-ID tie-break from the ID-storing version; resolve the position
            // only after sorting so the candidate order is unchanged.
            let mut neighbours: Vec<(f32, usize, Position)> = (0..n)
                .filter(|&other| other != pos)
                .filter_map(|other| {
                    let distance = distances
                        .distance_by_pos(Position::new(pos), Position::new(other))
                        .ok()?;
                    if !distance.is_finite() {
                        return None;
                    }
                    Some((distance, cities[other].id, Position::new(other)))
                })
                .collect();
            neighbours.sort_by(|left, right| {
                left.0
                    .total_cmp(&right.0)
                    .then_with(|| left.1.cmp(&right.1))
            });
            neighbours.truncate(k);
            *row = neighbours.into_iter().map(|(_, _, other)| other).collect();
        }

        Ok(Self { rows, id_index })
    }

    #[cfg(test)]
    pub(crate) fn num_cities(&self) -> usize {
        self.rows.len()
    }

    /// Neighbour positions for `pos`; empty for a position outside the problem.
    pub(crate) fn neighbours_by_pos(&self, pos: Position) -> &[Position] {
        self.rows
            .get(pos.into_inner())
            .map_or(&[], |row| row.as_slice())
    }

    pub(crate) fn city_position(&self, city_id: usize) -> Option<Position> {
        self.id_index.get(city_id).map(Position::new)
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

    let rows_a: &[Position] = candidates
        .city_position(a)
        .map_or(&[], |pos| candidates.neighbours_by_pos(pos));
    let rows_b: &[Position] = candidates
        .city_position(b)
        .map_or(&[], |pos| candidates.neighbours_by_pos(pos));

    for &city_pos in rows_a.iter().chain(rows_b) {
        let p = route_positions
            .get(city_pos.into_inner())
            .copied()
            .unwrap_or(usize::MAX);
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
    use crate::tsp::distance_matrix::DistanceMatrix;

    fn cities_at(ids: &[usize]) -> Vec<KDPoint> {
        ids.iter()
            .enumerate()
            .map(|(i, &id)| KDPoint::new_with_id(id, &[i as f32, 0.0]))
            .collect()
    }

    fn explicit_matrix(distances: Vec<f32>, cities: &[KDPoint]) -> DistanceMatrix {
        DistanceMatrix::new(cities.len(), distances, cities.to_vec()).unwrap()
    }

    #[test]
    fn matrix_candidates_rank_by_distance_not_geometry() {
        let cities = cities_at(&[1, 2, 3, 4]);
        // Geometry puts city 2 next to city 1; the matrix makes city 4 closest.
        let matrix = explicit_matrix(vec![10.0, 20.0, 1.0, 1.0, 30.0, 30.0], &cities);

        let geometric = CandidateList::build(&cities, 1).expect("unique city ids");
        assert_eq!(
            geometric.neighbours_by_pos(Position::new(0)),
            &[Position::new(1)]
        );

        let exact =
            CandidateList::build_from_distances(&matrix, &cities, 1).expect("unique city ids");
        assert_eq!(
            exact.neighbours_by_pos(Position::new(0)),
            &[Position::new(3)]
        );
    }

    #[test]
    fn matrix_candidates_skip_non_finite_distances() {
        let cities = cities_at(&[1, 2, 3, 4]);
        let matrix = explicit_matrix(vec![f32::NAN, 20.0, 1.0, 1.0, 30.0, 30.0], &cities);

        let exact =
            CandidateList::build_from_distances(&matrix, &cities, 2).expect("unique city ids");
        assert_eq!(
            exact.neighbours_by_pos(Position::new(0)),
            &[Position::new(3), Position::new(2)]
        );
    }

    #[test]
    fn candidates_are_distinct_neighbours() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![10.0, 0.0],
            vec![2.0, 0.0],
        ]);
        let candidates = CandidateList::build(&cities, 2).expect("unique city ids");
        let row = candidates.neighbours_by_pos(Position::new(0));
        assert_eq!(row.len(), 2);
        assert!(!row.contains(&Position::new(0)));
        assert!(row.contains(&Position::new(1)));
        assert!(row.contains(&Position::new(3)));
    }

    #[test]
    fn rows_are_indexed_by_dense_position() {
        let cities = cities_at(&[10, 20, 30, 40]);
        let candidates = CandidateList::build(&cities, 2).expect("unique city ids");

        assert_eq!(candidates.num_cities(), 4);
        assert_eq!(candidates.city_position(10), Some(Position::new(0)));
        assert_eq!(candidates.city_position(40), Some(Position::new(3)));
        assert!(candidates.neighbours_by_pos(Position::new(99)).is_empty());
        assert_eq!(candidates.route_positions(&[30, 10, 40, 20]), [1, 3, 0, 2]);
    }

    #[test]
    fn sparse_ids_do_not_size_the_table_by_the_largest_id() {
        let cities = cities_at(&[1_000_000, 2_000_000, 3_000_000]);
        let candidates = CandidateList::build(&cities, 2).expect("unique city ids");

        assert_eq!(candidates.num_cities(), 3);
        assert_eq!(candidates.city_position(1_000_000), Some(Position::new(0)));
        // Rows hold dense positions, never the external IDs.
        assert_eq!(
            candidates.neighbours_by_pos(Position::new(0)),
            &[Position::new(1), Position::new(2)]
        );
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
        let candidates = CandidateList::build(&cities, 2).expect("unique city ids");
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
        let candidates = CandidateList::build(&cities, 3).expect("unique city ids");
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
        let candidates = CandidateList::build(&cities, 1).expect("unique city ids");
        let mut out = vec![7];
        candidate_positions_into(&candidates, &[], 0, 10, 20, &mut out);
        assert!(out.is_empty());
    }
}

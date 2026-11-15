//! The distance-query boundary.
//!
//! Positions are dense `0..n` indices into the problem's city list; city IDs are external
//! identifiers (TSPLIB 1-based, API-supplied, possibly non-contiguous). `distance_by_pos`,
//! `point_at`, `pos2city_id`, and `city_id2pos` are the conversion boundary;
//! `distance_between`/`tour_length` accept IDs.
//!
//! Every backend implements only the five primitives. The method defaults below are shared, so
//! two backends cannot drift in summation order, id handling, or tie-breaking — the property the
//! byte-identical guarantee rests on.

use super::kdtree::KDPoint;
use super::{NearestResult, NearestResultItem};

pub trait DistanceSource: Send + Sync {
    fn num_cities(&self) -> usize;
    fn distance_by_pos(&self, pos1: usize, pos2: usize) -> Result<f32, &'static str>;
    fn pos2city_id(&self, pos: usize) -> Option<usize>;
    fn city_id2pos(&self, city_id: usize) -> Option<usize>;
    fn point_at(&self, pos: usize) -> Option<KDPoint>;

    /// False when the cities are TSPLIB EXPLICIT placeholders rather than real coordinates.
    fn has_coordinates(&self) -> bool {
        true
    }

    fn distance_between(&self, city_id1: usize, city_id2: usize) -> Result<f32, &'static str> {
        if city_id1 == city_id2 {
            return Ok(0.0);
        }
        let pos1 = self.city_id2pos(city_id1).ok_or("city_id1 not in index")?;
        let pos2 = self.city_id2pos(city_id2).ok_or("city_id2 not in index")?;
        self.distance_by_pos(pos1, pos2)
    }

    /// Returns the full row in position order, including the zero at the city's own position.
    fn distances_from(&self, city_id: usize) -> Vec<f32> {
        let pos = self.city_id2pos(city_id).expect("Unknown city id");
        let n = self.num_cities();
        let mut distances = Vec::with_capacity(n);
        for i in 0..n {
            distances.push(
                self.distance_by_pos(pos, i)
                    .expect("valid position in distances_from"),
            );
        }
        distances
    }

    fn tour_length(&self, path: &[usize]) -> f32 {
        if path.len() < 2 {
            return 0.0;
        }
        // Translate city IDs to positions upfront; unknown city ID → return 0.0.
        let positions: Option<Vec<usize>> = path.iter().map(|&id| self.city_id2pos(id)).collect();
        match positions {
            None => 0.0,
            Some(pos_path) => self.tour_length_by_pos(&pos_path),
        }
    }

    /// Position-based tour length — no id lookup per edge.
    fn tour_length_by_pos(&self, path: &[usize]) -> f32 {
        if path.len() < 2 {
            return 0.0;
        }
        let last = *path.last().unwrap();
        let mut total = self.distance_by_pos(last, path[0]).unwrap_or(0.0);
        for w in path.windows(2) {
            total += self.distance_by_pos(w[0], w[1]).unwrap_or(0.0);
        }
        total
    }

    fn nearest(&self, target: &KDPoint, n: usize) -> NearestResult {
        let mut search_result = NearestResult::new(*target, n);

        let city_pos = match self.city_id2pos(target.id) {
            Some(pos) => pos,
            None => return search_result,
        };

        for pos in 0..self.num_cities() {
            if pos == city_pos {
                continue;
            }
            let Some(pt) = self.point_at(pos) else {
                continue;
            };
            let Ok(distance) = self.distance_by_pos(city_pos, pos) else {
                continue;
            };
            search_result.add(pt, distance);
        }

        search_result
    }

    fn nearest_unvisited(
        &self,
        target: &KDPoint,
        is_candidate: &dyn Fn(usize) -> bool,
    ) -> Option<NearestResultItem> {
        let city_pos = self.city_id2pos(target.id)?;

        let mut best: Option<NearestResultItem> = None;
        for pos in 0..self.num_cities() {
            if pos == city_pos {
                continue;
            }
            let Some(pt) = self.point_at(pos) else {
                continue;
            };
            if !is_candidate(pt.id) {
                continue;
            }
            let Ok(distance) = self.distance_by_pos(city_pos, pos) else {
                continue;
            };
            let better = match &best {
                None => true,
                Some(current) => distance
                    .total_cmp(&current.distance)
                    .then_with(|| pt.id.cmp(&current.point.id))
                    .is_lt(),
            };
            if better {
                best = Some(NearestResultItem::new(pt, distance));
            }
        }

        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::DistanceType;
    use crate::tsp::distance_matrix::DistanceMatrix;
    use crate::tsp::kdtree;

    /// The skeleton the full conformance suite grows from: exercise every default through a
    /// `dyn` reference so object safety is part of the test, not an assumption.
    fn assert_skeleton(source: &dyn DistanceSource, n: usize) {
        assert_eq!(source.num_cities(), n);
        assert_eq!(source.distance_between(0, 0).unwrap(), 0.0);
        let tour: Vec<usize> = (0..n).collect();
        assert!(source.tour_length(&tour) > 0.0);
        assert!(source.nearest(&source.point_at(0).unwrap(), 1).n == 1);
    }

    #[test]
    fn dyn_distance_source_dispatches() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]]);
        let dm = DistanceMatrix::build(&cities, DistanceType::Euc2D).unwrap();
        assert_skeleton(&dm, 3);
    }
}

//! The distance-query boundary.
//!
//! Positions are dense `0..n` indices (typed as [`Position`]) into the problem's city list;
//! city IDs are external identifiers (TSPLIB 1-based, API-supplied, possibly non-contiguous).
//! `distance_by_pos`, `point_at`, `pos2city_id`, and `city_id2pos` are the conversion boundary;
//! `distance_between`/`tour_length` accept IDs.
//!
//! Every backend implements only the five primitives. The method defaults below are shared, so
//! two backends cannot drift in summation order, id handling, or tie-breaking — the property the
//! byte-identical guarantee rests on.

use super::Position;
use super::kdtree::KDPoint;
use super::{NearestResult, NearestResultItem};

pub trait DistanceSource: Send + Sync {
    fn num_cities(&self) -> usize;
    fn distance_by_pos(&self, pos1: Position, pos2: Position) -> Result<f32, &'static str>;
    fn pos2city_id(&self, pos: Position) -> Option<usize>;
    fn city_id2pos(&self, city_id: usize) -> Option<Position>;
    fn point_at(&self, pos: Position) -> Option<KDPoint>;

    fn is_coordinate_metric(&self) -> bool {
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
    ///
    /// # Panics
    ///
    /// Panics if `city_id` is not in the source. This preserves the pre-trait
    /// `DistanceMatrix::distances_from` contract; callers that may hold an unknown id should
    /// check `city_id2pos` first.
    fn distances_from(&self, city_id: usize) -> Vec<f32> {
        let pos = self.city_id2pos(city_id).expect("Unknown city id");
        let n = self.num_cities();
        let mut distances = Vec::with_capacity(n);
        for i in 0..n {
            distances.push(
                self.distance_by_pos(pos, Position::new(i))
                    .expect("valid position in distances_from"),
            );
        }
        distances
    }

    fn tour_length(&self, path: &[usize]) -> f32 {
        if path.len() < 2 {
            return 0.0;
        }
        // Single pass, same order as `tour_length_by_pos`: closing edge first, then the
        // consecutive edges. Keeping it allocation-free matters because solvers call this
        // inside candidate loops (`tabu_search::select`, simulated annealing, ...).
        let Some(first) = self.city_id2pos(path[0]) else {
            return 0.0;
        };
        let Some(last) = self.city_id2pos(*path.last().unwrap()) else {
            return 0.0;
        };
        // f64 accumulation: the f32 sum of ~1e3 edge lengths loses the last bits at
        // large-instance costs (the dsj1000 sum drifted by ~10), which makes a reported
        // cost or gap disagree with the same tour re-summed in a different order.
        let mut total = f64::from(self.distance_by_pos(last, first).unwrap_or(0.0));
        let mut prev = first;
        for &id in &path[1..path.len() - 1] {
            let Some(next) = self.city_id2pos(id) else {
                return 0.0;
            };
            total += f64::from(self.distance_by_pos(prev, next).unwrap_or(0.0));
            prev = next;
        }
        total += f64::from(self.distance_by_pos(prev, last).unwrap_or(0.0));
        total as f32
    }

    /// Position-based tour length — no id lookup per edge.
    fn tour_length_by_pos(&self, path: &[Position]) -> f32 {
        if path.len() < 2 {
            return 0.0;
        }
        let last = *path.last().unwrap();
        let mut total = f64::from(self.distance_by_pos(last, path[0]).unwrap_or(0.0));
        for w in path.windows(2) {
            total += f64::from(self.distance_by_pos(w[0], w[1]).unwrap_or(0.0));
        }
        total as f32
    }

    fn nearest(&self, target: &KDPoint, n: usize) -> NearestResult {
        let mut search_result = NearestResult::new(*target, n);

        let city_pos = match self.city_id2pos(target.id) {
            Some(pos) => pos,
            None => return search_result,
        };

        for i in 0..self.num_cities() {
            let pos = Position::new(i);
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
        for i in 0..self.num_cities() {
            let pos = Position::new(i);
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
        assert!(source.is_coordinate_metric());
        assert_eq!(source.distance_between(0, 0).unwrap(), 0.0);
        assert_eq!(source.distances_from(0).len(), n);
        let id_tour: Vec<usize> = (0..n).collect();
        let pos_tour: Vec<Position> = (0..n).map(Position::new).collect();
        assert!(source.tour_length(&id_tour) > 0.0);
        assert_eq!(
            source.tour_length_by_pos(&pos_tour),
            source.tour_length(&id_tour)
        );
        let target = source.point_at(Position::new(0)).unwrap();
        assert_eq!(source.nearest(&target, 1).nearest().len(), 1);
        assert!(
            source
                .nearest_unvisited(&target, &|id| id != target.id)
                .is_some()
        );
    }

    /// A source where every edge has the same cost, so a tour's exact length is
    /// `n * distance` and any drift comes purely from accumulation.
    struct ConstantDistance {
        n: usize,
        distance: f32,
    }

    impl DistanceSource for ConstantDistance {
        fn num_cities(&self) -> usize {
            self.n
        }
        fn distance_by_pos(&self, _: Position, _: Position) -> Result<f32, &'static str> {
            Ok(self.distance)
        }
        fn pos2city_id(&self, pos: Position) -> Option<usize> {
            (pos.into_inner() < self.n).then_some(pos.into_inner())
        }
        fn city_id2pos(&self, id: usize) -> Option<Position> {
            (id < self.n).then_some(Position::new(id))
        }
        fn point_at(&self, _: Position) -> Option<KDPoint> {
            Some(KDPoint::new_with_id(0, &[0.0, 0.0]))
        }
    }

    /// The f32 sum of 1,000 large edges drifts by ~10 from the exact f64 sum, which is what
    /// made dsj1000 report 18,660,178 instead of its exact 18,660,188.
    #[test]
    fn tour_length_accumulates_in_f64() {
        let source = ConstantDistance {
            n: 1_000,
            distance: 18_660.188,
        };
        let path: Vec<usize> = (0..source.n).collect();
        let expected = (source.n as f64 * f64::from(source.distance)) as f32;
        assert_eq!(source.tour_length(&path), expected);

        let positions: Vec<Position> = (0..source.n).map(Position::new).collect();
        assert_eq!(source.tour_length_by_pos(&positions), expected);
    }

    #[test]
    fn dyn_distance_source_dispatches() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]]);
        let dm = DistanceMatrix::build(&cities, DistanceType::Euc2D).unwrap();
        assert_skeleton(&dm, 3);
    }
}

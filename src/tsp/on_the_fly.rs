//! The coordinate-computed distance backend.
//!
//! `OnTheFly` keeps only the city coordinates, an id→position index, and a function pointer to
//! the TSPLIB primitive. Every lookup computes the distance, so the source needs O(n) memory
//! instead of the matrix's O(n²). It is the backend that makes 100k-city instances possible.

use super::distance_matrix::{att_distance, ceil_2d_distance, euc_2d_distance, geo_distance};
use super::id_index::IdIndex;
use super::kdtree::KDPoint;
use super::{DistanceSource, DistanceType};

pub struct OnTheFly {
    cities: Vec<KDPoint>, // position-indexed, same order as TspProblem.cities
    city_idx: IdIndex,
    distance: fn(&KDPoint, &KDPoint) -> f32,
}

impl OnTheFly {
    /// Build a coordinate-computed source for one of the four TSPLIB coordinate metrics.
    ///
    /// EXPLICIT is rejected: those distances are already materialized in the file, and parsing
    /// only supplies placeholder grid coordinates that would produce meaningless results.
    pub fn new(cities: &[KDPoint], dt: DistanceType) -> Result<Self, &'static str> {
        if cities.len() < 2 {
            return Err("on-the-fly distance source requires at least 2 points");
        }
        let distance = match dt {
            DistanceType::Euc2D => euc_2d_distance,
            DistanceType::Geo => geo_distance,
            DistanceType::Att => att_distance,
            DistanceType::Ceil2D => ceil_2d_distance,
            DistanceType::Explicit => {
                return Err("cannot compute EXPLICIT distances on the fly; use the matrix backend");
            }
        };
        let city_idx = IdIndex::build(cities)?;
        Ok(OnTheFly {
            cities: cities.to_vec(),
            city_idx,
            distance,
        })
    }
}

impl DistanceSource for OnTheFly {
    fn num_cities(&self) -> usize {
        self.cities.len()
    }

    fn distance_by_pos(&self, pos1: usize, pos2: usize) -> Result<f32, &'static str> {
        if pos1 == pos2 {
            return Ok(0.0);
        }
        let a = self.cities.get(pos1).ok_or("position out of range")?;
        let b = self.cities.get(pos2).ok_or("position out of range")?;
        Ok((self.distance)(a, b))
    }

    /// Resolve the dense index once per lookup instead of paying the trait default's two
    /// `city_id2pos` calls; BHK and LK call this in their inner loops.
    fn distance_between(&self, city_id1: usize, city_id2: usize) -> Result<f32, &'static str> {
        if city_id1 == city_id2 {
            return Ok(0.0);
        }
        let pos1 = self.city_idx.get(city_id1).ok_or("city_id1 not in index")?;
        let pos2 = self.city_idx.get(city_id2).ok_or("city_id2 not in index")?;
        self.distance_by_pos(pos1, pos2)
    }

    fn pos2city_id(&self, pos: usize) -> Option<usize> {
        self.cities.get(pos).map(|city| city.id)
    }

    fn city_id2pos(&self, city_id: usize) -> Option<usize> {
        self.city_idx.get(city_id)
    }

    fn point_at(&self, pos: usize) -> Option<KDPoint> {
        self.cities.get(pos).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::kdtree;

    fn cities() -> Vec<KDPoint> {
        kdtree::build_points(&[vec![0.0, 0.0], vec![3.0, 4.0], vec![0.0, 1.0]])
    }

    #[test]
    fn accepts_every_coordinate_type() {
        for dt in [
            DistanceType::Euc2D,
            DistanceType::Geo,
            DistanceType::Att,
            DistanceType::Ceil2D,
        ] {
            OnTheFly::new(&cities(), dt).expect("coordinate type must be accepted");
        }
    }

    #[test]
    fn rejects_explicit_and_small_inputs() {
        assert!(OnTheFly::new(&cities(), DistanceType::Explicit).is_err());
        assert!(OnTheFly::new(&cities()[..1], DistanceType::Euc2D).is_err());
    }

    #[test]
    fn rejects_duplicate_city_ids() {
        let duplicate = vec![
            KDPoint::new_with_id(1, &[0.0, 0.0]),
            KDPoint::new_with_id(1, &[3.0, 4.0]),
        ];
        assert!(OnTheFly::new(&duplicate, DistanceType::Euc2D).is_err());
    }

    #[test]
    fn self_distance_is_zero_and_positions_round_trip() {
        let source = OnTheFly::new(&cities(), DistanceType::Euc2D).unwrap();
        assert_eq!(source.distance_by_pos(1, 1).unwrap(), 0.0);
        for pos in 0..source.num_cities() {
            let id = source.pos2city_id(pos).unwrap();
            assert_eq!(source.city_id2pos(id), Some(pos));
            assert_eq!(source.point_at(pos).unwrap().id, id);
        }
        assert!(source.distance_by_pos(0, 99).is_err());
    }

    #[test]
    fn sparse_ids_use_the_map_index() {
        let cities = vec![
            KDPoint::new_with_id(10, &[0.0, 0.0]),
            KDPoint::new_with_id(20, &[3.0, 4.0]),
            KDPoint::new_with_id(30, &[0.0, 1.0]),
        ];
        let source = OnTheFly::new(&cities, DistanceType::Euc2D).unwrap();
        assert_eq!(source.city_id2pos(20), Some(1));
        assert_eq!(source.distance_between(10, 20).unwrap(), 5.0);
        assert!(source.distance_between(10, 99).is_err());
    }

    #[test]
    fn computes_the_declared_metric() {
        let source = OnTheFly::new(&cities(), DistanceType::Euc2D).unwrap();
        assert_eq!(source.distance_by_pos(0, 1).unwrap(), 5.0); // 3-4-5
        assert_eq!(source.distance_by_pos(0, 1).unwrap(), 5.0);
    }
}

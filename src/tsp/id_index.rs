//! City-ID → dense-position lookup.
//!
//! Most TSPLIB files number cities densely (`1..=n`), where a vector is the cheapest index.
//! Sparse external IDs (10, 20, 30, …) would make that vector scale with the largest ID instead
//! of the city count, so those fall back to a map.

use std::collections::HashMap;

use super::kdtree::KDPoint;

pub(crate) enum IdIndex {
    Dense(Vec<usize>),
    Sparse(HashMap<usize, usize>),
}

impl IdIndex {
    pub(crate) fn build(cities: &[KDPoint]) -> Self {
        let max_id = cities.iter().map(|city| city.id).max().unwrap_or(0);
        if max_id < cities.len().saturating_mul(4).max(1) {
            let mut dense = vec![usize::MAX; max_id + 1];
            for (position, city) in cities.iter().enumerate() {
                dense[city.id] = position;
            }
            Self::Dense(dense)
        } else {
            Self::Sparse(
                cities
                    .iter()
                    .enumerate()
                    .map(|(position, city)| (city.id, position))
                    .collect(),
            )
        }
    }

    pub(crate) fn get(&self, city_id: usize) -> Option<usize> {
        match self {
            Self::Dense(positions) => positions
                .get(city_id)
                .copied()
                .filter(|&position| position != usize::MAX),
            Self::Sparse(positions) => positions.get(&city_id).copied(),
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Dense(positions) => positions.iter().filter(|&&pos| pos != usize::MAX).count(),
            Self::Sparse(positions) => positions.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cities(ids: &[usize]) -> Vec<KDPoint> {
        ids.iter()
            .map(|&id| KDPoint::new_with_id(id, &[0.0, 0.0]))
            .collect()
    }

    #[test]
    fn empty_input_is_an_empty_dense_index() {
        let index = IdIndex::build(&[]);
        assert!(matches!(index, IdIndex::Dense(_)));
        assert_eq!(index.len(), 0);
        assert_eq!(index.get(0), None);
    }

    #[test]
    fn chooses_dense_at_the_boundary() {
        // n = 3, so the threshold is max_id < 12.
        let dense = IdIndex::build(&cities(&[0, 5, 11]));
        assert!(matches!(dense, IdIndex::Dense(_)));
        assert_eq!(dense.len(), 3);

        let sparse = IdIndex::build(&cities(&[0, 5, 12]));
        assert!(matches!(sparse, IdIndex::Sparse(_)));
        assert_eq!(sparse.len(), 3);
    }

    #[test]
    fn dense_index_handles_holes_and_the_last_position() {
        let index = IdIndex::build(&cities(&[0, 2, 5]));
        assert!(matches!(index, IdIndex::Dense(_)));
        assert_eq!(index.len(), 3);
        assert_eq!(index.get(1), None);
        assert_eq!(index.get(2), Some(1));
        assert_eq!(index.get(5), Some(2));
        assert_eq!(index.get(6), None);
    }

    #[test]
    fn sparse_index_handles_usize_max_without_allocating_by_id() {
        let index = IdIndex::build(&cities(&[0, usize::MAX]));
        assert!(matches!(index, IdIndex::Sparse(_)));
        assert_eq!(index.len(), 2);
        assert_eq!(index.get(usize::MAX), Some(1));
        assert_eq!(index.get(1), None);
    }

    #[test]
    fn duplicate_ids_collapse_in_both_modes() {
        let dense = IdIndex::build(&cities(&[1, 1]));
        assert!(matches!(dense, IdIndex::Dense(_)));
        assert_eq!(
            dense.len(),
            1,
            "duplicates must not pass the uniqueness check"
        );

        let sparse = IdIndex::build(&cities(&[10, 10]));
        assert!(matches!(sparse, IdIndex::Sparse(_)));
        assert_eq!(sparse.len(), 1);
    }
}

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

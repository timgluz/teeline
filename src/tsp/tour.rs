use std::collections::HashMap;

use crate::tsp::kdtree::{self, KDPoint};

pub type CityTable = HashMap<usize, KDPoint>;

pub fn total_distance(cities: &[KDPoint], route: &[usize]) -> f32 {
    let mut total = 0.0;
    let last_idx = route.len() - 1;

    let cities_table = city_table_from_vec(cities);
    for i in 0..last_idx {
        let distance = cities_table[&route[i]].distance(&cities_table[&route[i + 1]]);
        total += distance
    }

    total += cities_table[&route[last_idx]].distance(&cities_table[&route[0]]);

    total
}

pub fn city_table_from_vec(cities: &[kdtree::KDPoint]) -> CityTable {
    let table: CityTable = cities.iter().map(|c| (c.id, c.clone())).collect();

    return table;
}

pub struct Tour {
    pub total: f32,
    route: Vec<usize>,
    cities: Vec<KDPoint>, //todo: cities: CityTable,
}

impl Tour {
    pub fn new(route: &[usize], cities: &[kdtree::KDPoint]) -> Self {
        let mut tour = Tour {
            total: 0.0,
            route: route.to_vec(),
            cities: cities.to_vec(),
            // TODO: keep it as hash-map because Point.Id may not start from 0
            //cities: cities.iter().map(|c| (c.id, c.clone())).collect(),
        };

        tour.update_total();

        tour
    }

    pub fn len(&self) -> usize {
        self.route.len()
    }

    pub fn route(&self) -> &[usize] {
        self.route[..].as_ref()
    }

    pub fn cities(&self) -> &[kdtree::KDPoint] {
        &self.cities[..]
    }

    pub fn update_total(&mut self) {
        self.total = total_distance(self.cities(), self.route());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::assert_approx;

    #[test]
    fn total_distance_for_tsp_5_1() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);

        let route = vec![0, 1, 2, 3, 4];

        assert_approx(4.0, total_distance(&cities, &route));
    }
}

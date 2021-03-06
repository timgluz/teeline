/*
    DistanceMatrix is a data collection for keeping euclidean distances between array of 2D points;

    Given the fact the teeline only tackles symmetrical TSP problems with only positive distances.
    We can optimize the memory footprint by just keeping triangle under main diagonal,
    and then flatten it into 1D array;

    Visual representation with 2 cities:
    step1: we have a full 3x3 matrix
    +---+---+---+
    |1_1|1_2|1_3|
    +---+---+---+
    |2_1|2_2|2_3|
    +---+---|---|
    |3_1|3_2|3_3|
    +---+---+---+

    step2: we take only items under main diagonal
    +---+
    |2_1|
    +---+---+
    |3_1|3_2|
    +---+---+

    step3: we flatten it into 1D matrix

    +---+---+---+
    |2_1|3_1|3_2|
    +---+---+---+

    As result, we have a array with  N_cities * ( N_cities - 1) / 2 elements

    *Lookup logic*
    As you probably noticed, that this represention has limitation:
        the first city id must be bigger than second city id;

    from_id = max(city1_id, city2_id)
    to_city = min(city1_id, city2_id)

    Then the lookup would calculate a padding before the from_id, which is
    the size of small triangle top of from_id; and then we add the to_city to the padding;

    Example:
        city1 = 3, city2 = 2;
        prev_city = city1 - 1
        padding = (2-1) * 2 / 2 = 1
        pos = padding + city2 = 3
        distance[pos-1] // as Rust start counting from 0

    ps: this high-complexity doesnt make anysense outside this hobby project,
    because it adds more complexity than any actual benefits;
*/

use std::collections::HashMap;

use super::kdtree::KDPoint;
use super::{CityTable, NearestResult};

// to have similar builder as kdtree
pub fn from_cities(cities: &[KDPoint]) -> DistanceMatrix {
    DistanceMatrix::from_cities(cities).unwrap()
}

#[derive(Debug, Clone)]
pub struct DistanceMatrix {
    first_id: usize, // TODO: remove - the first city id, default 0
    n: usize,        // how many cities
    size: usize,     // how many distances under diagonal
    items: Vec<f32>,
    cities: CityTable,
    city_idx: HashMap<usize, usize>, // translates city_id to matrix_id
}

impl DistanceMatrix {
    pub fn new(first_id: usize, n: usize, distances: Vec<f32>, cities: CityTable) -> Self {
        let city_idx: HashMap<usize, usize> =
            cities.iter().map(|(k, c)| (c.id, k.clone())).collect();

        assert!(n == city_idx.len(), "city_idx size differs from n cities");
        DistanceMatrix {
            first_id,              // TODO: remove
            n,                     // how many cities
            size: distances.len(), // how many distances under diagonal
            items: distances,
            cities,
            city_idx,
        }
    }

    // assumes that cities are already sorted by id and ids are incrementally crowing
    pub fn from_cities(cities: &[KDPoint]) -> Result<Self, &'static str> {
        let n = cities.len();
        if n < 2 {
            return Err("distance matrix requires at least 2 points");
        }

        let size = n * (n - 1) / 2; // how many items on distance vec

        let mut city_table = CityTable::new();

        let mut distances = Vec::with_capacity(size);
        for i in 0..n {
            let pt1 = &cities[i];
            city_table.insert(i, pt1.clone());

            for j in 0..i {
                let pt2 = &cities[j];
                distances.push(pt1.distance(pt2));
            }
        }

        distances.shrink_to_fit();

        Ok(DistanceMatrix::new(0, n, distances, city_table))
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn distances(&self) -> &[f32] {
        &self.items
    }

    // It returns distance by raw vector ids for backward support for some solutions
    // preferred solution: distance_between as it checks if city exists on table
    pub fn distance_by_pos(&self, pos1: usize, pos2: usize) -> Result<f32, &'static str> {
        if pos1 == pos2 {
            return Ok(0.0);
        }

        let from_city = std::cmp::max(pos1, pos2);
        let to_city = std::cmp::min(pos1, pos2);

        let n_items_before = (from_city - 1) * from_city / 2;
        if n_items_before > self.size {
            return Err("city with biggest id is not in distance matrix");
        }

        let distance_idx = n_items_before + to_city;

        Ok(self.items[distance_idx])
    }

    /// returns distance between city n and m
    /// array is packed version of bottom triangle with given structure
    /// ||d2,1|d3,1|d3,2|d4,1|d4,2|d4,3||
    /// here bigger cityId works like padding, then smaller id acts as index from padding
    pub fn distance_between(&self, city_id1: usize, city_id2: usize) -> Result<f32, &'static str> {
        if city_id1 == city_id2 {
            return Ok(0.0); // elements on the diagonal
        }

        // translate city ids to matrix id
        let pos1 = self
            .city_idx
            .get(&city_id1)
            .expect("city_id1 doesnt exists in index");

        let pos2 = self
            .city_idx
            .get(&city_id2)
            .expect("city_id2 doesnt exists in index");

        self.distance_by_pos(*pos1, *pos2)
    }

    /// returns list of distances from city N, where 0 distance from the city;
    pub fn distances_from(&self, city_id: usize) -> Vec<f32> {
        let pos: usize = self
            .city_idx
            .get(&city_id)
            .expect("Unknown city id")
            .clone();

        self.distances_from_index(pos)
    }

    pub fn tour_length(&self, path: &[usize]) -> f32 {
        let tour_length = path.len();
        if tour_length < 2 {
            return 0.0;
        }

        let last_city_id = path.last().unwrap().clone();
        let mut total = self.distance_between(last_city_id, path[0]).unwrap();

        for i in 1..tour_length {
            total += self.distance_between(path[i], path[i - 1]).unwrap();
        }

        total
    }

    pub fn city_index(&self) -> &HashMap<usize, usize> {
        &self.city_idx
    }

    pub fn pos2city_id(&self, pos: &usize) -> Option<usize> {
        self.cities.get(pos).map(|c| c.id.clone())
    }

    pub fn city_id2pos(&self, city_id: &usize) -> Option<usize> {
        self.city_idx.get(city_id).map(|i| i.clone())
    }

    pub fn nearest(&self, target: &KDPoint, n: usize) -> NearestResult {
        let mut search_result = NearestResult::new(target.clone(), f32::INFINITY, n);

        if let Some(city_pos) = self.city_id2pos(&target.id) {
            let distances_from_target = self.distances_from_index(city_pos);
            for (pos, distance) in distances_from_target.iter().enumerate() {
                let city_id = self.pos2city_id(&pos).unwrap();

                // the NearestResult takes care of ordering the results
                if let Some(pt) = self.cities.get(&city_id) {
                    search_result.add(pt.clone(), distance.clone());
                }
            }
        } else {
            panic!("DistanceMatrix.nearest with unknow_id");
        }

        search_result
    }

    fn distances_from_index(&self, pos: usize) -> Vec<f32> {
        let mut distances: Vec<f32> = vec![];

        // all the values from the city row aka with smalled_ids
        for i in 0..pos {
            let d = self.distance_between(pos, i).unwrap_or(-1.0); //-1 would mean error
            distances.push(d);
        }

        // all the values form the cities with bigger ID
        // it starts from city , which would return distance 0, which we need for place holder
        let n_cities = self.n;
        for i in pos..n_cities {
            let d = self.distance_between(i, pos).unwrap_or(-1.0);
            distances.push(d);
        }

        distances
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::assert_approx;
    use crate::tsp::kdtree;

    #[test]
    fn test_build_distance_matrix_from_empty_list() {
        let cities = kdtree::build_points(&vec![]);

        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_err());
    }

    #[test]
    fn test_build_distance_matrix_from_singleton_list() {
        let cities = kdtree::build_points(&[vec![100.0, 100.0]]);

        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_err());
    }

    #[test]
    fn test_build_distance_matrix_from_2_item_list() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![0.0, 1.0]]);

        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(&[1.0], res.distances());
    }

    #[test]
    fn test_build_distance_matrix_from_3_item_list() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![0.0, 1.0], vec![2.0, 0.0]]);

        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(3, res.len());
        assert_approx(1.0, res.distances()[0]); // from 1 to 2
        assert_approx(2.0, res.distances()[1]); // from 1 to 3
        assert_approx(2.236_068, res.distances()[2]); // from 2 to 3
    }

    #[test]
    fn test_distance_matrix_distance_between_with_3_cities_example() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![0.0, 1.0], vec![2.0, 0.0]]);
        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_ok());
        let dm = res.unwrap();
        let d23 = 2.236_068;

        // city id starts from 0
        assert_approx(1.0, dm.distance_between(0, 1).unwrap());
        assert_approx(1.0, dm.distance_between(1, 0).unwrap());
        assert_approx(2.0, dm.distance_between(0, 2).unwrap());
        assert_approx(2.0, dm.distance_between(2, 0).unwrap());
        assert_approx(d23, dm.distance_between(1, 2).unwrap());
        assert_approx(d23, dm.distance_between(2, 1).unwrap());
    }

    #[test]
    fn test_distance_matrix_distance_between_with_4_cities_example() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![2.0, 0.0],
            vec![4.0, 0.0],
        ]);
        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_ok());
        let dm = res.unwrap();
        let d23 = 2.236_068;
        let d31 = 4.123_1055;

        assert_eq!(6, dm.len());

        // city id starts from 0
        assert_approx(1.0, dm.distance_between(1, 0).unwrap());
        assert_approx(2.0, dm.distance_between(2, 0).unwrap());
        assert_approx(d23, dm.distance_between(2, 1).unwrap());
        assert_approx(4.0, dm.distance_between(3, 0).unwrap());
        assert_approx(d31, dm.distance_between(3, 1).unwrap());
        assert_approx(2.0, dm.distance_between(3, 2).unwrap())
    }

    #[test]
    fn test_distance_matrix_distances_from_city1() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![2.0, 0.0],
            vec![4.0, 0.0],
        ]);
        let dm = DistanceMatrix::from_cities(&cities).unwrap();

        let res = dm.distances_from(0);
        assert_eq!(4, res.len());
        assert_approx(0.0, res[0]);
        assert_approx(1.0, res[1]);
        assert_approx(2.0, res[2]);
        assert_approx(4.0, res[3]);
    }

    #[test]
    fn test_distance_matrix_tour_length_with_tsp_5_1() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);

        let route = vec![0, 1, 2, 3, 4];
        let dm = DistanceMatrix::from_cities(&cities).unwrap();

        assert_approx(4.0, dm.tour_length(&route));
    }

    #[test]
    fn test_nearest_for_tsp_5_1() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);

        let dm = from_cities(&cities);

        let res = dm.nearest(&cities[0], 3);
        assert_eq!(cities[1].id, res.point.id);

        let res2 = dm.nearest(&cities[1], 3);
        assert_eq!(cities[0].id, res2.point.id);

        let res3 = dm.nearest(&cities[2], 3);
        assert_eq!(cities[1].id, res3.point.id);

        let res4 = dm.nearest(&cities[3], 3);
        assert_eq!(cities[2].id, res4.point.id);

        let res5 = dm.nearest(&cities[4], 2);
        assert_eq!(cities[0].id, res5.point.id);
    }
}

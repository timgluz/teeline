/*
    DistanceMatrix is data collection to keep euclidean distances between 2D points;

    Because TSP problem makes simplification and only considers symmetrical and positive distances.
    It also discards self-symmetrical path aka return to same node.
    We can save disk and computational time by only considering bottom half of the distance matrix;

    The upper-half is achived by reversing indexes, so that bigger coordinate reflects to row number
*/

use super::kdtree::KDPoint;

#[derive(Debug, Clone)]
pub struct DistanceMatrix {
    first_id: usize, // the first city id, default 0
    n: usize,        // how many cities
    size: usize,     // how many distances under diagonal
    items: Vec<f32>,
}

impl DistanceMatrix {
    pub fn new(first_id: usize, n: usize, distances: Vec<f32>) -> Self {
        DistanceMatrix {
            first_id,
            n,                     // how many cities
            size: distances.len(), // how many distances under diagonal
            items: distances,
        }
    }

    // assumes that cities are already sorted by id and ids are incrementally crowing
    pub fn from_cities(cities: &[KDPoint]) -> Result<Self, &'static str> {
        let n = cities.len();
        if n < 2 {
            return Err("distance matrix requires at least 2 points");
        }

        let size = n * (n - 1) / 2; // how many items on distance vec

        let mut distances = Vec::with_capacity(size);
        for i in 1..n {
            let pt1 = &cities[i];

            for j in 0..i {
                let pt2 = &cities[j];
                distances.push(pt1.distance(pt2));
            }
        }

        distances.shrink_to_fit();

        Ok(DistanceMatrix::new(0, n, distances))
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn distances(&self) -> &[f32] {
        &self.items
    }

    /// returns distance between city n and m
    /// array is packed version of bottom triangle with given structure
    /// ||d2,1|d3,1|d3,2|d4,1|d4,2|d4,3||
    /// here bigger cityId works like padding, then smaller id acts as index from padding
    pub fn distance_between(&self, city_id1: usize, city_id2: usize) -> Result<f32, &'static str> {
        if city_id1 == city_id2 {
            return Ok(0.0); // elements on the diagonal
        }

        let from_city = std::cmp::max(city_id1, city_id2);
        let to_city = std::cmp::min(city_id1, city_id2);

        let prev_city = from_city - self.first_id;
        let n_items_before = (prev_city - 1) * prev_city / 2;
        if n_items_before > self.size {
            return Err("city with biggest id is not in distance matrix");
        }

        let distance_idx = n_items_before + to_city;

        Ok(self.items[distance_idx])
    }

    /// returns list of distances from city N, where 0 distance from the city;
    pub fn distances_from(&self, city_id: usize) -> Vec<f32> {
        let mut distances: Vec<f32> = vec![];

        // all the values from the city row
        for i in self.first_id..city_id {
            let d = self.distance_between(city_id, i).unwrap_or(-1.0); //-1 would mean error
            distances.push(d);
        }

        // all the values form the cities with bigger ID
        // it starts from city_id, which would return distance 0, which we need for place holder
        let n_cities = self.n;
        for i in city_id..n_cities {
            let d = self.distance_between(i, city_id).unwrap_or(-1.0);
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
}

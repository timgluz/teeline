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
    n: usize,    // how many cities
    size: usize, // how many distances under diagonal
    items: Vec<f32>,
    cities: CityTable,
    city_idx: HashMap<usize, usize>, // translates city_id to matrix_id
}

impl DistanceMatrix {
    pub fn new(n: usize, distances: Vec<f32>, cities: CityTable) -> Self {
        let city_idx: HashMap<usize, usize> = cities.iter().map(|(k, c)| (c.id, *k)).collect();

        assert!(n == city_idx.len(), "city_idx size differs from n cities");
        assert_eq!(
            distances.len(),
            n * (n - 1) / 2,
            "distances length {} != n*(n-1)/2={} for n={}",
            distances.len(),
            n * (n - 1) / 2,
            n
        );
        DistanceMatrix {
            n,
            size: distances.len(),
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
        for (i, pt1) in cities.iter().enumerate() {
            city_table.insert(i, *pt1);

            for pt2 in cities.iter().take(i) {
                distances.push(pt1.distance(pt2));
            }
        }

        distances.shrink_to_fit();

        Ok(DistanceMatrix::new(n, distances, city_table))
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn num_cities(&self) -> usize {
        self.n
    }

    pub fn num_distances(&self) -> usize {
        self.size
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
        if pos1 >= self.n || pos2 >= self.n {
            return Err("position out of range");
        }
        let from = pos1.max(pos2);
        let to = pos1.min(pos2);
        let idx = from * (from - 1) / 2 + to;
        self.items.get(idx).copied().ok_or("distance index out of range")
    }

    /// returns distance between city n and m
    /// array is packed version of bottom triangle with given structure
    /// ||d2,1|d3,1|d3,2|d4,1|d4,2|d4,3||
    /// here bigger cityId works like padding, then smaller id acts as index from padding
    pub fn distance_between(&self, city_id1: usize, city_id2: usize) -> Result<f32, &'static str> {
        if city_id1 == city_id2 {
            return Ok(0.0);
        }
        let pos1 = self.city_idx.get(&city_id1).copied().ok_or("city_id1 not in index")?;
        let pos2 = self.city_idx.get(&city_id2).copied().ok_or("city_id2 not in index")?;
        self.distance_by_pos(pos1, pos2)
    }

    /// returns list of distances from city N, where 0 distance from the city;
    pub fn distances_from(&self, city_id: usize) -> Vec<f32> {
        let pos: usize = *self.city_idx.get(&city_id).expect("Unknown city id");

        self.distances_from_index(pos)
    }

    pub fn tour_length(&self, path: &[usize]) -> f32 {
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

    /// Position-based tour length — no HashMap lookups per edge.
    /// All positions must be in 0..num_cities().
    pub fn tour_length_by_pos(&self, path: &[usize]) -> f32 {
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

    pub fn city_index(&self) -> &HashMap<usize, usize> {
        &self.city_idx
    }

    pub fn pos2city_id(&self, pos: usize) -> Option<usize> {
        self.cities.get(&pos).map(|c| c.id)
    }

    pub fn city_id2pos(&self, city_id: usize) -> Option<usize> {
        self.city_idx.get(&city_id).copied()
    }

    pub fn nearest(&self, target: &KDPoint, n: usize) -> NearestResult {
        let mut search_result = NearestResult::new(*target, f32::INFINITY, n);

        let city_pos = match self.city_id2pos(target.id) {
            Some(pos) => pos,
            None => return search_result, // unknown target → empty result, no panic
        };

        let distances_from_target = self.distances_from_index(city_pos);
        for (pos, distance) in distances_from_target.iter().enumerate() {
            if pos == city_pos {
                continue; // skip self (belt-and-suspenders; add() already gates on pt.id)
            }
            // Look up by matrix position, not city_id.  city_id != pos when city IDs
            // are not 0-based (e.g. 1-indexed TSPLIB files like berlin52.tsp).
            if let Some(pt) = self.cities.get(&pos) {
                search_result.add(*pt, *distance);
            }
        }

        search_result
    }

    fn distances_from_index(&self, pos: usize) -> Vec<f32> {
        let mut distances = Vec::with_capacity(self.n);
        for i in 0..pos {
            distances.push(self.distance_by_pos(pos, i).expect("valid position in distances_from_index"));
        }
        for i in pos..self.n {
            distances.push(self.distance_by_pos(i, pos).expect("valid position in distances_from_index"));
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

    // Regression: nearest must work when city IDs are 1-based (as in TSPLIB files like berlin52).
    // The old code used city_id as a matrix-position key; since city_id != matrix_pos for
    // 1-indexed cities, it returned the wrong city with distance 0.0, making every "nearest"
    // the immediately next city in the ID sequence and turning NN into a no-op sorted walk.
    #[test]
    fn test_nearest_with_one_indexed_city_ids() {
        let cities = vec![
            KDPoint::new_with_id(1, &[0.0, 0.0]),
            KDPoint::new_with_id(2, &[10.0, 0.0]), // far
            KDPoint::new_with_id(3, &[0.0, 1.0]),  // closest to city 1
        ];
        let dm = from_cities(&cities);

        let res = dm.nearest(&cities[0], 2); // nearest to city 1
        // city 3 (distance 1.0) must be reported closer than city 2 (distance 10.0)
        assert_eq!(
            res.nearest().first().unwrap().point.id,
            3,
            "nearest to city 1 should be city 3, not city 2"
        );
    }
}

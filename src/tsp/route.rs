/// Route is ordered list of city ids that our traveling salesperson
/// is going to visit
use rand::seq::SliceRandom;
use rand::Rng;

use super::kdtree::KDPoint;

#[derive(Debug, Clone)]
pub struct Route {
    route: Vec<usize>,
}

impl Route {
    pub fn from_cities(cities: &[KDPoint]) -> Self {
        let route: Vec<usize> = cities.iter().map(|x| x.id.clone()).collect();

        Route { route }
    }

    pub fn get(&self, pos: usize) -> Option<usize> {
        self.route.get(pos).map(|r| r.clone())
    }

    pub fn len(&self) -> usize {
        self.route.len()
    }

    pub fn route(&self) -> &[usize] {
        self.route.as_slice()
    }

    pub fn shuffle(&mut self) {
        let mut rng = rand::thread_rng();

        self.route.shuffle(&mut rng);
    }

    // it swaps 2 cities using 2-opt
    pub fn random_successor(&self) -> Route {
        let mut candidate = self.route.clone().to_vec();

        let (from_pos, to_pos) = random_position_pair(self.len());
        swap_cities(&mut candidate, from_pos, to_pos);

        Route { route: candidate }
    }
}

impl PartialEq for Route {
    fn eq(&self, other: &Route) -> bool {
        self.route == other.route
    }
}

fn random_position_pair(n_items: usize) -> (usize, usize) {
    let mut pair = random_pair(n_items);
    let max_iter = 10;

    for _ in 0..max_iter {
        // make sure that indexes are different and not adjacent
        if pair.1.wrapping_sub(pair.0) > 1 {
            break;
        }

        pair = random_pair(n_items);
    }

    pair
}

fn random_pair(n_items: usize) -> (usize, usize) {
    if n_items < 2 {
        panic!("n_items must be bigger than 2");
    }

    let mut rng = rand::thread_rng();
    let pos1 = rng.gen_range(0, n_items);
    let pos2 = rng.gen_range(0, n_items);

    if pos1 < pos2 {
        (pos1, pos2)
    } else {
        (pos2, pos1)
    }
}

fn swap_cities(route: &mut Vec<usize>, from: usize, to: usize) {
    if to >= route.len() {
        panic!("to can not be same or bigger than route size");
    }

    // 2-OPT keeps changes in more stable
    let reversed_seq: Vec<usize> = route[from..=to].iter().map(|x| x.clone()).rev().collect();

    // swap values from routes with reversed_seq
    for (i, swapped_val) in reversed_seq.iter().enumerate() {
        route[from + i] = swapped_val.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::kdtree;

    #[test]
    fn test_route_from_cities_with_1_elem() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0]]);
        let route = Route::from_cities(&cities);

        assert_eq!(1, route.len());
        assert_eq!(Some(0), route.get(0));
    }

    #[test]
    fn test_route_from_cities_with_2_elem() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 1.0]]);
        let route = Route::from_cities(&cities);

        assert_eq!(2, route.len());
        assert_eq!(Some(0), route.get(0));
        assert_eq!(Some(1), route.get(1));
    }

    #[test]
    fn test_route_random_successor_returns_new_route() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);

        let route1 = Route::from_cities(&cities);
        let route2 = route1.random_successor();

        assert!(route1.len() == route2.len())
    }
}

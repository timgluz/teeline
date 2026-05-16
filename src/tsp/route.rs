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
    pub fn new(path: &[usize]) -> Self {
        Route {
            route: path.to_vec(),
        }
    }

    pub fn from_cities(cities: &[KDPoint]) -> Self {
        let route: Vec<usize> = cities.iter().map(|x| x.id).collect();

        Route { route }
    }

    pub fn get(&self, pos: usize) -> Option<usize> {
        self.route.get(pos).copied()
    }

    pub fn len(&self) -> usize {
        self.route.len()
    }

    pub fn is_empty(&self) -> bool {
        self.route.is_empty()
    }

    pub fn route(&self) -> &[usize] {
        self.route.as_slice()
    }

    pub fn shuffle(&mut self) {
        let mut rng = rand::rng();
        self.route.shuffle(&mut rng);
    }

    // it swaps 2 cities using 2-opt
    pub fn random_successor(&self) -> Route {
        let mut candidate = self.route.clone().to_vec();

        let (from_pos, to_pos) = random_position_pair(self.len());
        swap_cities(&mut candidate, from_pos, to_pos);

        Route { route: candidate }
    }

    pub fn sort(&mut self) {
        self.route.sort()
    }
}

impl PartialEq for Route {
    fn eq(&self, other: &Route) -> bool {
        self.route == other.route
    }
}

// maybe into utils?
pub fn random_position_pair(n_items: usize) -> (usize, usize) {
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

// from Skiena ch.7.5.1 - random sampling
fn random_pair(n_items: usize) -> (usize, usize) {
    if n_items < 2 {
        panic!("n_items must be bigger than 2");
    }

    let mut rng = rand::rng();
    let pos1 = rng.random_range(0..n_items);
    let pos2 = rng.random_range(0..n_items);

    if pos1 < pos2 {
        (pos1, pos2)
    } else {
        (pos2, pos1)
    }
}

fn swap_cities(route: &mut [usize], from: usize, to: usize) {
    if to >= route.len() {
        panic!("to can not be same or bigger than route size");
    }

    // 2-OPT keeps changes more stable
    let reversed_seq: Vec<usize> = route[from..=to].iter().copied().rev().collect();

    for (i, swapped_val) in reversed_seq.iter().enumerate() {
        route[from + i] = *swapped_val;
    }
}

/// Greedy swap sequence that transforms `from` into `to`.
/// Returns `Vec<(pos_i, pos_j)>` of swaps to apply in order.
/// Both slices must be permutations of the same elements.
pub fn swap_sequence(from: &[usize], to: &[usize]) -> Vec<(usize, usize)> {
    let n = from.len();
    let mut tmp = from.to_vec();
    let mut seq: Vec<(usize, usize)> = Vec::new();
    for i in 0..n.saturating_sub(1) {
        if tmp[i] != to[i] {
            let j = tmp[i + 1..]
                .iter()
                .position(|&x| x == to[i])
                .map(|p| p + i + 1)
                .expect("swap_sequence: permutations must share the same elements");
            seq.push((i, j));
            tmp.swap(i, j);
        }
    }
    seq
}

/// Apply an ordered list of `(i, j)` swaps to `position`, returning the result.
pub fn apply_swaps(position: &[usize], swaps: &[(usize, usize)]) -> Vec<usize> {
    let mut pos = position.to_vec();
    for &(i, j) in swaps {
        pos.swap(i, j);
    }
    pos
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
    fn test_swap_sequence_converts_from_to_to() {
        let from = vec![1, 2, 3, 4];
        let to = vec![1, 3, 2, 4];
        assert_eq!(apply_swaps(&from, &swap_sequence(&from, &to)), to);
    }

    #[test]
    fn test_swap_sequence_identity_is_empty() {
        let v = vec![1, 2, 3];
        assert!(swap_sequence(&v, &v).is_empty());
    }

    #[test]
    fn test_apply_swaps_empty_is_identity() {
        let pos = vec![1, 2, 3];
        assert_eq!(apply_swaps(&pos, &[]), pos);
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

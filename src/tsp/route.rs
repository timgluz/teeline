use rand::RngExt;
/// Route is ordered list of city ids that our traveling salesperson
/// is going to visit
use rand::seq::SliceRandom;
use std::collections::HashSet;

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

    /// Returns the city ID at route rank `pos` (not a dense city position).
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

/// Reverses the route segment between two route ranks (a 2-opt move).
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
///
/// Both inputs are **routes** (sequences of city IDs), not position lists; the returned
/// pairs are route ranks.
pub fn swap_sequence(from: &[usize], to: &[usize]) -> Vec<(usize, usize)> {
    let n = from.len();
    assert_eq!(
        n,
        to.len(),
        "swap_sequence: routes must have the same length"
    );
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

/// Apply an ordered list of route-rank swaps to a route of city IDs, returning the result.
pub fn apply_swaps(route: &[usize], swaps: &[(usize, usize)]) -> Vec<usize> {
    let mut pos = route.to_vec();
    for &(i, j) in swaps {
        pos.swap(i, j);
    }
    pos
}

/// Fisher-Yates shuffle of `city_ids`; the shared random start for population solvers.
pub(crate) fn random_tour(city_ids: &[usize], rng: &mut impl RngExt) -> Vec<usize> {
    let mut tour = city_ids.to_vec();
    for i in (1..tour.len()).rev() {
        let j = rng.random_range(0..=i);
        tour.swap(i, j);
    }
    tour
}

/// Order-crossover move toward `target`: copy a random contiguous segment of `target`
/// (wrapping around the ends) and fill the rest in `cur`'s relative order.
///
/// A transposition can break four edges at once and is blind to tour rotation; this move
/// keeps the target's edges inside the copied segment and breaks only its junctions.
/// `used` is caller-owned scratch so a hot loop does not allocate one per move.
pub(crate) fn move_toward(
    cur: &[usize],
    target: &[usize],
    fraction: f64,
    rng: &mut impl RngExt,
    used: &mut HashSet<usize>,
) -> Vec<usize> {
    let n = cur.len();
    if n < 4 {
        return target.to_vec();
    }
    let seg_len = ((fraction * n as f64).round() as usize).clamp(1, n - 1);
    let start = rng.random_range(0..n);

    let mut child = vec![usize::MAX; n];
    used.clear();
    for k in 0..seg_len {
        let idx = (start + k) % n;
        child[idx] = target[idx];
        used.insert(target[idx]);
    }

    let mut fill = (start + seg_len) % n;
    for k in 0..n {
        let city = cur[(start + seg_len + k) % n];
        if !used.contains(&city) {
            child[fill] = city;
            fill = (fill + 1) % n;
        }
    }

    child
}

/// One random 2-opt reversal. Shared by the PSO perturbation and FPA's local pollination.
pub(crate) fn random_reversal(tour: &mut [usize], rng: &mut impl RngExt) {
    let n = tour.len();
    if n < 3 {
        return;
    }
    let a = rng.random_range(0..n);
    let b = rng.random_range(0..n);
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    if hi - lo > 1 {
        tour[lo..=hi].reverse();
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

    #[test]
    fn test_random_tour_is_a_permutation() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let ids = vec![0, 1, 2, 3, 4, 5];
        let mut tour = random_tour(&ids, &mut rng);
        tour.sort();
        assert_eq!(tour, ids);
    }

    #[test]
    fn test_move_toward_returns_a_permutation() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let cur = vec![0, 1, 2, 3, 4, 5];
        let target = vec![3, 4, 5, 0, 1, 2];
        let mut used = HashSet::new();
        let mut child = move_toward(&cur, &target, 0.5, &mut rng, &mut used);
        child.sort();
        assert_eq!(child, cur);
    }

    #[test]
    fn test_move_toward_copies_a_target_segment() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let cur = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let target: Vec<usize> = (0..8).rev().collect();
        let mut used = HashSet::new();
        let child = move_toward(&cur, &target, 0.5, &mut rng, &mut used);

        let copied = child.iter().zip(&target).filter(|(a, b)| a == b).count();
        assert!(
            copied >= 4,
            "expected at least the 4-city target segment, got {copied}"
        );
    }
}

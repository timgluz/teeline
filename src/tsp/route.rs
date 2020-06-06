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

fn random_position_pair(n_items: usize) -> (usize, usize) {
    let mut rng = rand::thread_rng();
    let pos1 = rng.gen_range(0, n_items);
    let pos2 = rng.gen_range(0, n_items);

    // TODO: make sure that abs_diff is bigger than 1
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

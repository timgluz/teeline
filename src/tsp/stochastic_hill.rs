use rand::seq::SliceRandom;
use rand::Rng;

use super::kdtree::KDPoint;
use super::tour::{self, Tour};

pub const MAX_STALE: usize = 10_000;
pub const MAX_EPOCH: usize = 5_000_000; // TODO: check better condition

pub fn solve(cities: &[KDPoint]) -> Tour {
    let mut best_route: Vec<usize> = cities.iter().map(|x| x.id.clone()).collect();
    let mut prev_best_route: Option<Vec<usize>> = None;
    let mut prev_distance = f32::MAX;

    //mix up the cities to avoid getting stuck due bad initial state
    shuffle_route(&mut best_route);
    let mut best_distance = tour::total_distance(cities, &best_route);

    let mut epoch = 0;
    let mut n_stale = 0;
    loop {
        let candidate = generate_successor(&best_route);
        let candidate_distance = tour::total_distance(&cities, &candidate);

        if candidate_distance < best_distance {
            best_route = candidate;
            best_distance = candidate_distance;

            n_stale = 0;
        } else {
            n_stale += 1; // to measure how long we have been walking around on the platoo
        }

        epoch += 1;

        // restart search if been wandering too long on the platoo
        if n_stale > MAX_STALE {
            // memorize the global best candidate in case restart goes wrong direction
            if best_distance < prev_distance {
                prev_best_route = Some(best_route.clone());
                prev_distance = best_distance;
            }

            shuffle_route(&mut best_route);
            best_distance = tour::total_distance(cities, &best_route);
            n_stale = 0;
        }

        // check if we should finish search
        if epoch > MAX_EPOCH {
            break;
        }
    }

    if prev_distance < best_distance {
        best_route = prev_best_route.unwrap()
    };

    Tour::new(&best_route, cities)
}

fn generate_successor(route: &[usize]) -> Vec<usize> {
    let mut candidate = route.clone().to_vec();

    let (from_pos, to_pos) = random_position_pair(route.len());
    swap_cities(&mut candidate, from_pos, to_pos);

    candidate
}

fn random_position_pair(n_items: usize) -> (usize, usize) {
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

fn shuffle_route(cities: &mut Vec<usize>) {
    let mut rng = rand::thread_rng();

    cities.shuffle(&mut rng);
}

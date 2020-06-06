use rand::Rng;

use super::kdtree::KDPoint;
use super::route::Route;
use super::tour::{self, Tour};

const MAX_TEMPERATURE: f32 = 1_000.0;
const MIN_TEMPERATURE: f32 = 0.000_1;
const MAX_EPOCH: usize = 100_000;

pub fn solve(cities: &[KDPoint]) -> Tour {
    let cooling_rate = 0.000_1;
    let mut epoch = 0;

    let mut best_route = Route::from_cities(cities);
    let mut best_distance = tour::total_distance(cities, best_route.route());

    let mut temperature = MAX_TEMPERATURE;
    while epoch < MAX_EPOCH || temperature > MIN_TEMPERATURE {
        let candidate = best_route.random_successor();
        let candidate_distance = tour::total_distance(cities, candidate.route());

        if is_acceptable(temperature, best_distance, candidate_distance) {
            best_route = candidate;
            best_distance = candidate_distance;
        }

        temperature = cooling(temperature, cooling_rate);
        epoch += 1;
    }

    Tour::new(best_route.route(), cities)
}

fn cooling(temperature: f32, cooling_rate: f32) -> f32 {
    temperature - cooling_rate * temperature
}

fn is_acceptable(temperature: f32, old_distance: f32, new_distance: f32) -> bool {
    if new_distance < old_distance {
        return true;
    }

    // if they are basically same - then false
    if (new_distance - old_distance).abs() < f32::EPSILON {
        return false;
    }

    let mut rng = rand::thread_rng();

    let p: f32 = rng.gen();
    let criteria = metropolis(temperature, old_distance, new_distance);

    //println!("P {:?} < {:?}", p, criteria);

    p < criteria
}

fn metropolis(t: f32, e1: f32, e2: f32) -> f32 {
    (-(e2 - e1) / t).exp()
}

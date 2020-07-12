use rand::Rng;

use super::kdtree::KDPoint;
use super::route::Route;
use super::{total_distance, Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    let cooling_rate = options.cooling_rate;
    let mut epoch = 0;

    let mut best_route = Route::from_cities(cities);
    let mut best_distance = total_distance(cities, best_route.route());

    let mut temperature = options.max_temperature;
    while epoch < options.epochs || temperature > options.min_temperature {
        let candidate = best_route.random_successor();
        let candidate_distance = total_distance(cities, candidate.route());

        if is_acceptable(temperature, best_distance, candidate_distance) {
            best_route = candidate;
            best_distance = candidate_distance;

            if options.verbose {
                println!(
                    "SA: epoch.{:?} new best distance: {:?}",
                    epoch, best_distance
                );
            }
        }

        temperature = cooling(temperature, cooling_rate);
        epoch += 1;
    }

    Solution::new(best_route.route(), cities)
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

    p < criteria
}

// TODO: double-check implementation as rust exp may behave differently
fn metropolis(t: f32, e1: f32, e2: f32) -> f32 {
    (-(e2 - e1) / t).exp()
}

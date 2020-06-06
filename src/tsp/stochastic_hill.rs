use super::kdtree::KDPoint;
use super::route::Route;
use super::tour::{self, Tour};

pub const MAX_STALE: usize = 10_000;
pub const MAX_EPOCH: usize = 5_000_000; // TODO: check better condition

pub fn solve(cities: &[KDPoint]) -> Tour {
    let mut best_route = Route::from_cities(cities);

    let mut prev_best_route: Option<Route> = None;
    let mut prev_distance = f32::MAX;

    //mix up the cities to avoid getting stuck due bad initial state
    best_route.shuffle();
    let mut best_distance = tour::total_distance(cities, &best_route.route());

    let mut epoch = 0;
    let mut n_stale = 0;
    loop {
        let candidate = best_route.random_successor();
        let candidate_distance = tour::total_distance(&cities, candidate.route());

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

            best_route.shuffle();
            best_distance = tour::total_distance(cities, &best_route.route());
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

    Tour::new(best_route.route(), cities)
}

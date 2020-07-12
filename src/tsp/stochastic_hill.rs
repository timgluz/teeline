use super::kdtree::KDPoint;
use super::route::Route;
use super::{total_distance, Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    let mut current_route = Route::from_cities(cities);
    let mut best_route = current_route.clone();

    //mix up the cities to avoid getting stuck due bad initial state
    current_route.shuffle();

    let mut epoch = 0;
    let mut n_stale = 0;
    let mut best_distance = total_distance(cities, &best_route.route());
    loop {
        let candidate = current_route.random_successor();
        let candidate_distance = total_distance(&cities, candidate.route());

        if candidate_distance < best_distance {
            best_route = candidate;
            best_distance = candidate_distance;

            n_stale = 0;

            if options.verbose {
                println!("Epoch: {:?}, new best distance: {:}", epoch, best_distance);
            }
        } else {
            n_stale += 1; // to measure how long we have been walking around on the platoo
        }

        epoch += 1;

        // restart search if been wandering too long on the platoo
        if n_stale > options.platoo_epochs && options.platoo_epochs > 0 {
            if options.verbose {
                println!(
                    "Epoch: {:?}, got stuck after {:?} steps, going to restart search",
                    epoch, options.platoo_epochs
                );
            }

            current_route.shuffle();
            n_stale = 0;
        }

        // check if we should finish search
        if options.epochs > 0 && epoch > options.epochs {
            break;
        }
    }

    Solution::new(best_route.route(), cities)
}

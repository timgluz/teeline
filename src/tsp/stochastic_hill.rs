use super::kdtree::KDPoint;
use super::progress::{send_progress, ProgressMessage};
use super::route::Route;
use super::{total_distance, Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    tracing::info!(
        epochs = options.epochs,
        platoo_epochs = options.platoo_epochs,
        "hill climbing starting"
    );

    let mut current_route = Route::from_cities(cities);

    //mix up the cities to avoid getting stuck due bad initial state
    current_route.shuffle();
    send_progress(ProgressMessage::PathUpdate(current_route.clone(), 0.0));

    // Baseline from the shuffled state — sequential ordering would be
    // an artificially low bar that random successors can never beat.
    let mut best_route = current_route.clone();
    let mut best_distance = total_distance(cities, best_route.route());

    let mut epoch = 0;
    let mut n_stale = 0;
    let mut found_improvement = false;
    loop {
        let candidate = current_route.random_successor();
        let candidate_distance = total_distance(cities, candidate.route());

        if candidate_distance < best_distance {
            current_route = candidate.clone(); // follow the gradient
            best_route = candidate;
            best_distance = candidate_distance;
            found_improvement = true;

            n_stale = 0;

            send_progress(ProgressMessage::PathUpdate(
                best_route.clone(),
                best_distance,
            ));

            tracing::info!(epoch, tour_length = best_distance, "hill: new best");
        } else {
            n_stale += 1; // to measure how long we have been walking around on the platoo
        }

        epoch += 1;

        // restart search if been wandering too long on the platoo
        if n_stale > options.platoo_epochs && options.platoo_epochs > 0 {
            tracing::warn!(epoch, plateau_epochs = options.platoo_epochs, "hill: plateau, restarting");

            current_route.shuffle();
            n_stale = 0;

            send_progress(ProgressMessage::PathUpdate(current_route.clone(), 0.0));
        }

        // check if we should finish the search
        if options.epochs > 0 && epoch > options.epochs {
            break;
        }
    }

    if !found_improvement {
        tracing::warn!(epochs = options.epochs, tour_length = best_distance, "hill: no improvement found");
    }

    send_progress(ProgressMessage::Done);
    Solution::new(best_route.route(), cities)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::kdtree;

    fn tsp5_cities() -> Vec<KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ])
    }

    fn fast_options() -> SolverOptions {
        let mut opts = SolverOptions::default();
        opts.epochs = 500;
        opts.platoo_epochs = 100;
        opts
    }

    #[test]
    fn test_solve_visits_all_cities() {
        let cities = tsp5_cities();
        let tour = solve(&cities, &fast_options());

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_solve_tour_length_is_positive_and_finite() {
        let cities = tsp5_cities();
        let tour = solve(&cities, &fast_options());

        assert!(tour.total > 0.0, "tour length should be positive");
        assert!(tour.total.is_finite(), "tour length should be finite");
    }

    #[test]
    fn test_solve_terminates_within_epoch_limit() {
        let cities = tsp5_cities();
        let mut opts = SolverOptions::default();
        opts.epochs = 100;
        opts.platoo_epochs = 50;
        let tour = solve(&cities, &opts);
        // the test itself verifies termination by returning
        assert_eq!(tour.route().len(), cities.len());
    }
}

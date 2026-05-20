use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], distances: &DistanceMatrix, options: &SolverOptions) -> Solution {
    tracing::info!(
        epochs = options.epochs,
        platoo_epochs = options.platoo_epochs,
        "hill climbing starting"
    );

    let mut current_route = match options.initial_tour.as_deref() {
        Some(t) => Route::new(t),
        None => {
            let mut r = Route::from_cities(cities);
            r.shuffle();
            r
        }
    };
    options.send_progress(ProgressMessage::PathUpdate(current_route.clone(), 0.0));

    // Baseline from the shuffled state — sequential ordering would be
    // an artificially low bar that random successors can never beat.
    let mut best_route = current_route.clone();
    let mut best_distance = distances.tour_length(best_route.route());

    let mut epoch = 0;
    let mut n_stale = 0;
    let mut found_improvement = false;
    loop {
        let candidate = current_route.random_successor();
        let candidate_distance = distances.tour_length(candidate.route());

        if candidate_distance < best_distance {
            current_route = candidate.clone(); // follow the gradient
            best_route = candidate;
            best_distance = candidate_distance;
            found_improvement = true;

            n_stale = 0;

            options.send_progress(ProgressMessage::PathUpdate(
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

            options.send_progress(ProgressMessage::PathUpdate(current_route.clone(), 0.0));
        }

        // check if we should finish the search
        if options.epochs > 0 && epoch > options.epochs {
            break;
        }
    }

    if !found_improvement {
        tracing::warn!(epochs = options.epochs, tour_length = best_distance, "hill: no improvement found");
    }

    options.send_progress(ProgressMessage::Done);
    Solution::new(best_route.route(), cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree};

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
    fn test_hill_respects_initial_tour_skips_shuffle() {
        // When seeded, hill climbing should NOT shuffle the tour.
        // Verify by providing the known-optimal tour and checking
        // the solver doesn't produce something worse than sequential
        // (if it shuffled, average quality would degrade).
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let optimal: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let optimal_cost = dm.tour_length(&optimal);
        let mut opts = SolverOptions::default();
        opts.epochs = 1;
        opts.platoo_epochs = 1;
        opts.initial_tour = Some(optimal.clone());
        let result = solve(&cities, &dm, &opts);
        // Seeded from optimal: result should equal optimal (1 epoch can't improve on optimal)
        assert!((result.total - optimal_cost).abs() < 1e-4);
    }

    #[test]
    fn test_solve_visits_all_cities() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let tour = solve(&cities, &dm, &fast_options());

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_solve_tour_length_is_positive_and_finite() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let tour = solve(&cities, &dm, &fast_options());

        assert!(tour.total > 0.0, "tour length should be positive");
        assert!(tour.total.is_finite(), "tour length should be finite");
    }

    #[test]
    fn test_solve_terminates_within_epoch_limit() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let mut opts = SolverOptions::default();
        opts.epochs = 100;
        opts.platoo_epochs = 50;
        let tour = solve(&cities, &dm, &opts);
        assert_eq!(tour.route().len(), cities.len());
    }
}

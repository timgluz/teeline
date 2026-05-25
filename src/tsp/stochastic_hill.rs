use std::sync::mpsc;

use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem};

pub fn solve(
    problem: &TspProblem,
    opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    tracing::info!(
        epochs = opts.epochs,
        platoo_epochs = opts.platoo_epochs,
        "hill climbing starting"
    );

    let mut current_route = match init_tour {
        Some(t) => Route::new(t),
        None => {
            let mut r = Route::from_cities(cities);
            r.shuffle();
            r
        }
    };
    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(current_route.clone(), 0.0));
    }

    let mut best_route = current_route.clone();
    let mut best_distance = distances.tour_length(best_route.route());

    let mut epoch = 0;
    let mut n_stale = 0;
    let mut found_improvement = false;
    loop {
        let candidate = current_route.random_successor();
        let candidate_distance = distances.tour_length(candidate.route());

        if candidate_distance < best_distance {
            current_route = candidate.clone();
            best_route = candidate;
            best_distance = candidate_distance;
            found_improvement = true;

            n_stale = 0;

            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::PathUpdate(best_route.clone(), best_distance));
            }

            tracing::info!(epoch, tour_length = best_distance, "hill: new best");
        } else {
            n_stale += 1;
        }

        epoch += 1;

        if n_stale > opts.platoo_epochs && opts.platoo_epochs > 0 {
            tracing::warn!(epoch, plateau_epochs = opts.platoo_epochs, "hill: plateau, restarting");

            current_route.shuffle();
            n_stale = 0;

            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::PathUpdate(current_route.clone(), 0.0));
            }
        }

        if opts.epochs > 0 && epoch > opts.epochs {
            break;
        }
    }

    if !found_improvement {
        tracing::warn!(epochs = opts.epochs, tour_length = best_distance, "hill: no improvement found");
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(best_route.route(), cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, kdtree::KDPoint, HeuristicOptions, TspProblem};

    fn tsp5_cities() -> Vec<KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ])
    }

    fn fast_opts() -> HeuristicOptions {
        HeuristicOptions { epochs: 500, platoo_epochs: 100, ..HeuristicOptions::default() }
    }

    #[test]
    fn test_hill_respects_initial_tour_skips_shuffle() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let optimal: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let optimal_cost = dm.tour_length(&optimal);
        let opts = HeuristicOptions { epochs: 1, platoo_epochs: 1, ..HeuristicOptions::default() };
        let problem = TspProblem::new(cities, dm);
        let result = solve(&problem, &opts, None, Some(&optimal));
        assert!((result.total - optimal_cost).abs() < 1e-4);
    }

    #[test]
    fn test_solve_visits_all_cities() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities.clone(), dm);
        let tour = solve(&problem, &fast_opts(), None, None);

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_solve_tour_length_is_positive_and_finite() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);
        let tour = solve(&problem, &fast_opts(), None, None);

        assert!(tour.total > 0.0, "tour length should be positive");
        assert!(tour.total.is_finite(), "tour length should be finite");
    }

    #[test]
    fn test_solve_terminates_within_epoch_limit() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities.clone(), dm);
        let opts = HeuristicOptions { epochs: 100, platoo_epochs: 50, ..HeuristicOptions::default() };
        let tour = solve(&problem, &opts, None, None);
        assert_eq!(tour.route().len(), cities.len());
    }
}

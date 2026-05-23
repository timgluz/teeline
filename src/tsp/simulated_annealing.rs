use std::sync::mpsc;

use rand::Rng;

use super::probability::{cooling, metropolis};
use super::progress::ProgressMessage;
use super::route::Route;
use super::{SAOptions, Solution, TspProblem};

pub fn solve(
    problem: &TspProblem,
    opts: &SAOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let cooling_rate = opts.cooling_rate;
    let mut epoch = 0;

    tracing::info!(
        epochs = opts.heuristic.epochs,
        max_temp = opts.max_temperature,
        cooling_rate = opts.cooling_rate,
        "SA starting"
    );

    let mut best_route = problem.initial_tour.as_deref()
        .map(Route::new)
        .unwrap_or_else(|| Route::from_cities(cities));
    let mut best_distance = distances.tour_length(best_route.route());

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(best_route.clone(), best_distance));
    }

    let mut temperature = opts.max_temperature;
    while epoch < opts.heuristic.epochs || temperature > opts.min_temperature {
        let candidate = best_route.random_successor();
        let candidate_distance = distances.tour_length(candidate.route());

        if is_acceptable(temperature, best_distance, candidate_distance) {
            best_route = candidate;
            best_distance = candidate_distance;

            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::PathUpdate(best_route.clone(), best_distance));
            }
            tracing::info!(epoch, tour_length = best_distance, "SA: new best");
        }

        tracing::debug!(epoch, temperature, "SA: tick");
        temperature = cooling(temperature, cooling_rate);
        epoch += 1;
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(best_route.route(), cities, distances)
}

fn is_acceptable(temperature: f32, old_distance: f32, new_distance: f32) -> bool {
    if new_distance < old_distance {
        return true;
    }

    if (new_distance - old_distance).abs() < f32::EPSILON {
        return false;
    }

    let mut rng = rand::rng();
    let p: f32 = rng.random();
    let criteria = metropolis(temperature, old_distance, new_distance);

    p < criteria
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, HeuristicOptions, SAOptions, TspProblem};

    #[test]
    fn test_sa_respects_initial_tour() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0], vec![0.0, 0.5], vec![0.0, 1.0],
            vec![1.0, 1.0], vec![1.0, 0.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let optimal: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let opts = SAOptions {
            heuristic: HeuristicOptions { epochs: 0, ..HeuristicOptions::default() },
            min_temperature: 1_000_000.0,
            max_temperature: 0.0,
            ..SAOptions::default()
        };
        let problem = TspProblem { cities, distances: dm, initial_tour: Some(optimal.clone()) };
        let result = solve(&problem, &opts, None);
        assert_eq!(result.route(), optimal.as_slice());
    }

    #[test]
    fn test_is_acceptable_always_accepts_improvement() {
        let temperature = 0.001;
        assert!(is_acceptable(temperature, 100.0, 50.0));
        assert!(is_acceptable(temperature, 100.0, 99.999));
    }

    #[test]
    fn test_is_acceptable_never_accepts_equal_distance() {
        let temperature = 1_000_000.0;
        assert!(!is_acceptable(temperature, 10.0, 10.0));
    }

    #[test]
    fn test_is_acceptable_probabilistic_for_worsening_at_high_temperature() {
        let temperature = 1_000_000.0;
        let accepted = (0..1000)
            .filter(|_| is_acceptable(temperature, 10.0, 10.001))
            .count();
        assert!(accepted > 900, "expected >90% acceptance at high T, got {accepted}/1000");
    }

    #[test]
    fn test_is_acceptable_rarely_accepts_worsening_at_low_temperature() {
        let temperature = 0.0001;
        let accepted = (0..1000)
            .filter(|_| is_acceptable(temperature, 10.0, 20.0))
            .count();
        assert!(accepted < 100, "expected <10% acceptance at low T, got {accepted}/1000");
    }
}

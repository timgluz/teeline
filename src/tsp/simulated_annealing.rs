use std::sync::mpsc;

use rand::RngExt;

use super::probability::{cooling, metropolis};
use super::progress::ProgressMessage;
use super::route::Route;
use super::{SAOptions, Solution, TspProblem};

/// Counts loop iterations on the current thread, for tests only. Compiled out of release
/// and debug builds.
///
/// The epoch budget was previously unobservable without timing the process, which is why a
/// bound that never took effect went unnoticed — a timing assertion is swamped by fixed
/// startup cost on small instances. Counting iterations makes the bound directly assertable.
///
/// Thread-local rather than a global: `cargo test` runs tests in parallel, and a shared
/// counter would mix iterations from concurrently-running tests.
#[cfg(test)]
thread_local! {
    pub(crate) static ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub fn solve(
    problem: &TspProblem,
    opts: &SAOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
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

    let mut best_route = init_tour
        .map(Route::new)
        .unwrap_or_else(|| Route::from_cities(cities));
    let mut best_distance = distances.tour_length(best_route.route());

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(
            best_route.clone(),
            best_distance,
        ));
    }

    let mut temperature = opts.max_temperature;
    // `&&` so both bounds are strict caps: the run ends at whichever expires first. With
    // `||` the run continued until the *last* one expired, so `epochs` acted as a floor
    // rather than a budget — at defaults the geometric cooling schedule needs ~138k
    // iterations (1000 -> 0.001 at rate 0.0001) while `epochs` defaults to 10k, meaning
    // `--epochs` could not shorten a run at all. Meanwhile a small `epochs` truncated the
    // run while the temperature was still near its maximum, so no annealing happened.
    while epoch < opts.heuristic.epochs && temperature > opts.min_temperature {
        #[cfg(test)]
        ITERATIONS.with(|n| n.set(n.get() + 1));

        let candidate = best_route.random_successor();
        let candidate_distance = distances.tour_length(candidate.route());

        if is_acceptable(temperature, best_distance, candidate_distance) {
            best_route = candidate;
            best_distance = candidate_distance;

            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::PathUpdate(
                    best_route.clone(),
                    best_distance,
                ));
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
    use crate::tsp::{HeuristicOptions, SAOptions, TspProblem, distance_matrix, kdtree};

    #[test]
    fn test_sa_respects_initial_tour() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let optimal: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let opts = SAOptions {
            heuristic: HeuristicOptions {
                epochs: 0,
                ..HeuristicOptions::default()
            },
            min_temperature: 1_000_000.0,
            max_temperature: 0.0,
            ..SAOptions::default()
        };
        let problem = TspProblem::new(cities, dm);
        let result = solve(&problem, &opts, None, Some(&optimal));
        assert_eq!(result.route(), optimal.as_slice());
    }

    /// The epoch budget is a strict cap, not a floor.
    ///
    /// Regression guard for a real defect: with `while epoch < epochs || temperature >
    /// min_temperature`, the loop ran until *both* bounds expired. The geometric cooling
    /// schedule (rate 0.0001, 1000 -> 0.001) needs ~138k iterations, far more than the default
    /// 10k epochs, so raising `--epochs` from 10k to 50k to 138k changed nothing and the flag
    /// could not bound a run.
    ///
    /// This asserts iterations, not wall time: on small instances the fixed startup cost swamps
    /// the compute, which is why timing-based checks missed this.
    #[test]
    fn test_sa_epochs_is_a_strict_cap() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![10.0, 0.0],
            vec![20.0, 5.0],
            vec![30.0, 1.0],
            vec![40.0, 8.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);

        // A budget far below the ~138k the temperature schedule would take, so only the epoch
        // bound can stop this run.
        let epochs = 1_000;
        let opts = SAOptions {
            heuristic: HeuristicOptions {
                epochs,
                ..HeuristicOptions::default()
            },
            ..SAOptions::default()
        };

        ITERATIONS.with(|n| n.set(0));
        let _ = solve(&problem, &opts, None, None);
        let iterations = ITERATIONS.with(|n| n.get());

        assert_eq!(
            iterations, epochs,
            "epochs must cap the run: expected exactly {epochs} iterations, ran {iterations}. \
             A count far above {epochs} means the epoch bound is being OR-ed with the \
             temperature schedule instead of AND-ed, so raising --epochs cannot shorten a run."
        );
    }

    /// The temperature schedule still terminates a run on its own when the epoch budget is
    /// effectively unbounded, so making `epochs` a strict cap did not remove the cooling bound.
    #[test]
    fn test_sa_temperature_schedule_still_terminates() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![10.0, 0.0],
            vec![20.0, 5.0],
            vec![30.0, 1.0],
            vec![40.0, 8.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);

        // High cooling rate so the schedule reaches min_temperature quickly, and an epoch
        // budget large enough not to be the binding constraint.
        let opts = SAOptions {
            heuristic: HeuristicOptions {
                epochs: 10_000,
                ..HeuristicOptions::default()
            },
            cooling_rate: 0.01,
            min_temperature: 0.001,
            max_temperature: 1_000.0,
        };

        ITERATIONS.with(|n| n.set(0));
        let _ = solve(&problem, &opts, None, None);
        let iterations = ITERATIONS.with(|n| n.get());

        assert!(
            iterations > 0 && iterations < 10_000,
            "the cooling schedule should stop the run before the epoch budget: ran {iterations}"
        );
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
        assert!(
            accepted > 900,
            "expected >90% acceptance at high T, got {accepted}/1000"
        );
    }

    #[test]
    fn test_is_acceptable_rarely_accepts_worsening_at_low_temperature() {
        let temperature = 0.0001;
        let accepted = (0..1000)
            .filter(|_| is_acceptable(temperature, 10.0, 20.0))
            .count();
        assert!(
            accepted < 100,
            "expected <10% acceptance at low T, got {accepted}/1000"
        );
    }
}

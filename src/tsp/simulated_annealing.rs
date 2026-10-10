use std::sync::mpsc;

use rand::RngExt;

use super::probability::{cooling, metropolis};
use super::progress::ProgressMessage;
use super::route::Route;
use super::{SAOptions, Solution, TspProblem};

// Counts loop iterations on the current thread, for tests only. Compiled out of all non-test
// builds.
//
// The epoch budget was previously unobservable without timing the process, which is how a bound
// that never took effect went unnoticed: a timing assertion is swamped by fixed startup cost on
// small instances. Counting iterations makes the bound directly assertable.
//
// Thread-local rather than a global, because `cargo test` runs tests in parallel and a shared
// counter would mix iterations from concurrently-running tests.
#[cfg(test)]
thread_local! {
    pub(crate) static ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Iterations the geometric cooling schedule needs to go from `max_temperature` to
/// `min_temperature`: `ln(max/min) / -ln(1 - cooling_rate)`, matching the loop's
/// `t <- t * (1 - cooling_rate)`.
///
/// This is the bound a user's `epochs` has to exceed for the temperature schedule to be the
/// rule that ends the run. It is a function of the cooling parameters, so it must be derived
/// rather than hardcoded: at the defaults it is ~138,149, but `--cooling_rate=0.00001` needs
/// ~1.4M, and a hardcoded cap would silently truncate that run while still hot.
pub(crate) fn schedule_length(opts: &SAOptions) -> usize {
    let rate = opts.cooling_rate as f64;
    let max_t = opts.max_temperature as f64;
    let min_t = opts.min_temperature as f64;
    if rate <= 0.0 || rate >= 1.0 || max_t <= 0.0 || min_t <= 0.0 || max_t <= min_t {
        return 0;
    }
    let n = (max_t / min_t).ln() / -(1.0 - rate).ln();
    if !n.is_finite() || n <= 0.0 {
        return 0;
    }
    n.ceil() as usize
}

/// Resolves the epoch budget the loop actually applies.
///
/// Precedence, highest first:
///
/// 1. `epochs == 0` means **unbounded** — the temperature schedule alone decides. Zero must keep
///    this meaning because the old `||` gave it that by accident, and a bare `&&` would otherwise
///    read it as "zero iterations" and silently return the initial tour unchanged.
/// 2. An `epochs` **above** the schedule length is a genuine cap and is honoured exactly, so a
///    user can extend a run beyond the schedule.
/// 3. Anything else — the generic default, or a value below the schedule — resolves to the
///    schedule length. Truncating before the schedule finishes stops the run while the
///    temperature is still high, which accepts nearly every move and degenerates into a random
///    walk; that is a quality regression the user did not ask for and cannot see. A budget that
///    asked for this is reported at warn level so the behaviour is not silent.
///
/// The schedule length depends on the cooling rate (`0.0001` needs ~138k iterations, `0.00001`
/// needs ~1.38M), which is why no constant can serve here and why this is resolved at run time
/// rather than stored in a constructor.
fn usable_epochs(opts: &SAOptions) -> usize {
    let epochs = opts.heuristic.epochs;
    if epochs == 0 {
        return usize::MAX;
    }
    let schedule = schedule_length(opts).max(1);
    if epochs < schedule {
        tracing::warn!(
            requested_epochs = epochs,
            schedule_length = schedule,
            "SA: epoch budget is below the cooling schedule length, so it is raised to the \
             schedule length; the run would otherwise stop while the temperature is still high. \
             Pass a larger value to extend it, or 0 for no cap."
        );
        return schedule;
    }
    epochs
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

    // Resolved before logging so the log reports the budget actually used. Resolving it here,
    // rather than in a constructor, is what makes CLI, TOML, wasm and API paths agree.
    let epoch_limit = usable_epochs(opts);

    tracing::info!(
        epochs = epoch_limit,
        requested_epochs = opts.heuristic.epochs,
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
    // `&&`, and the epoch bound is resolved by `usable_epochs` (it is raised to the schedule
    // length when smaller, which is why `epoch < epoch_limit` is not the bound that usually
    // expires). With `||` the loop instead ran until the *last* bound expired, so `epochs` acted
    // as a floor rather than a budget: at defaults the schedule needs ~138k iterations while the
    // default was 10k, so `--epochs` could not shorten a run at all.
    //
    // `epochs == 0` means "no epoch cap" — the temperature schedule decides — which is what the
    // previous `||` gave it by accident. With a bare `&&` it would instead mean *zero* iterations
    // and silently return the initial tour unchanged.
    while epoch < epoch_limit && temperature > opts.min_temperature {
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

    fn tiny_problem() -> TspProblem {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![10.0, 0.0],
            vec![20.0, 5.0],
            vec![30.0, 1.0],
            vec![40.0, 8.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        TspProblem::new(cities, dm)
    }

    /// A budget below the schedule length is raised to it, so a run cannot be truncated while the
    /// temperature is still high.
    ///
    /// Regression guard in both directions. Originally `||` made the loop run
    /// `max(epochs, schedule)`, so the budget was ignored entirely — 10k, 50k and 138k all did
    /// identical work — while a *low* value truncated the run into a random walk. A bare `&&` then
    /// inverted the failure, making a low value a real truncation.
    ///
    /// The deliberate trade-off is that `--epochs` cannot shorten a run: the default budget is
    /// itself small relative to the schedule, so "I want a short run" is indistinguishable from
    /// "I accepted the default". Raising is the safe choice, and `epochs == 0` still means no cap.
    #[test]
    fn test_sa_epochs_below_the_schedule_is_raised_to_it() {
        let problem = tiny_problem();
        let schedule = schedule_length(&SAOptions::default());
        assert!(
            schedule > 1_000,
            "test premise: the default schedule should exceed 1k iterations, got {schedule}"
        );

        let opts = SAOptions {
            heuristic: HeuristicOptions {
                epochs: 1_000,
                ..HeuristicOptions::default()
            },
            ..SAOptions::default()
        };

        ITERATIONS.with(|n| n.set(0));
        let _ = solve(&problem, &opts, None, None);
        let iterations = ITERATIONS.with(|n| n.get());

        // Tolerance, not equality: `schedule_length` is computed in f64 from the analytic form,
        // while the loop cools in f32 (`t - rate * t`) and accumulates rounding over ~138k steps.
        // The real count can therefore differ from the analytic figure by a few iterations, and an
        // exact assertion would be platform-dependent.
        let tolerance = schedule / 100;
        assert!(
            iterations <= schedule && iterations + tolerance >= schedule,
            "a budget below the schedule must be raised to it: ran {iterations}, schedule {schedule}"
        );
    }

    /// `epochs == 0` means "no epoch cap": the temperature schedule decides. With a bare `&&` it
    /// would instead mean zero iterations and silently return the initial tour unchanged, which
    /// is a real regression because `||` previously made 0 unbounded by accident.
    #[test]
    fn test_sa_zero_epochs_means_unbounded_not_zero_iterations() {
        let problem = tiny_problem();
        let opts = SAOptions {
            heuristic: HeuristicOptions {
                epochs: 0,
                ..HeuristicOptions::default()
            },
            ..SAOptions::default()
        };

        ITERATIONS.with(|n| n.set(0));
        let _ = solve(&problem, &opts, None, None);
        let iterations = ITERATIONS.with(|n| n.get());

        assert!(
            iterations > 1_000,
            "epochs == 0 must mean unbounded, not zero iterations (ran {iterations})"
        );
    }

    /// A run with an explicit budget above the schedule length is capped there, and never by the
    /// schedule outlasting it — the `||` defect in the other direction.
    #[test]
    fn test_sa_temperature_bound_ends_a_run_with_a_larger_budget() {
        let problem = tiny_problem();
        // The explicit budget exceeds the schedule, so `usable_epochs` leaves it as given and the
        // temperature bound ends the run first. Kept deliberately fast: a slow cooling rate would
        // derive a multi-million-iteration schedule, which is correct but slow to execute.
        let opts = SAOptions {
            heuristic: HeuristicOptions {
                epochs: 5_000,
                ..HeuristicOptions::default()
            },
            cooling_rate: 0.01,
            ..SAOptions::default()
        };
        let schedule = schedule_length(&opts);
        assert!(
            schedule < 5_000,
            "test premise: schedule {schedule} should be below the explicit budget"
        );

        ITERATIONS.with(|n| n.set(0));
        let _ = solve(&problem, &opts, None, None);
        let iterations = ITERATIONS.with(|n| n.get());

        // Same f32-vs-f64 reasoning as above, so a tolerance rather than equality.
        assert!(
            iterations <= schedule,
            "the run must not exceed the schedule: ran {iterations}, schedule {schedule}"
        );
        assert!(
            schedule - iterations <= schedule / 100,
            "the run should end at the schedule, not far short of it: \
             ran {iterations}, schedule {schedule}"
        );
    }

    /// `schedule_length` is derived from the cooling parameters rather than hardcoded, and a
    /// degenerate schedule falls back to the explicit budget instead of clamping it to zero.
    #[test]
    fn test_sa_schedule_length_follows_the_cooling_rate() {
        // The cap must be derived from the cooling parameters, not hardcoded: a slower rate needs
        // far more iterations, and a fixed cap would silently truncate that run while still hot.
        // A hardcoded 150k would cover rate 0.0001 but not 0.00001.
        let with_rate = |rate: f32| SAOptions {
            cooling_rate: rate,
            ..SAOptions::default()
        };

        let fast = schedule_length(&with_rate(0.01));
        let default = schedule_length(&with_rate(0.0001));
        let slow = schedule_length(&with_rate(0.00001));

        assert!(
            (1_300..=1_500).contains(&fast),
            "rate 0.01 should need ~1375 iterations, got {fast}"
        );
        assert_eq!(default, 138_149, "rate 0.0001 has a known schedule length");
        assert!(
            slow > 1_000_000,
            "rate 0.00001 needs >1M iterations, so a fixed 150k cap cannot be correct: got {slow}"
        );

        // A degenerate schedule must not produce a cap of 0, which would run zero iterations.
        let broken = SAOptions {
            cooling_rate: 1.0,
            ..SAOptions::default()
        };
        assert_eq!(
            schedule_length(&broken),
            0,
            "invalid rate yields no schedule"
        );
        let opts = SAOptions {
            cooling_rate: 1.0,
            heuristic: HeuristicOptions {
                epochs: 10,
                ..HeuristicOptions::default()
            },
            ..SAOptions::default()
        };
        assert_eq!(
            usable_epochs(&opts),
            10,
            "a degenerate schedule must fall back to the explicit budget, not clamp it to 0"
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

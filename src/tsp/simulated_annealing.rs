use std::sync::mpsc;

use rand::RngExt;

use super::probability::{cooling, metropolis};
use super::progress::ProgressMessage;
use super::route::Route;
use super::{SAOptions, Solution, TspProblem};

/// Counts loop iterations on the current thread, for tests only. Compiled out of all
/// non-test builds.
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

/// Safety cap applied when `epochs` is not explicitly bounded: `epochs == 0` means unbounded,
/// and an `epochs` below the schedule length would truncate the run while it is still hot.
///
/// Kept here rather than in any single constructor so CLI, TOML, wasm and API paths agree —
/// the default previously lived only in `from_cli`, leaving every other constructor on the
/// generic 10k value and reintroducing the regression this guard exists to prevent.
fn usable_epochs(opts: &SAOptions) -> usize {
    if opts.heuristic.epochs == 0 {
        return usize::MAX;
    }
    opts.heuristic.epochs.max(schedule_length(opts).max(1))
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
    // Resolved once, so every constructor path (CLI, TOML, wasm, API) agrees. See
    // `usable_epochs` for why this cannot live in `SAOptions::default()`.
    let epoch_limit = usable_epochs(opts);

    // `&&` so both bounds are strict caps: the run ends at whichever expires first. With
    // `||` the run continued until the *last* one expired, so `epochs` acted as a floor
    // rather than a budget — at defaults the geometric cooling schedule needs ~138k
    // iterations (1000 -> 0.001 at rate 0.0001) while `epochs` defaulted to 10k, meaning
    // `--epochs` could not shorten a run at all. Meanwhile a small `epochs` truncated the
    // run while the temperature was still near its maximum, so no annealing happened.
    //
    // `epochs == 0` means "no epoch cap" — the temperature schedule decides — which is what
    // the previous `||` gave it by accident. With a bare `&&` it would instead mean *zero*
    // iterations and silently return the initial tour unchanged.
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

    #[test]
    fn test_sa_epochs_below_the_schedule_is_clamped() {
        // Regression guard. With `||` the epoch bound was a floor: raising it from 10k to 50k to
        // 138k changed nothing, because the geometric schedule (rate 0.0001, 1000 -> 0.001) needs
        // ~138,149 iterations. With a bare `&&` the opposite failure appeared: a user-supplied
        // 1,000 cap truncated the run at T ~= 990 of 1000, which accepts nearly every move and
        // degenerates into a random walk (measured on a280: ~32k tours vs ~3.4k).
        //
        // So an epoch budget below the schedule length is raised to it, and the run still ends on
        // the temperature bound rather than being cut off while hot.
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

        assert_eq!(
            iterations, schedule,
            "an epochs value below the schedule length must be raised to it, so the run is not \
             truncated while the temperature is still high"
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
    fn test_sa_epochs_above_the_schedule_is_respected() {
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

        assert_eq!(
            iterations, schedule,
            "the temperature schedule should end this run before the explicit budget"
        );
    }

    /// The temperature schedule still terminates a run on its own when the epoch budget is
    /// effectively unbounded, so making `epochs` a strict cap did not remove the cooling bound.
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

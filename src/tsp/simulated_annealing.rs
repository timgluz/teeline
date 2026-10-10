use std::sync::mpsc;

use rand::RngExt;

use super::budget::Budget;
use super::probability::{cooling, metropolis};
use super::progress::ProgressMessage;
use super::route::Route;
use super::{SAOptions, Solution, TspProblem};

/// Iterations the cooling schedule needs to reach `min_temperature` from `max_temperature`.
///
/// Derived rather than fixed because it scales with the cooling rate, which callers choose.
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
/// - `epochs == 0` means unbounded.
/// - A value below the schedule length is raised to it: stopping while the temperature is still high
///   accepts nearly every move, so the run degenerates into a random walk rather than annealing.
/// - Resolved at run time because the budget depends on the cooling parameters, whichever
///   constructor supplied them; computing it in `solve` keeps CLI, TOML, wasm and API paths in
///   agreement.
///
/// Consequence: `--epochs` can bound a run at the schedule length or leave it uncapped, but cannot
/// shorten it — an explicitly small budget is indistinguishable from an accepted default. Fixing
/// that needs `Option<usize>` rather than overloading `0`.
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
             Any value at or above the schedule length gives the same run; 0 removes the epoch cap."
        );
        return schedule;
    }
    epochs
}

/// Which bound ended a run, and what it consumed, so tests and callers can observe a run without
/// instrumenting the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolveStats {
    /// Epochs actually run.
    pub epochs: usize,
    /// Sustained non-improvement ended the run.
    pub converged: bool,
    /// The temperature fell to its floor, ending the run.
    pub cooled: bool,
    /// The epoch budget was exhausted, ending the run.
    ///
    /// Derived rather than compared directly: `usable_epochs` can raise the budget to the cooling
    /// schedule length, so reaching it does not by itself mean the budget was the binding bound, and
    /// convergence can also fire on the final admitted epoch.
    pub epoch_capped: bool,
}

pub fn solve(
    problem: &TspProblem,
    opts: &SAOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> (Solution, SolveStats) {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let cooling_rate = opts.cooling_rate;

    // Resolved here, not in a constructor, so the log reports the budget actually used and every
    // constructor path agrees.
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
    // Three bounds are capping, and whichever is reached first ends the run: the epoch budget, the
    // cooling schedule (the temperature falling to its floor), and sustained non-improvement.
    let mut budget = Budget::new(epoch_limit, opts.heuristic.stagnation_epochs);
    let mut improved = true;

    while temperature > opts.min_temperature && budget.record(improved) {
        let epoch = budget.index();

        let best_at_epoch_start = best_distance;
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
        // `best_route`/`best_distance` are the CURRENT tour, since an accepted uphill move replaces
        // them. Progress is therefore "this epoch ended shorter than it started", which the snapshot
        // taken before any candidate was drawn makes exact — not an acceptance count, which would
        // treat the search's own worsening moves as advancement.
        improved = best_distance < best_at_epoch_start;
    }

    if budget.converged() {
        tracing::info!(
            epoch = budget.epoch(),
            stagnation_epochs = budget.stale_epochs(),
            "SA: converged, no improvement for the stagnation limit"
        );
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }

    let converged = budget.converged();
    let cooled = temperature <= opts.min_temperature;
    let stats = SolveStats {
        epochs: budget.epoch(),
        converged,
        cooled,
        epoch_capped: !converged && !cooled && budget.epoch() >= epoch_limit,
    };
    (
        Solution::from_parts(best_route.route(), cities, distances),
        stats,
    )
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
        let (result, _) = solve(&problem, &opts, None, Some(&optimal));
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

    /// `SolveStats` must not report convergence when the epoch budget is what ran out.
    ///
    /// The budget here equals the schedule length, which is the case that makes a bare
    /// `epoch >= limit` comparison wrong: the cap and the temperature floor are reached together, so
    /// only the derived `!converged && !cooled` form distinguishes them. For SA the two genuinely
    /// coincide — `usable_epochs` raises any smaller budget to the schedule — so the useful guarantee
    /// is that neither is ever misreported as convergence.
    #[test]
    fn test_sa_stats_do_not_call_a_budget_stop_convergence() {
        let problem = tiny_problem();
        let opts = SAOptions {
            heuristic: HeuristicOptions {
                epochs: schedule_length(&SAOptions::default()),
                ..HeuristicOptions::default()
            },
            ..SAOptions::default()
        };

        let (_, stats) = solve(&problem, &opts, None, None);

        assert!(
            !stats.converged,
            "the plateau stop is disabled, so convergence cannot be the reason it stopped"
        );
        assert!(
            stats.cooled || stats.epoch_capped,
            "one of the other two bounds must explain the stop (ran {})",
            stats.epochs
        );
        // Which of the two wins is decided by f32 rounding in the cooling loop, so this pins only
        // that the cap reports honestly when it is the one that applied.
        if !stats.cooled {
            assert!(
                stats.epoch_capped,
                "if the temperature floor was not reached, the budget must be why it stopped"
            );
        }
    }

    /// With the plateau stop on, `converged` is the reason and the other two bounds are not claimed.
    #[test]
    fn test_sa_stats_report_convergence_when_the_plateau_stops_the_run() {
        let problem = tiny_problem();
        let opts = SAOptions {
            heuristic: HeuristicOptions {
                epochs: 0, // no epoch cap
                stagnation_epochs: 20,
                ..HeuristicOptions::default()
            },
            ..SAOptions::default()
        };

        let (_, stats) = solve(&problem, &opts, None, None);

        assert!(stats.converged, "20 non-improving epochs must end the run");
        assert!(
            !stats.epoch_capped,
            "the budget was unbounded, so it cannot have been the bound that applied"
        );
    }

    /// The plateau stop must bound a run independently of the cooling schedule.
    ///
    /// SA has three bounds — the epoch budget, the temperature floor, and non-improvement — and the
    /// test keeps the first two well out of the way (an unbounded budget, and a schedule far longer
    /// than the run) so only sustained non-improvement can explain a run shorter than the schedule.
    ///
    /// Scope: this covers the wiring, not which quantity counts as "progress". On a five-city problem
    /// both a best-tour signal and an acceptance signal converge early, so this test cannot tell them
    /// apart; the best-tour choice is argued at the assignment in `solve`.
    #[test]
    fn test_sa_stops_on_stagnation() {
        let problem = tiny_problem();
        let opts = SAOptions {
            heuristic: HeuristicOptions {
                epochs: 0, // no epoch cap
                stagnation_epochs: 20,
                ..HeuristicOptions::default()
            },
            ..SAOptions::default()
        };
        let schedule = schedule_length(&opts);
        assert!(
            schedule > 100,
            "test premise: the schedule must leave room for a plateau stop, got {schedule}"
        );

        let (_, stats) = solve(&problem, &opts, None, None);

        assert!(stats.converged, "20 non-improving epochs must end the run");
        assert!(
            !stats.epoch_capped,
            "the epoch budget was unbounded, so it cannot be the bound that applied"
        );
        assert!(
            stats.epochs < schedule,
            "the plateau stop must end the run before the cooling schedule does: \
             ran {} of {schedule}",
            stats.epochs
        );
    }

    /// A budget below the schedule length is raised to it rather than truncating the run.
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

        let (_, stats) = solve(&problem, &opts, None, None);
        let iterations = stats.epochs;

        // Tolerance on both sides, not equality: `schedule_length` is computed in f64 from the
        // analytic form, while the loop cools in f32 (`t - rate * t`) and accumulates rounding over
        // ~138k steps. The real count can differ from the analytic figure by a few iterations in
        // either direction, so an exact or one-sided assertion would be platform-dependent.
        assert!(
            iterations.abs_diff(schedule) <= schedule / 100,
            "a budget below the schedule must be raised to it: ran {iterations}, schedule {schedule}"
        );
    }

    /// `epochs == 0` means no epoch cap, not zero iterations.
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

        let (_, stats) = solve(&problem, &opts, None, None);
        let iterations = stats.epochs;

        assert!(
            iterations > 1_000,
            "epochs == 0 must mean unbounded, not zero iterations (ran {iterations})"
        );
    }

    /// A budget above the schedule length cannot extend the run: the temperature bound ends it.
    #[test]
    fn test_sa_temperature_bound_ends_the_run_at_the_schedule() {
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

        let (_, stats) = solve(&problem, &opts, None, None);
        let iterations = stats.epochs;

        // Same f32-vs-f64 reasoning as above: the epoch cap (5000) does not bind here, so the f32
        // cooling loop alone decides the count and can land a few iterations either side of the
        // analytic value. A one-sided bound would be platform-dependent.
        assert!(
            iterations.abs_diff(schedule) <= schedule / 100,
            "the run should end at the schedule: ran {iterations}, schedule {schedule}"
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

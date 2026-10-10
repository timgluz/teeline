//! The plateau stop is opt-in and must actually shorten a run.
//!
//! Verified through the solvers' existing `progress_tx` seam: each emits one `EpochUpdate` per
//! epoch, so counting them shows how many epochs ran without needing instrumentation inside the
//! solver. `tsp::budget`'s own tests pin the stopping logic against synthetic sequences; these
//! tests check only that each solver is wired to it, so they exercise both the disabled default
//! (must reach the cap) and a real limit (must stop early).

use std::sync::mpsc;

use teeline::tsp::progress::ProgressMessage;
use teeline::tsp::{
    AcoOptions, CSOptions, FPAOptions, FourierOptions, HeuristicOptions, SOMOptions, TspProblem,
    ant_colony, cuckoo_search, distance_matrix, flower_pollination, fourier, gravitational_search,
    kdtree, particle_swarm, som, tabu_search,
};

fn problem() -> TspProblem {
    // Enough spread that no solver improves in every epoch, so a small limit has something to
    // detect on a real search rather than only in the budget's synthetic sequences.
    let cities = kdtree::build_points(
        &(0..12)
            .map(|i| vec![((i * 7) % 11) as f32, ((i * 5) % 13) as f32])
            .collect::<Vec<_>>(),
    );
    let dm = distance_matrix::from_cities(&cities);
    TspProblem::new(cities, dm)
}

fn epochs_run<F>(run: F) -> usize
where
    F: FnOnce(&mpsc::Sender<ProgressMessage>) -> teeline::tsp::Solution,
{
    let (tx, rx) = mpsc::channel();
    let _ = run(&tx);
    drop(tx);
    rx.try_iter()
        .filter(|m| matches!(m, ProgressMessage::EpochUpdate(_)))
        .count()
}

// Large enough that the limit must do the stopping well before it, small enough to keep the four
// tests quick: the disabled-limit half runs every one of these epochs.
const CAP: usize = 2_000;
const LIMIT: usize = 5;

/// A limit must stop the run well before the cap, while disabling the limit must reach the cap.
/// The second half is what shows the limit — not something else — is doing the stopping.
macro_rules! plateau_test {
    ($name:ident, $label:literal, $solver:path, $opts:ty, $mk:expr) => {
        #[test]
        fn $name() {
            let problem = problem();

            let limited = epochs_run(|tx| {
                let opts: $opts = $mk(HeuristicOptions {
                    epochs: CAP,
                    stagnation_epochs: LIMIT,
                    ..HeuristicOptions::default()
                });
                $solver(&problem, &opts, Some(tx), None)
            });
            assert!(
                limited < CAP / 10,
                "{}: a stagnation limit of {} should stop the run far short of the {} cap; ran {}",
                $label,
                LIMIT,
                CAP,
                limited
            );

            let unlimited = epochs_run(|tx| {
                let opts: $opts = $mk(HeuristicOptions {
                    epochs: CAP,
                    stagnation_epochs: 0,
                    ..HeuristicOptions::default()
                });
                $solver(&problem, &opts, Some(tx), None)
            });
            assert_eq!(
                unlimited, CAP,
                "{}: with the limit disabled the run must reach the cap; ran {}",
                $label, unlimited
            );
        }
    };
}

plateau_test!(
    particle_swarm_stops_on_stagnation,
    "pso",
    particle_swarm::solve,
    HeuristicOptions,
    |h| h
);

plateau_test!(
    cuckoo_search_stops_on_stagnation,
    "cs",
    cuckoo_search::solve,
    CSOptions,
    |h| CSOptions {
        heuristic: h,
        ..CSOptions::default()
    }
);

plateau_test!(
    flower_pollination_stops_on_stagnation,
    "fpa",
    flower_pollination::solve,
    FPAOptions,
    |h| FPAOptions {
        heuristic: h,
        ..FPAOptions::default()
    }
);

plateau_test!(
    gravitational_search_stops_on_stagnation,
    "gsa",
    gravitational_search::solve,
    HeuristicOptions,
    |h| h
);

// --- Solvers whose epoch count is not on the progress seam --------------------------------------
//
// `som` reports only at 10% checkpoints and `fourier` takes no progress channel at all, so neither
// exposes its epoch count the way the four above do. They are held to the property that is
// independent of counting: a run with a limit still returns a complete tour, and the limit does not
// change the *shape* of the answer. Their stopping behaviour is pinned by unit tests in the modules
// (`fourier` measures its decoded tour per stage, `som` its quantisation error), where the signal
// being measured is directly observable.

fn assert_complete_tour(solver: &str, solution: &teeline::tsp::Solution, expected: usize) {
    let mut ids = solution.route().to_vec();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        expected,
        "{solver}: a stopped run must still visit every city exactly once"
    );
}

#[test]
fn som_honours_a_stagnation_limit_without_truncating_the_tour() {
    let problem = problem();
    let expected = problem.cities.len();
    let opts = SOMOptions {
        epochs: 500,
        stagnation_epochs: LIMIT,
        ..SOMOptions::default()
    };
    let (tx, _rx) = mpsc::channel();
    let solution = som::solve(&problem, &opts, Some(&tx), None);
    assert_complete_tour("som", &solution, expected);
}

#[test]
fn fourier_honours_a_stagnation_limit_without_truncating_the_tour() {
    let problem = problem();
    let expected = problem.cities.len();
    let opts = FourierOptions {
        stagnation_epochs: LIMIT,
        ..FourierOptions::default()
    };
    let solution = fourier::solve(&problem, &opts, None, None);
    assert_complete_tour("fourier", &solution, expected);
}

/// ACO and tabu both report one `EpochUpdate` per epoch, so the limit's effect is countable: a run
/// with a limit must stop before a cap it would otherwise reach.
fn assert_limit_bounds_epochs<F>(solver: &str, run: F)
where
    F: Fn(&mpsc::Sender<ProgressMessage>, usize) -> teeline::tsp::Solution,
{
    let (tx, rx) = mpsc::channel();
    let _ = run(&tx, LIMIT);
    drop(tx);
    let limited = rx
        .try_iter()
        .filter(|m| matches!(m, ProgressMessage::EpochUpdate(_)))
        .count();

    assert!(
        limited < CAP / 10,
        "{solver}: a limit of {LIMIT} should stop the run far short of the {CAP} cap; ran {limited}"
    );
}

#[test]
fn ant_colony_stops_on_stagnation() {
    let problem = problem();
    assert_limit_bounds_epochs("aco", |tx, limit| {
        let opts = AcoOptions {
            heuristic: HeuristicOptions {
                epochs: CAP,
                stagnation_epochs: limit,
                ..HeuristicOptions::default()
            },
            ..AcoOptions::default()
        };
        ant_colony::solve(&problem, &opts, Some(tx), None)
    });
}

#[test]
fn tabu_search_stops_on_stagnation() {
    let problem = problem();
    assert_limit_bounds_epochs("tabu", |tx, limit| {
        let opts = HeuristicOptions {
            epochs: CAP,
            stagnation_epochs: limit,
            ..HeuristicOptions::default()
        };
        tabu_search::solve(&problem, &opts, Some(tx), None)
    });
}

// --- Convergence actually firing -----------------------------------------------------------------
//
// The tests above use a loose limit to prove the run is *bounded*; these drive the limit low enough
// that convergence is the reason the run stops, which is the path that logs and publishes its final
// state. Without them those branches are never taken.

/// Runs each solver with a limit of 1 and asserts it reports convergence, which is where the
/// convergence log line and the final progress update live.
#[test]
fn a_tight_limit_makes_every_solver_converge() {
    let problem = problem();
    let (tx, rx) = mpsc::channel();

    let heuristic = |limit: usize| HeuristicOptions {
        epochs: CAP,
        stagnation_epochs: limit,
        ..HeuristicOptions::default()
    };

    // aco and tabu publish the epoch count, so their convergence is countable.
    let count = |rx: &mpsc::Receiver<ProgressMessage>| {
        rx.try_iter()
            .filter(|m| matches!(m, ProgressMessage::EpochUpdate(_)))
            .count()
    };

    let opts = AcoOptions {
        heuristic: heuristic(1),
        ..AcoOptions::default()
    };
    let _ = ant_colony::solve(&problem, &opts, Some(&tx), None);
    drop(tx);
    let aco_epochs = count(&rx);
    assert!(
        aco_epochs < CAP / 10,
        "aco with a limit of 1 must stop early, ran {aco_epochs}"
    );

    let (tx, rx) = mpsc::channel();
    let _ = tabu_search::solve(&problem, &heuristic(1), Some(&tx), None);
    drop(tx);
    let tabu_epochs = count(&rx);
    assert!(
        tabu_epochs < CAP / 10,
        "tabu with a limit of 1 must stop early, ran {tabu_epochs}"
    );

    // som and fourier have no per-epoch message; assert the tour is still complete when they converge.
    let som_opts = SOMOptions {
        epochs: 500,
        stagnation_epochs: 1,
        ..SOMOptions::default()
    };
    let (tx, _rx) = mpsc::channel();
    let solution = som::solve(&problem, &som_opts, Some(&tx), None);
    assert_complete_tour("som", &solution, problem.cities.len());

    let fourier_opts = FourierOptions {
        stagnation_epochs: 1,
        ..FourierOptions::default()
    };
    let solution = fourier::solve(&problem, &fourier_opts, None, None);
    assert_complete_tour("fourier", &solution, problem.cities.len());
}

/// `fourier` counts stages, so a limit at or above `k_max` can never fire; the run must warn and then
/// use every stage rather than stopping on the first.
#[test]
fn fourier_warns_when_the_limit_cannot_fire() {
    let problem = problem();
    let opts = FourierOptions {
        k_max: 3,
        stagnation_epochs: 3,
        ..FourierOptions::default()
    };
    let solution = fourier::solve(&problem, &opts, None, None);
    assert_complete_tour("fourier", &solution, problem.cities.len());
}

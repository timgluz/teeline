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
    CSOptions, FPAOptions, HeuristicOptions, TspProblem, cuckoo_search, distance_matrix,
    flower_pollination, gravitational_search, kdtree, particle_swarm,
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

//! Regression tests for `nn`'s run-to-run variance.
//!
//! `nn` used to return different tours across process invocations of `data/tsplib/a280.tsp`
//! (three distinct costs were reachable: 3148.11, 3168.95, 3551.81). Two defects
//! caused it:
//!
//! 1. The greedy step asked for "the nearest unvisited among the k nearest overall",
//!    which is not the nearest unvisited. Whenever all k nearest were already
//!    visited it fell through to a second, differently-defined selection path, so
//!    the result depended on how often that happened.
//! 2. That fallback resolved ties with `min_by` over a `HashSet`, and `min_by`
//!    returns the first element *in iteration order* when all comparisons are equal.
//!    `HashSet` iteration order varies per process, so exactly-tied cities were
//!    chosen nondeterministically. a280 is a drilling grid with 71 exact ties among
//!    its 279 greedy steps, which is why it exposed the bug so readily.
//!
//! An in-process repeat cannot catch this: `HashSet`'s seed is per process, so the
//! bug only appears across invocations. These tests therefore (a) assert `nn` matches
//! an independent greedy reference — which the k-limited approximation violates — and
//! (b) repeat the solve to catch any reintroduced per-call nondeterminism.

use std::collections::HashSet;
use std::path::Path;

use teeline::tsp::{HeuristicOptions, TspProblem, nearest_neighbor, tsplib};

fn load(name: &str) -> TspProblem {
    let path = Path::new("tests/fixtures").join(name);
    let data =
        tsplib::read_from_file(&path).unwrap_or_else(|e| panic!("failed to read {name}: {e}"));
    let cities = data.cities().to_vec();
    // Must go through TspLibData::distance_matrix(): it honours the file's declared
    // EDGE_WEIGHT_TYPE and uses raw_distances for EXPLICIT. `distance_matrix::from_cities`
    // silently applies the default EUC_2D, which turns an EXPLICIT matrix instance into a
    // coordinate one and yields a meaningless tour — gr17 scores 21.16 by Euclidean
    // distance against 2187 by its declared matrix.
    let dm = data
        .distance_matrix()
        .unwrap_or_else(|e| panic!("failed to build distance matrix for {name}: {e}"));
    TspProblem::new(cities, dm)
}

fn solve_nn(problem: &TspProblem) -> teeline::tsp::Solution {
    nearest_neighbor::solve(problem, &HeuristicOptions::default(), None, None)
}

/// Independent reference implementation of greedy nearest neighbour.
///
/// Deliberately written against the public API in the most obvious way possible:
/// scan every unvisited city, take the minimum by `(distance, city_id)`. It shares
/// no code with the solver, so agreeing with it is meaningful.
fn reference_greedy_nn(problem: &TspProblem) -> Vec<usize> {
    let cities = &problem.cities;
    let dm = &problem.distances;

    let mut unvisited: HashSet<usize> = cities.iter().map(|c| c.id).collect();
    let start = cities[0].id;
    let mut route = vec![start];
    unvisited.remove(&start);

    while !unvisited.is_empty() {
        let current = *route.last().unwrap();
        let mut best: Option<(f32, usize)> = None;
        for &candidate in &unvisited {
            let d = dm.distance_between(current, candidate).unwrap_or(f32::MAX);
            let better = match best {
                None => true,
                Some((bd, bi)) => (d, candidate) < (bd, bi),
            };
            if better {
                best = Some((d, candidate));
            }
        }
        let (_, next) = best.expect("unvisited is non-empty");
        route.push(next);
        unvisited.remove(&next);
    }
    route
}

/// The core regression: `nn` must pick the *true* nearest unvisited city at every step.
///
/// The previous k-limited implementation returned a different (and longer) tour here —
/// 3168.95 or 3551.81 instead of 3148.11 on a280.
#[test]
fn nn_matches_greedy_reference_on_a280() {
    let problem = load("a280.tsp");
    let solution = solve_nn(&problem);
    let expected = reference_greedy_nn(&problem);

    assert_eq!(
        solution.route(),
        expected.as_slice(),
        "nn diverged from a plain greedy nearest-neighbour reference"
    );
}

#[test]
fn nn_matches_greedy_reference_on_berlin52() {
    let problem = load("berlin52.tsp");
    let solution = solve_nn(&problem);
    assert_eq!(solution.route(), reference_greedy_nn(&problem).as_slice());
}

/// Repeating the solve must not change the answer.
///
/// Weaker than cross-process determinism (see the module docs), but it does catch a
/// reintroduced per-call source of variation, and it is the assertion the original
/// code would already have needed.
#[test]
fn nn_is_stable_across_repeated_solves() {
    let problem = load("a280.tsp");
    let first = solve_nn(&problem);

    for attempt in 2..=5 {
        let again = solve_nn(&problem);
        assert_eq!(
            again.total, first.total,
            "nn returned a different cost on attempt {attempt}"
        );
        assert_eq!(
            again.route(),
            first.route(),
            "route changed on attempt {attempt}"
        );
    }
}

/// `nn` seeds these solvers via `auto_expand_with_nn`, so it must be fixed for them to
/// be reproducible. This pins the coupling at the tour level rather than the flag level.
#[test]
fn nn_seeded_two_opt_is_stable() {
    use teeline::tsp::two_opt;

    let problem = load("a280.tsp");
    let seed = solve_nn(&problem).route().to_vec();

    let first = two_opt::solve(&problem, &HeuristicOptions::default(), None, Some(&seed));
    for attempt in 2..=3 {
        let again = two_opt::solve(&problem, &HeuristicOptions::default(), None, Some(&seed));
        assert_eq!(
            again.total, first.total,
            "2-opt seeded from a fixed nn tour differed on attempt {attempt}"
        );
    }
}

/// Guards the property directly: at every step of the produced tour, the next city must
/// be a nearest unvisited city from the current one — no farther alternative may exist.
#[test]
fn every_greedy_step_chooses_a_nearest_unvisited_city() {
    let problem = load("a280.tsp");
    let route = solve_nn(&problem).route().to_vec();
    let dm = &problem.distances;

    for window in route.windows(2) {
        let (from, chosen) = (window[0], window[1]);
        let chosen_dist = dm.distance_between(from, chosen).unwrap_or(f32::MAX);

        // Everything visited strictly before `chosen` is unavailable; the rest must not
        // offer a strictly closer city (or an equal distance with a lower id).
        let index = route.iter().position(|&c| c == chosen).unwrap();
        let already_visited: HashSet<usize> = route[..index].iter().copied().collect();

        for &other in route.iter().skip(index) {
            if already_visited.contains(&other) {
                continue;
            }
            let other_dist = dm.distance_between(from, other).unwrap_or(f32::MAX);
            assert!(
                (other_dist, other) >= (chosen_dist, chosen),
                "at step ending in {chosen}, city {other} was at least as good \
                 (d={other_dist} vs {chosen_dist}) — greedy choice was not nearest"
            );
        }
    }
}

/// The fix must hold on every edge-weight code path, not just EUC_2D.
///
/// `nn` was only ever verified on EUC_2D instances, so a regression specific to another
/// metric would have gone unnoticed. These fixtures cover the other types the corpus
/// actually uses: EXPLICIT (a raw lower-diagonal matrix) and ATT (pseudo-Euclidean).
///
/// Costs are asserted as reproducible and plausible rather than pinned to exact
/// constants, so a future algorithmic improvement does not fail the suite.
#[test]
fn nn_is_stable_and_sane_on_explicit_and_att_instances() {
    for (fixture, optimum) in [("gr17.tsp", 2085.0_f32), ("att48.tsp", 33523.71_f32)] {
        let problem = load(fixture);
        let first = solve_nn(&problem);

        // A valid tour over every city.
        let mut seen = first.route().to_vec();
        seen.sort_unstable();
        let mut expected_ids: Vec<usize> = problem.cities.iter().map(|c| c.id).collect();
        expected_ids.sort_unstable();
        assert_eq!(seen, expected_ids, "{fixture}: tour is not a permutation");

        // Matches the independent greedy reference under the *declared* metric.
        assert_eq!(
            first.route(),
            reference_greedy_nn(&problem).as_slice(),
            "{fixture}: diverged from the greedy reference"
        );

        // Reproducible across repeated solves.
        for _ in 0..3 {
            assert_eq!(
                solve_nn(&problem).total,
                first.total,
                "{fixture}: unstable across repeated solves"
            );
        }

        // Within the band a greedy construction can plausibly reach. This is the
        // assertion that would catch the metric being wrong: under Euclid, gr17's
        // tour costs 21.16 against an optimum of 2085, a nonsense -99% gap.
        let gap_pct = (first.total - optimum) / optimum * 100.0;
        assert!(
            (0.0..=60.0).contains(&gap_pct),
            "{fixture}: gap {gap_pct:.1}% is implausible for greedy NN (optimum {optimum})"
        );
    }
}

/// A minimal instance with an exact distance tie, pinning the tie-break rule.
///
/// Every other fixture in this crate is either tie-free or only asserted to visit all
/// cities, so a regression back to hash-ordered or `n_nearest`-limited selection could
/// pass them. This one fails such a regression directly.
///
/// Geometry: city 0 at the origin, city 1 at (1,0), city 2 at (-1,0). From city 0,
/// cities 1 and 2 are *exactly* 1.0 away — a genuine float tie, not a near-tie — so only
/// the tie-break decides the route. Lowest id must win, giving [0, 1, 2].
#[test]
fn nn_breaks_exact_ties_by_lowest_city_id() {
    use teeline::tsp::kdtree;

    let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![-1.0, 0.0]]);
    let dm = teeline::tsp::distance_matrix::from_cities(&cities);
    let problem = TspProblem::new(cities.clone(), dm);

    let first = solve_nn(&problem);
    assert_eq!(
        first.route(),
        &[cities[0].id, cities[1].id, cities[2].id],
        "an exact tie must resolve to the lowest city id"
    );

    // Stable across repeats, which is what hash-ordered selection could not guarantee.
    for _ in 0..5 {
        assert_eq!(solve_nn(&problem).route(), first.route());
    }

    // `n_nearest` must not influence `nn` at all: it is a candidate-list limit for other
    // solvers, and using it here is exactly what made the greedy step approximate.
    let with_small_k = HeuristicOptions {
        n_nearest: 1,
        ..Default::default()
    };
    let with_large_k = HeuristicOptions {
        n_nearest: 1000,
        ..Default::default()
    };
    assert_eq!(
        nearest_neighbor::solve(&problem, &with_small_k, None, None).route(),
        nearest_neighbor::solve(&problem, &with_large_k, None, None).route(),
        "n_nearest changed the route; nn must ignore it"
    );
}

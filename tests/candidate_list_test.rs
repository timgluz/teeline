//! Candidate-list quality and trade-off tests for two-opt and or-opt.
//!
//! The candidate list is the default for coordinate inputs; these tests pin the resulting tour
//! quality against the published optima and against the full-neighbourhood run. The solvers are
//! seeded with nearest-neighbor, matching the CLI's auto-expansion.

use std::path::Path;

use teeline::tsp::{HeuristicOptions, TspProblem, nearest_neighbor, or_opt, tsplib, two_opt};

fn problem(fixture: &str) -> TspProblem {
    let path = Path::new("tests/fixtures").join(fixture);
    let data =
        tsplib::read_from_file(&path).unwrap_or_else(|e| panic!("failed to read {fixture}: {e}"));
    TspProblem::new(data.cities().to_vec(), data.distance_matrix().unwrap())
}

fn nn_seed(problem: &TspProblem) -> Vec<usize> {
    nearest_neighbor::solve(problem, &HeuristicOptions::default(), None, None)
        .route()
        .to_vec()
}

fn full_neighbourhood() -> HeuristicOptions {
    HeuristicOptions {
        n_nearest: 1000,
        ..HeuristicOptions::default()
    }
}

#[test]
fn two_opt_candidate_list_matches_the_documented_a280_gap() {
    // Published optimum 2579; 12% = 2888.5.
    let problem = problem("a280.tsp");
    let seed = nn_seed(&problem);
    let solution = two_opt::solve(&problem, &HeuristicOptions::default(), None, Some(&seed));
    assert!(solution.total > 0.0);
    assert!(
        solution.total <= 2889.0,
        "a280 two-opt candidate-list cost {} exceeds 12% over 2579",
        solution.total
    );
}

#[test]
fn or_opt_candidate_list_matches_the_documented_a280_gap() {
    // Published optimum 2579; 13% = 2914.3.
    let problem = problem("a280.tsp");
    let seed = nn_seed(&problem);
    let solution = or_opt::solve(&problem, &HeuristicOptions::default(), None, Some(&seed));
    assert!(solution.total > 0.0);
    assert!(
        solution.total <= 2914.0,
        "a280 or-opt candidate-list cost {} exceeds 13% over 2579",
        solution.total
    );
}

#[test]
fn two_opt_candidate_list_is_within_two_percent_of_full_on_a280() {
    let problem = problem("a280.tsp");
    let seed = nn_seed(&problem);
    let full = two_opt::solve(&problem, &full_neighbourhood(), None, Some(&seed));
    let candidates = two_opt::solve(&problem, &HeuristicOptions::default(), None, Some(&seed));
    assert!(
        candidates.total <= full.total * 1.02,
        "two-opt candidate list {} vs full {}",
        candidates.total,
        full.total
    );
}

#[test]
fn or_opt_candidate_list_is_within_two_percent_of_full_on_a280() {
    let problem = problem("a280.tsp");
    let seed = nn_seed(&problem);
    let full = or_opt::solve(&problem, &full_neighbourhood(), None, Some(&seed));
    let candidates = or_opt::solve(&problem, &HeuristicOptions::default(), None, Some(&seed));
    assert!(
        candidates.total <= full.total * 1.02,
        "or-opt candidate list {} vs full {}",
        candidates.total,
        full.total
    );
}

#[test]
#[ignore = "slow: att532"]
fn candidate_lists_match_the_documented_att532_gaps() {
    // Published optimum 27686; two-opt 15% = 31838.9, or-opt 18% = 32669.5.
    let problem = problem("att532.tsp");
    let seed = nn_seed(&problem);

    let two = two_opt::solve(&problem, &HeuristicOptions::default(), None, Some(&seed));
    assert!(
        two.total <= 31839.0,
        "att532 two-opt candidate-list cost {} exceeds 15% over 27686",
        two.total
    );

    let or = or_opt::solve(&problem, &HeuristicOptions::default(), None, Some(&seed));
    assert!(
        or.total <= 32670.0,
        "att532 or-opt candidate-list cost {} exceeds 18% over 27686",
        or.total
    );
}

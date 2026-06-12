/// Integration tests for the Or-opt TSP solver.
///
/// Fast tests run berlin52 with the default HeuristicOptions for structural
/// validation. The quality test is marked #[ignore] and can be run with:
///   cargo test --test or_opt_test -- --include-ignored
use std::path::Path;
use teeline::tsp::{HeuristicOptions, TspProblem, distance_matrix, kdtree, or_opt, tsplib};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn load_tsp(fixture: &str) -> tsplib::TspLibData {
    let path = Path::new("tests/fixtures").join(fixture);
    tsplib::read_from_file(&path).unwrap_or_else(|e| panic!("failed to read {fixture}: {e}"))
}

fn make_problem(cities: Vec<kdtree::KDPoint>) -> TspProblem {
    let dm = distance_matrix::from_cities(&cities);
    TspProblem::new(cities, dm)
}

fn is_valid_tour(route: &[usize], cities: &[kdtree::KDPoint]) -> bool {
    let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
    expected.sort_unstable();
    let mut got = route.to_vec();
    got.sort_unstable();
    got == expected
}

// ─── fast structural tests ────────────────────────────────────────────────────

#[test]
fn or_opt_berlin52_returns_valid_tour() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let sol = or_opt::solve(&problem, &HeuristicOptions::default(), None, None);
    assert_eq!(sol.route().len(), 52, "tour must visit all 52 cities");
    assert!(
        is_valid_tour(sol.route(), &cities),
        "tour must contain every city exactly once"
    );
}

#[test]
fn or_opt_berlin52_reported_distance_matches_tour() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let dm = distance_matrix::from_cities(&cities);
    let sol = or_opt::solve(&problem, &HeuristicOptions::default(), None, None);

    let route = sol.route();
    let n = route.len();
    let recomputed: f32 = (0..n)
        .map(|i| {
            dm.distance_between(route[i], route[(i + 1) % n])
                .unwrap_or(f32::MAX)
        })
        .sum();

    assert!(
        (sol.total - recomputed).abs() < 1.0,
        "reported total ({:.1}) must match recomputed tour length ({:.1})",
        sol.total,
        recomputed,
    );
}

#[test]
fn or_opt_berlin52_improves_over_identity_tour() {
    // Identity order (city IDs in original order) is a poor starting tour;
    // Or-opt should always find improvements on it.
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let identity: Vec<usize> = cities.iter().map(|c| c.id).collect();
    let identity_cost = problem.distances.tour_length(&identity);

    let sol = or_opt::solve(&problem, &HeuristicOptions::default(), None, Some(&identity));
    assert!(
        sol.total < identity_cost,
        "or-opt ({:.1}) should improve over identity tour ({:.1})",
        sol.total,
        identity_cost,
    );
}

// ─── quality test (berlin52) — run with --include-ignored ────────────────────

/// Optimal tour cost for berlin52 is 7542. 2-opt achieves ~50% gap tolerance.
/// Or-opt with a NN seed should also stay within 50% of optimal.
/// Run with: cargo test --test or_opt_test -- --include-ignored
#[test]
#[ignore = "slow quality check; run with: cargo test --test or_opt_test -- --include-ignored"]
fn or_opt_quality_berlin52() {
    use teeline::tsp::nearest_neighbor;

    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());

    // Warm-start with NN tour (mirrors how the CLI auto-expands)
    let nn_sol = nearest_neighbor::solve(&problem, &HeuristicOptions::default(), None, None);
    let sol = or_opt::solve(
        &problem,
        &HeuristicOptions::default(),
        None,
        Some(nn_sol.route()),
    );

    // Target: within 50% of optimal (7542) = ≤11313, matching the 2-opt bound.
    assert!(
        sol.total <= 11313.0,
        "or-opt quality check: got {:.1}, want ≤11313 (≤50% above optimal 7542)",
        sol.total,
    );
}

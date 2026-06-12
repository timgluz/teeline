/// Integration tests for the Christofides approximation TSP solver.
///
/// Fast tests run berlin52 with default options for structural validation.
/// The quality test is marked #[ignore] and can be run with:
///   cargo test --test christofides_test -- --include-ignored
use std::path::Path;
use teeline::tsp::{HeuristicOptions, TspProblem, christofides, distance_matrix, kdtree, tsplib};

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
fn christofides_berlin52_returns_valid_tour() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let sol = christofides::solve(&problem, &HeuristicOptions::default(), None, None);
    assert_eq!(sol.route().len(), 52, "tour must visit all 52 cities");
    assert!(
        is_valid_tour(sol.route(), &cities),
        "tour must contain every city exactly once"
    );
}

#[test]
fn christofides_berlin52_reported_distance_matches_tour() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let dm = distance_matrix::from_cities(&cities);
    let sol = christofides::solve(&problem, &HeuristicOptions::default(), None, None);

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

/// Christofides guarantees ≤1.5× optimal on EUC_2D.
/// Known optimal for berlin52 = 7542; bound = 11313.
#[test]
fn christofides_berlin52_within_approximation_bound() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities);
    let sol = christofides::solve(&problem, &HeuristicOptions::default(), None, None);
    assert!(
        sol.total <= 11313.0,
        "Christofides must stay within 1.5× optimal (7542): got {:.1}, want ≤11313",
        sol.total,
    );
}

/// Empirical quality floor: Christofides on berlin52 consistently produces tours
/// around 8500–9500. Assert ≤10000 to catch regressions without depending on
/// random variation (the algorithm is fully deterministic).
#[test]
fn christofides_berlin52_empirical_quality() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities);
    let sol = christofides::solve(&problem, &HeuristicOptions::default(), None, None);
    assert!(
        sol.total <= 10000.0,
        "Christofides empirical quality regression: got {:.1}, want ≤10000",
        sol.total,
    );
}

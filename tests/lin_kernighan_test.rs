/// Integration tests for the Lin-Kernighan TSP solver.
///
/// Fast tests use berlin52 with epochs=5 for quick structural validation.
/// The quality test is marked #[ignore] and can be run with:
///   cargo test --test lin_kernighan_test -- --include-ignored
use std::path::Path;
use teeline::tsp::{
    HeuristicOptions, LKOptions, TspProblem, distance_matrix, kdtree, lin_kernighan,
    nearest_neighbor, tsplib,
};

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

fn lk_opts_fast() -> LKOptions {
    LKOptions {
        heuristic: HeuristicOptions {
            epochs: 5,
            platoo_epochs: 5,
            n_nearest: 5,
            verbose: false,
        },
        max_depth: 5,
    }
}

fn nn_tour_cost(problem: &TspProblem) -> f32 {
    nearest_neighbor::solve(problem, &HeuristicOptions::default(), None, None).total
}

// ─── fast structural tests (berlin52, 52 cities, epochs=5) ───────────────────

#[test]
fn lk_solve_returns_valid_tour() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let sol = lin_kernighan::solve(&problem, &lk_opts_fast(), None, None);
    assert_eq!(sol.route().len(), 52, "tour must visit all 52 cities");
    assert!(
        is_valid_tour(sol.route(), &cities),
        "tour must contain every city exactly once"
    );
}

#[test]
fn lk_solve_improves_on_nn_seed() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities);
    let nn_cost = nn_tour_cost(&problem);
    let lk_sol = lin_kernighan::solve(&problem, &lk_opts_fast(), None, None);
    assert!(
        lk_sol.total < nn_cost,
        "LK ({:.1}) must improve over NN ({:.1}) even with epochs=5",
        lk_sol.total,
        nn_cost,
    );
}

#[test]
fn lk_solve_reported_distance_matches_tour() {
    // Verify that the `total` field reported by the solver is consistent with
    // the tour returned in `route()`. We recompute tour length manually via the
    // distance matrix and compare within a small epsilon.
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let dm = distance_matrix::from_cities(&cities);
    let sol = lin_kernighan::solve(&problem, &lk_opts_fast(), None, None);

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

// ─── quality test (berlin52, epochs=200) — run with --include-ignored ────────

/// Optimal tour cost for berlin52 is 7542.
/// This ILS-2opt implementation consistently achieves ~8-9% gap (~8100-8300).
/// True ≤2% gap requires depth-3 LK or better (see GH #184).
/// Run with: cargo test --test lin_kernighan_test -- --include-ignored
#[test]
#[ignore = "slow quality check; run with: cargo test --test lin_kernighan_test -- --include-ignored"]
fn lk_solve_quality_berlin52() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities);
    let opts = LKOptions {
        heuristic: HeuristicOptions {
            epochs: 200,
            platoo_epochs: 20,
            n_nearest: 5,
            verbose: false,
        },
        max_depth: 5,
    };
    let sol = lin_kernighan::solve(&problem, &opts, None, None);
    // Known optimal for berlin52 is 7542; this ILS-2opt achieves ~8-9% gap (~8100-8300).
    // Threshold ≤8500 (+12.7%) gives CI headroom while still catching broken implementations.
    assert!(
        sol.total <= 8500.0,
        "LK quality check failed: got {:.1}, want ≤8500 (~+12.7% above optimal 7542)",
        sol.total,
    );
}

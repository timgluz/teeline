/// Integration tests for the 3-opt local search solver.
///
/// Fast tests use gr17 (17 cities → 680 triples per pass vs 22 100 for berlin52).
/// Slow tests on berlin52 and att48 are marked #[ignore] and can be run with:
///   cargo test --test three_opt_test -- --include-ignored
use std::path::Path;
use teeline::tsp::{distance_matrix, kdtree, nearest_neighbor, three_opt, tsplib, SolverOptions};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn load_tsp(fixture: &str) -> tsplib::TspLibData {
    let path = Path::new("tests/fixtures").join(fixture);
    tsplib::read_from_file(&path)
        .unwrap_or_else(|e| panic!("failed to read {fixture}: {e}"))
}

fn build_dm(cities: &[kdtree::KDPoint]) -> distance_matrix::DistanceMatrix {
    distance_matrix::from_cities(cities)
}

fn is_valid_tour(route: &[usize], cities: &[kdtree::KDPoint]) -> bool {
    let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
    expected.sort_unstable();
    let mut got = route.to_vec();
    got.sort_unstable();
    got == expected
}

fn nn_tour_length(cities: &[kdtree::KDPoint]) -> f32 {
    let dm = build_dm(cities);
    nearest_neighbor::solve(cities, &dm, &SolverOptions::default()).total
}

// ─── fast integration tests (gr17, 17 cities) ────────────────────────────────

#[test]
fn three_opt_valid_tour_gr17() {
    let cities = load_tsp("gr17.tsp").cities().to_vec();
    let dm = build_dm(&cities);
    let tour = three_opt::solve(&cities, &dm, &SolverOptions::default());
    assert!(is_valid_tour(tour.route(), &cities), "gr17 tour is invalid");
}

#[test]
fn three_opt_improves_on_gr17() {
    let cities = load_tsp("gr17.tsp").cities().to_vec();
    let dm = build_dm(&cities);
    let three_opt_total = three_opt::solve(&cities, &dm, &SolverOptions::default()).total;
    let nn_total = nn_tour_length(&cities);
    assert!(
        three_opt_total < nn_total,
        "3-opt ({three_opt_total:.1}) must improve over NN ({nn_total:.1}) on gr17"
    );
}

// ─── slow integration tests (berlin52, att48) — run with --include-ignored ──

/// O(n³) with n=52 is slow in debug mode; use `cargo test -- --include-ignored` to run.
#[test]
#[ignore = "slow in debug mode (~minutes); run with: cargo test --test three_opt_test -- --include-ignored"]
fn three_opt_valid_tour_berlin52() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let dm = build_dm(&cities);
    let tour = three_opt::solve(&cities, &dm, &SolverOptions::default());
    assert!(is_valid_tour(tour.route(), &cities), "berlin52 tour is invalid");
}

#[test]
#[ignore = "slow in debug mode (~minutes); run with: cargo test --test three_opt_test -- --include-ignored"]
fn three_opt_improves_on_berlin52() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let dm = build_dm(&cities);
    let three_opt_total = three_opt::solve(&cities, &dm, &SolverOptions::default()).total;
    let nn_total = nn_tour_length(&cities);
    assert!(
        three_opt_total < nn_total,
        "3-opt ({three_opt_total:.1}) must improve over NN ({nn_total:.1}) on berlin52"
    );
}

#[test]
#[ignore = "slow in debug mode (~minutes); run with: cargo test --test three_opt_test -- --include-ignored"]
fn three_opt_valid_tour_att48() {
    let cities = load_tsp("att48.tsp").cities().to_vec();
    let dm = build_dm(&cities);
    let tour = three_opt::solve(&cities, &dm, &SolverOptions::default());
    assert!(is_valid_tour(tour.route(), &cities), "att48 tour is invalid");
}

#[test]
#[ignore = "slow in debug mode (~minutes); run with: cargo test --test three_opt_test -- --include-ignored"]
fn three_opt_improves_on_att48() {
    let cities = load_tsp("att48.tsp").cities().to_vec();
    let dm = build_dm(&cities);
    let three_opt_total = three_opt::solve(&cities, &dm, &SolverOptions::default()).total;
    let nn_total = nn_tour_length(&cities);
    assert!(
        three_opt_total < nn_total,
        "3-opt ({three_opt_total:.1}) must improve over NN ({nn_total:.1}) on att48"
    );
}

/// Integration tests for the Kohonen SOM TSP solver.
///
/// Fast tests use minimal settings (epochs=500) for quick structural validation.
/// The quality test is #[ignore] and requires: cargo test --test som_test -- --include-ignored
use std::path::Path;
use teeline::tsp::{SOMOptions, TspProblem, distance_matrix, kdtree, tsplib};

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

fn fast_opts() -> SOMOptions {
    SOMOptions {
        epochs: 500,
        ..SOMOptions::default()
    }
}

#[test]
fn som_valid_tour_berlin52() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let sol = teeline::tsp::som::solve(&problem, &fast_opts(), None, None);
    assert_eq!(sol.route().len(), 52, "tour must visit all 52 cities");
    assert!(
        is_valid_tour(sol.route(), &cities),
        "tour must contain every city exactly once"
    );
    assert!(sol.total > 0.0, "tour distance must be positive");
    assert!(sol.total.is_finite(), "tour distance must be finite");
}

/// Run with: cargo test --test som_test -- --include-ignored
#[test]
#[ignore = "slow quality check; run with: cargo test --test som_test -- --include-ignored"]
fn som_quality_berlin52() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let opts = SOMOptions::default(); // 100_000 epochs
    let sol = teeline::tsp::som::solve(&problem, &opts, None, None);
    assert!(
        is_valid_tour(sol.route(), &cities),
        "quality run must still produce a valid tour"
    );
    // SOM typically reaches 5–15% gap; threshold ≤20% gives headroom above optimal 7544.
    let threshold = 7_544.37 * 1.20;
    assert!(
        sol.total <= threshold,
        "SOM quality check: got {:.1}, want ≤{:.1} (~+20% above optimal 7544)",
        sol.total,
        threshold,
    );
}

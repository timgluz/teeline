/// Integration tests for the Fourier-basis TSP solver.
///
/// Fast tests use minimal settings (k_max=2, m=50, epochs=10) for quick structural validation.
/// The quality test is #[ignore] and requires: cargo test --test fourier_test -- --include-ignored
use std::path::Path;
use teeline::tsp::{FourierOptions, TspProblem, distance_matrix, kdtree, tsplib};

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

fn fast_opts() -> FourierOptions {
    FourierOptions {
        k_max: 2,
        m: 50,
        epochs: 10,
        ..FourierOptions::default()
    }
}

// ─── fast structural tests ────────────────────────────────────────────────────

#[test]
fn fourier_valid_tour_berlin52() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let sol = teeline::tsp::fourier::solve(&problem, &fast_opts(), None, None);
    assert_eq!(sol.route().len(), 52, "tour must visit all 52 cities");
    assert!(
        is_valid_tour(sol.route(), &cities),
        "tour must contain every city exactly once"
    );
    assert!(sol.total > 0.0, "tour distance must be positive");
    assert!(sol.total.is_finite(), "tour distance must be finite");
}

#[test]
fn fourier_valid_tour_small() {
    // Cities with non-contiguous IDs to validate the argsort → city ID mapping
    let cities: Vec<kdtree::KDPoint> = vec![5usize, 10, 15, 20, 25]
        .into_iter()
        .enumerate()
        .map(|(i, id)| {
            use std::f32::consts::PI;
            kdtree::KDPoint {
                id,
                coords: [
                    (2.0 * PI * i as f32 / 5.0).cos(),
                    (2.0 * PI * i as f32 / 5.0).sin(),
                ],
            }
        })
        .collect();
    let problem = make_problem(cities.clone());
    let sol = teeline::tsp::fourier::solve(&problem, &fast_opts(), None, None);
    let mut got = sol.route().to_vec();
    got.sort_unstable();
    assert_eq!(
        got,
        vec![5, 10, 15, 20, 25],
        "tour must contain original city IDs [5,10,15,20,25], not array positions [0..4]"
    );
}

// ─── slow quality test ────────────────────────────────────────────────────────

/// Run with: cargo test --test fourier_test -- --include-ignored
#[test]
#[ignore = "slow quality check; run with: cargo test --test fourier_test -- --include-ignored"]
fn fourier_quality_berlin52() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let opts = FourierOptions {
        k_max: 4,
        m: 200,
        epochs: 400,
        ..FourierOptions::default()
    };
    let sol = teeline::tsp::fourier::solve(&problem, &opts, None, None);
    assert!(
        is_valid_tour(sol.route(), &cities),
        "quality run must still produce a valid tour"
    );
    // Fourier consistently produces 8549 (+13.3%) on berlin52 with default settings.
    // Threshold ≤9200 (+21.9%) gives ~7% headroom above observed cost.
    assert!(
        sol.total <= 9_200.0,
        "Fourier quality check: got {:.1}, want ≤9200 (~+21.9% above optimal 7544); typical is ~8549",
        sol.total,
    );
}

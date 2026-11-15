//! Acceptance tests for derived layouts on coordinate-less EXPLICIT instances.
//!
//! `gr17` has no `NODE_COORD_SECTION` or `DISPLAY_DATA_SECTION`, so its city layout is derived
//! from the explicit distance matrix. The geometric solvers must produce valid tours from that
//! layout, and must beat the placeholder-grid order the parser used before this change.

use std::path::Path;
use teeline::tsp::{
    DistanceBackend, FourierOptions, HeuristicOptions, SOMOptions, SfcOptions, TspProblem,
    branch_bound, fourier, kdtree::KDPoint, som, space_filling_curve, tsplib,
};

fn load(name: &str) -> tsplib::TspLibData {
    let path = Path::new("tests/fixtures").join(name);
    tsplib::read_from_file(&path).unwrap_or_else(|e| panic!("failed to read {name}: {e}"))
}

fn is_valid(route: &[usize], problem: &TspProblem) -> bool {
    let mut expected: Vec<usize> = problem.cities.iter().map(|city| city.id).collect();
    expected.sort_unstable();
    let mut actual = route.to_vec();
    actual.sort_unstable();
    actual == expected
}

/// The old parser's placeholder layout: a `ceil(sqrt(n))`-column grid of unit cells.
fn placeholder_grid(n: usize) -> Vec<KDPoint> {
    let columns = (n as f64).sqrt().ceil() as usize;
    (0..n)
        .map(|i| KDPoint::new_with_id(i + 1, &[(i % columns) as f32, (i / columns) as f32]))
        .collect()
}

#[test]
fn sfc_on_gr17_beats_the_placeholder_grid_tour() {
    let data = load("gr17.tsp");
    let matrix = data.distance_matrix().unwrap();
    let placeholder = TspProblem::new(placeholder_grid(17), matrix);
    let baseline =
        space_filling_curve::solve(&placeholder, &SfcOptions::default(), None, None).total;

    let problem = data.to_problem(DistanceBackend::Auto).unwrap();
    let solution = space_filling_curve::solve(&problem, &SfcOptions::default(), None, None);
    assert!(is_valid(solution.route(), &problem), "sfc tour is invalid");
    assert!(
        solution.total < baseline,
        "derived SFC {:.1} did not beat the placeholder baseline {:.1}",
        solution.total,
        baseline
    );
}

#[test]
fn fourier_and_som_return_valid_tours_on_gr17() {
    let problem = load("gr17.tsp").to_problem(DistanceBackend::Auto).unwrap();

    let fourier_solution = fourier::solve(
        &problem,
        &FourierOptions {
            k_max: 2,
            m: 50,
            epochs: 20,
            ..Default::default()
        },
        None,
        None,
    );
    assert!(
        is_valid(fourier_solution.route(), &problem),
        "fourier tour is invalid"
    );

    let som_solution = som::solve(
        &problem,
        &SOMOptions {
            epochs: 500,
            ..Default::default()
        },
        None,
        None,
    );
    assert!(
        is_valid(som_solution.route(), &problem),
        "som tour is invalid"
    );
}

#[test]
fn branch_bound_returns_a_valid_tour_on_gr17() {
    let problem = load("gr17.tsp").to_problem(DistanceBackend::Auto).unwrap();
    let solution = branch_bound::solve(
        &problem,
        &HeuristicOptions {
            n_nearest: 3,
            ..Default::default()
        },
        None,
        None,
    );
    assert!(
        is_valid(solution.route(), &problem),
        "branch_bound tour is invalid"
    );
}

#[test]
fn berlin52_keeps_its_file_coordinates() {
    let data = load("berlin52.tsp");
    assert!(data.has_real_coordinates());
    let original = data.cities().to_vec();
    let problem = data.to_problem(DistanceBackend::Auto).unwrap();
    assert_eq!(problem.cities, original);
}

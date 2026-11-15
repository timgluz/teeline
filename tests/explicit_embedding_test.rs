//! Acceptance tests for derived layouts on coordinate-less EXPLICIT instances.
//!
//! `gr17` has no `NODE_COORD_SECTION` or `DISPLAY_DATA_SECTION`, so its city layout is derived
//! from the explicit distance matrix. The geometric solvers must produce valid tours from that
//! layout, and must beat the placeholder-grid order the parser used before this change.

use std::path::Path;
use teeline::tsp::{
    AcoOptions, AppOptions, CSOptions, DistanceBackend, FPAOptions, FourierOptions, GAOptions,
    HeuristicOptions, LKOptions, SAOptions, SOMOptions, SfcOptions, Solvers, TspProblem,
    branch_bound, fourier, kdtree::KDPoint, lin_kernighan, nearest_neighbor, solve_problem, som,
    space_filling_curve, tsplib,
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

#[test]
fn lin_kernighan_on_gr17_uses_exact_matrix_candidates() {
    let problem = load("gr17.tsp").to_problem(DistanceBackend::Auto).unwrap();
    let options = LKOptions {
        seed: Some(42),
        ..Default::default()
    };
    let first = lin_kernighan::solve(&problem, &options, None, None);
    let second = lin_kernighan::solve(&problem, &options, None, None);
    assert!(is_valid(first.route(), &problem), "lk tour is invalid");
    assert_eq!(
        first.route(),
        second.route(),
        "seeded lk must be deterministic"
    );

    let nearest = nearest_neighbor::solve(&problem, &HeuristicOptions::default(), None, None);
    assert!(
        first.total <= nearest.total,
        "lk ({:.1}) must not worsen the NN seed ({:.1}) on gr17",
        first.total,
        nearest.total
    );
    assert!(
        first.total < 4125.0,
        "lk ({:.1}) must beat the placeholder-coordinate baseline (4125.0)",
        first.total
    );
}

#[test]
fn lin_kernighan_returns_valid_tours_on_explicit_with_display_data() {
    for name in ["bayg29.tsp", "bays29.tsp"] {
        let problem = load(name).to_problem(DistanceBackend::Auto).unwrap();
        let solution = lin_kernighan::solve(
            &problem,
            &LKOptions {
                seed: Some(7),
                ..Default::default()
            },
            None,
            None,
        );
        assert!(
            is_valid(solution.route(), &problem),
            "{name} lk tour is invalid"
        );
    }
}

#[test]
fn every_registered_solver_returns_a_valid_tour_on_gr17() {
    let problem = load("gr17.tsp").to_problem(DistanceBackend::Auto).unwrap();

    let heuristic = HeuristicOptions {
        epochs: 5,
        ..Default::default()
    };
    let options = AppOptions {
        sa: Some(SAOptions {
            heuristic: heuristic.clone(),
            cooling_rate: 0.1,
            max_temperature: 10.0,
            min_temperature: 1.0,
        }),
        ga: Some(GAOptions {
            heuristic: heuristic.clone(),
            ..Default::default()
        }),
        cs: Some(CSOptions {
            heuristic: heuristic.clone(),
            ..Default::default()
        }),
        fpa: Some(FPAOptions {
            heuristic: heuristic.clone(),
            ..Default::default()
        }),
        lk: Some(LKOptions {
            heuristic: heuristic.clone(),
            max_depth: 5,
            seed: Some(1),
        }),
        fourier: Some(FourierOptions {
            k_max: 2,
            m: 30,
            epochs: 5,
            ..Default::default()
        }),
        sfc: Some(SfcOptions {
            multi_orientation: 1,
            ..Default::default()
        }),
        som: Some(SOMOptions {
            epochs: 50,
            ..Default::default()
        }),
        aco: Some(AcoOptions {
            heuristic: heuristic.clone(),
            num_ants: 4,
            ..Default::default()
        }),
        heuristic: Some(heuristic),
    };

    for meta in Solvers::all_meta() {
        let solver = meta
            .name
            .parse::<Solvers>()
            .unwrap_or_else(|e| panic!("{}: {e}", meta.name));
        let solution = solve_problem(solver, &problem, &options)
            .unwrap_or_else(|e| panic!("{} failed: {e}", meta.name));
        assert!(
            is_valid(solution.route(), &problem),
            "{} returned an invalid tour",
            meta.name
        );
        assert!(
            solution.total.is_finite(),
            "{} returned a non-finite cost",
            meta.name
        );
    }
}

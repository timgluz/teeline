//! Compact EXPLICIT storage: width selection, exact round-trips, and solver equivalence.
//!
//! The stored representation is an implementation detail; these tests pin the user-visible
//! contract: every weight survives, tour costs are unchanged, and the narrowest exact width is
//! chosen.

use std::collections::HashMap;
use std::path::Path;

use teeline::tsp::{
    AppOptions, CityTable, DistanceBackend, Solvers, TspProblem,
    distance_matrix::DistanceMatrix,
    kdtree::{self, KDPoint},
    solve_problem, tsplib,
};

fn fixture(name: &str) -> tsplib::TspLibData {
    let path = Path::new("tests/fixtures").join(name);
    tsplib::read_from_file(&path).unwrap_or_else(|e| panic!("failed to read {name}: {e}"))
}

/// Recompute a tour directly from the packed raw weights, independently of the matrix lookup.
fn reference_cost(route: &[usize], raw: &[f32], cities: &[KDPoint]) -> f32 {
    let positions: HashMap<usize, usize> = cities
        .iter()
        .enumerate()
        .map(|(position, city)| (city.id, position))
        .collect();

    route
        .iter()
        .zip(route.iter().cycle().skip(1))
        .map(|(&a, &b)| {
            let (from, to) = {
                let (a, b) = (positions[&a], positions[&b]);
                (a.max(b), a.min(b))
            };
            raw[from * (from - 1) / 2 + to]
        })
        .sum()
}

#[test]
fn bays29_compacts_to_u16_and_solves_exactly() {
    let data = fixture("bays29.tsp");
    let raw = data.raw_distances().expect("bays29 is EXPLICIT").to_vec();

    let matrix = data.distance_matrix().expect("explicit matrix builds");
    assert_eq!(matrix.storage_kind(), "u16");
    assert_eq!(matrix.stored_bytes(), raw.len() * 2);
    assert_eq!(matrix.distances().as_ref(), raw.as_slice());

    let problem = data
        .into_problem(DistanceBackend::Auto)
        .expect("explicit problem builds");
    let solution =
        solve_problem(Solvers::NearestNeighbor, &problem, &AppOptions::default()).unwrap();

    assert_eq!(
        solution.total,
        reference_cost(solution.route(), &raw, &problem.cities),
        "the matrix cost must equal the raw-weight recomputation"
    );
}

#[test]
fn wide_explicit_weights_stay_f32_and_solve_exactly() {
    let n = 20;
    let cities = kdtree::build_points(
        &(0..n)
            .map(|i| vec![i as f32, (i % 5) as f32])
            .collect::<Vec<_>>(),
    );
    let table: CityTable = cities
        .iter()
        .enumerate()
        .map(|(position, city)| (position, *city))
        .collect();

    let mut raw = Vec::with_capacity(n * (n - 1) / 2);
    for i in 1..n {
        for j in 0..i {
            raw.push(((i * 7_919 + j * 104_729) % 1_000_000 + 65_536) as f32);
        }
    }
    assert!(raw.iter().all(|&weight| weight > 65_535.0));

    let matrix = DistanceMatrix::new(n, raw.clone(), table).unwrap();
    assert_eq!(matrix.storage_kind(), "f32");
    assert_eq!(matrix.stored_bytes(), raw.len() * 4);

    let problem = TspProblem::new(cities, matrix);
    let solution =
        solve_problem(Solvers::NearestNeighbor, &problem, &AppOptions::default()).unwrap();

    assert_eq!(
        solution.total,
        reference_cost(solution.route(), &raw, &problem.cities)
    );
}

#[test]
fn fractional_and_negative_weights_fall_back_to_f32() {
    let n = 4;
    let cities = kdtree::build_points(&(0..n).map(|i| vec![i as f32, 0.0]).collect::<Vec<_>>());
    let table: CityTable = cities
        .iter()
        .enumerate()
        .map(|(position, city)| (position, *city))
        .collect();

    let raw = vec![1.0, 2.5, 3.0, -4.0, 5.0, 6.0];
    let matrix = DistanceMatrix::new(n, raw.clone(), table).unwrap();
    assert_eq!(matrix.storage_kind(), "f32");
    assert_eq!(matrix.stored_bytes(), raw.len() * 4);
    assert_eq!(matrix.distances().as_ref(), raw.as_slice());
}

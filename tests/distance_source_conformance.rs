//! Shared conformance and equivalence suite for every `DistanceSource` backend.
//!
//! The matrix and on-the-fly backends must answer the same queries identically. The matrix
//! path is the reference; the on-the-fly path computes the same TSPLIB primitives per lookup.
//! `from_cities_unrounded` is deliberately excluded: it is a test-only fifth path that neither
//! backend represents.

use std::path::Path;
use std::sync::Arc;

use teeline::tsp::{
    AppOptions, DistanceSource, DistanceType, LKOptions, Solvers, TspProblem, distance_matrix,
    lin_kernighan, on_the_fly::OnTheFly, solve_problem, tsplib,
};

/// One fixture per coordinate metric. CEIL_2D has no tracked file, so `berlin52` coordinates
/// are measured with the CEIL_2D primitive as the synthetic CEIL_2D case.
const FIXTURES: &[(&str, DistanceType)] = &[
    ("a280.tsp", DistanceType::Euc2D),
    ("ulysses16.tsp", DistanceType::Geo),
    ("att48.tsp", DistanceType::Att),
    ("berlin52.tsp", DistanceType::Ceil2D),
];

fn load(fixture: &str) -> tsplib::TspLibData {
    let path = Path::new("tests/fixtures").join(fixture);
    tsplib::read_from_file(&path).unwrap_or_else(|e| panic!("failed to read {fixture}: {e}"))
}

fn backends(fixture: &str, dt: DistanceType) -> Vec<(&'static str, Box<dyn DistanceSource>)> {
    let data = load(fixture);
    let cities = data.cities();
    vec![
        ("matrix", Box::new(distance_matrix::build(cities, dt))),
        (
            "on-the-fly",
            Box::new(OnTheFly::new(cities, dt).expect("coordinate source builds")),
        ),
    ]
}

fn problems(fixture: &str, dt: DistanceType) -> Vec<(&'static str, TspProblem)> {
    let data = load(fixture);
    vec![
        (
            "matrix",
            TspProblem::new(
                data.cities().to_vec(),
                distance_matrix::build(data.cities(), dt),
            ),
        ),
        (
            "on-the-fly",
            TspProblem::from_source(
                data.cities().to_vec(),
                Arc::new(OnTheFly::new(data.cities(), dt).expect("coordinate source builds")),
            ),
        ),
    ]
}

#[test]
fn every_backend_agrees_on_every_pair() {
    for &(fixture, dt) in FIXTURES {
        let data = load(fixture);
        let cities = data.cities();
        let matrix = distance_matrix::build(cities, dt);
        let onfly = OnTheFly::new(cities, dt).expect("coordinate source builds");
        let n = cities.len();

        for a in 0..n {
            for b in 0..n {
                assert_eq!(
                    matrix.distance_by_pos(a, b).unwrap(),
                    onfly.distance_by_pos(a, b).unwrap(),
                    "{fixture} ({dt:?}) position pair ({a}, {b})"
                );
            }
        }

        // Nearest-neighbour results must agree too, not just raw distances.
        let target = matrix.point_at(0).unwrap();
        let matrix_near = matrix
            .nearest(&target, 3)
            .nearest()
            .iter()
            .map(|item| item.point.id)
            .collect::<Vec<_>>();
        let onfly_near = onfly
            .nearest(&target, 3)
            .nearest()
            .iter()
            .map(|item| item.point.id)
            .collect::<Vec<_>>();
        assert_eq!(matrix_near, onfly_near, "{fixture}: nearest set differs");
    }
}

#[test]
fn every_backend_obeys_the_conformance_contract() {
    for &(fixture, dt) in FIXTURES {
        for (name, source) in backends(fixture, dt) {
            let n = source.num_cities();
            assert!(source.has_coordinates(), "{fixture}/{name}");
            assert_eq!(source.tour_length(&[]), 0.0);
            assert_eq!(source.tour_length(&[0]), 0.0);

            let first_id = source.pos2city_id(0).expect("position 0 exists");
            assert_eq!(source.city_id2pos(first_id), Some(0));
            assert_eq!(source.point_at(0).unwrap().id, first_id);
            assert_eq!(source.distance_between(first_id, first_id).unwrap(), 0.0);
            assert_eq!(source.distance_by_pos(0, 0).unwrap(), 0.0);
            assert_eq!(source.distances_from(first_id).len(), n);

            assert!(
                source.distance_by_pos(n, 0).is_err(),
                "{fixture}/{name}: position == n must be out of range"
            );
            assert!(
                source.distance_between(usize::MAX, first_id).is_err(),
                "{fixture}/{name}: unknown city id must be an error"
            );
            assert_eq!(
                source.tour_length(&[usize::MAX, first_id]),
                0.0,
                "{fixture}/{name}: unknown id must not make a tour"
            );

            let target = source.point_at(0).unwrap();
            assert_eq!(source.nearest(&target, 1).nearest().len(), 1);
            assert!(
                source
                    .nearest_unvisited(&target, &|id| id != target.id)
                    .is_some()
            );
            assert!(
                source.nearest_unvisited(&target, &|_| false).is_none(),
                "{fixture}/{name}: an empty candidate set must yield None"
            );
        }
    }
}

#[test]
fn deterministic_solvers_agree_across_backends() {
    let cases = [
        (
            "berlin52.tsp",
            DistanceType::Euc2D,
            Solvers::NearestNeighbor,
        ),
        (
            "berlin52.tsp",
            DistanceType::Euc2D,
            Solvers::SpaceFillingCurve,
        ),
        ("berlin52.tsp", DistanceType::Euc2D, Solvers::TwoOpt),
        ("berlin52.tsp", DistanceType::Euc2D, Solvers::OrOpt),
        ("burma14.tsp", DistanceType::Geo, Solvers::BellmanKarp),
    ];

    for (fixture, dt, solver) in cases {
        let runs = problems(fixture, dt)
            .into_iter()
            .map(|(name, problem)| {
                let solution = solve_problem(solver, &problem, &AppOptions::default()).unwrap();
                (name, solution.route().to_vec(), solution.total)
            })
            .collect::<Vec<_>>();

        assert_eq!(
            runs[0].1, runs[1].1,
            "{fixture}: {solver:?} tour differs between {} and {}",
            runs[0].0, runs[1].0
        );
        assert_eq!(
            runs[0].2, runs[1].2,
            "{fixture}: {solver:?} cost differs between {} and {}",
            runs[0].0, runs[1].0
        );
    }
}

#[test]
fn seeded_lk_agrees_across_backends_on_geo() {
    let opts = LKOptions {
        seed: Some(7),
        ..LKOptions::default()
    };
    let runs = problems("ulysses16.tsp", DistanceType::Geo)
        .into_iter()
        .map(|(name, problem)| {
            let solution = lin_kernighan::solve(&problem, &opts, None, None);
            (name, solution.route().to_vec(), solution.total)
        })
        .collect::<Vec<_>>();

    assert_eq!(
        runs[0].1, runs[1].1,
        "seeded LK tour differs between {} and {}",
        runs[0].0, runs[1].0
    );
    assert_eq!(runs[0].2, runs[1].2);
}

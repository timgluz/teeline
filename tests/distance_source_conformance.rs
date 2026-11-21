//! Shared conformance and equivalence suite for every `DistanceSource` backend.
//!
//! The matrix and on-the-fly backends must answer the same queries identically. The matrix
//! path is the reference; the on-the-fly path computes the same TSPLIB primitives per lookup.
//! `from_cities_unrounded` is deliberately excluded: it is a test-only fifth path that neither
//! backend represents.

use std::path::Path;
use std::sync::Arc;

use teeline::tsp::{
    AppOptions, DistanceSource, DistanceType, LKOptions, Position, Solvers, TspProblem,
    distance_matrix, kdtree::KDPoint, lin_kernighan, on_the_fly::OnTheFly, solve_problem, tsplib,
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
                    matrix
                        .distance_by_pos(Position::new(a), Position::new(b))
                        .unwrap(),
                    onfly
                        .distance_by_pos(Position::new(a), Position::new(b))
                        .unwrap(),
                    "{fixture} ({dt:?}) position pair ({a}, {b})"
                );
            }
        }

        // Nearest-neighbour results must agree too, not just raw distances.
        let target = matrix.point_at(Position::new(0)).unwrap();
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
            assert!(source.is_coordinate_metric(), "{fixture}/{name}");
            assert_eq!(source.tour_length(&[]), 0.0);

            let first_id = source
                .pos2city_id(Position::new(0))
                .expect("position 0 exists");
            let second_id = source
                .pos2city_id(Position::new(1))
                .expect("position 1 exists");
            assert_eq!(source.tour_length(&[first_id]), 0.0);
            assert_eq!(source.city_id2pos(first_id), Some(Position::new(0)));
            assert_eq!(source.point_at(Position::new(0)).unwrap().id, first_id);
            assert_eq!(source.distance_between(first_id, first_id).unwrap(), 0.0);
            assert_eq!(
                source
                    .distance_by_pos(Position::new(0), Position::new(0))
                    .unwrap(),
                0.0
            );

            let row = source.distances_from(first_id);
            assert_eq!(row.len(), n);
            for (pos, &distance) in row.iter().enumerate() {
                assert_eq!(
                    distance,
                    source
                        .distance_by_pos(Position::new(0), Position::new(pos))
                        .unwrap(),
                    "{fixture}/{name}: distances_from disagrees at position {pos}"
                );
            }

            assert!(
                source
                    .distance_by_pos(Position::new(n), Position::new(0))
                    .is_err(),
                "{fixture}/{name}: first position == n must be out of range"
            );
            assert!(
                source
                    .distance_by_pos(Position::new(0), Position::new(n))
                    .is_err(),
                "{fixture}/{name}: second position == n must be out of range"
            );
            assert!(
                source.distance_between(usize::MAX, first_id).is_err(),
                "{fixture}/{name}: unknown first city id must be an error"
            );
            assert!(
                source.distance_between(first_id, usize::MAX).is_err(),
                "{fixture}/{name}: unknown second city id must be an error"
            );
            assert_eq!(
                source.tour_length(&[usize::MAX, first_id]),
                0.0,
                "{fixture}/{name}: unknown id at the start must not make a tour"
            );
            assert_eq!(
                source.tour_length(&[first_id, usize::MAX, second_id]),
                0.0,
                "{fixture}/{name}: unknown id in the middle must not make a tour"
            );
            assert_eq!(
                source.tour_length(&[first_id, second_id, usize::MAX]),
                0.0,
                "{fixture}/{name}: unknown id at the end must not make a tour"
            );

            let target = source.point_at(Position::new(0)).unwrap();
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
        // Candidate lists rank neighbours by raw Euclidean coordinates, so cover the
        // non-Euclidean metrics and an instance smaller than the candidate floor (14 < 16).
        ("burma14.tsp", DistanceType::Geo, Solvers::TwoOpt),
        ("burma14.tsp", DistanceType::Geo, Solvers::OrOpt),
        ("att48.tsp", DistanceType::Att, Solvers::TwoOpt),
        ("att48.tsp", DistanceType::Att, Solvers::OrOpt),
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

#[test]
fn non_contiguous_city_ids_map_correctly() {
    let cities = vec![
        KDPoint::new_with_id(10, &[0.0, 0.0]),
        KDPoint::new_with_id(20, &[3.0, 4.0]),
        KDPoint::new_with_id(30, &[0.0, 1.0]),
    ];
    let matrix = distance_matrix::build(&cities, DistanceType::Euc2D);
    let onfly = OnTheFly::new(&cities, DistanceType::Euc2D).expect("coordinate source builds");
    let route = vec![10, 20, 30];

    for (name, source) in [
        ("matrix", &matrix as &dyn DistanceSource),
        ("on-the-fly", &onfly as &dyn DistanceSource),
    ] {
        assert_eq!(source.pos2city_id(Position::new(0)), Some(10), "{name}");
        assert_eq!(source.pos2city_id(Position::new(1)), Some(20), "{name}");
        assert_eq!(source.pos2city_id(Position::new(2)), Some(30), "{name}");
        assert_eq!(source.city_id2pos(20), Some(Position::new(1)), "{name}");
        assert_eq!(source.distance_between(10, 20).unwrap(), 5.0, "{name}");
        // EUC_2D rounds each edge: sqrt(3^2 + 3^2) = 4.24.. -> 4.
        assert_eq!(source.distance_between(20, 30).unwrap(), 4.0, "{name}");
        assert_eq!(source.tour_length(&route), 10.0, "{name}");
    }

    assert_eq!(
        matrix.tour_length(&route),
        onfly.tour_length(&route),
        "tour length must use the original city IDs, not positions"
    );

    // A solver must return original city IDs, never dense positions.
    let problem = TspProblem::new(cities, matrix);
    let solution = solve_problem(Solvers::NearestNeighbor, &problem, &AppOptions::default())
        .expect("nearest neighbor solves");
    let mut ids = solution.route().to_vec();
    ids.sort_unstable();
    assert_eq!(ids, vec![10, 20, 30], "route must keep the original IDs");
    assert_eq!(solution.total, 10.0);
}

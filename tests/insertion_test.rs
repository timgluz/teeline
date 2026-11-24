use std::path::Path;

use teeline::config::load_pipeline_config;
use teeline::tsp::{
    AppOptions, DistanceBackend, InsertionOptions, InsertionStrategy, Solvers, TspProblem,
    distance_matrix::DistanceMatrix, kdtree::KDPoint, pipeline, solve_problem as solve, tsplib,
    validate_tour,
};

const STRATEGIES: [InsertionStrategy; 3] = [
    InsertionStrategy::Nearest,
    InsertionStrategy::Farthest,
    InsertionStrategy::Cheapest,
];

fn options(strategy: InsertionStrategy) -> AppOptions {
    AppOptions {
        insertion: Some(InsertionOptions { strategy }),
        ..Default::default()
    }
}

fn matrix_problem(n: usize) -> TspProblem {
    let weights = [
        [0., 2., 9., 10., 7.],
        [2., 0., 6., 4., 3.],
        [9., 6., 0., 8., 5.],
        [10., 4., 8., 0., 1.],
        [7., 3., 5., 1., 0.],
    ];
    let cities: Vec<_> = (0..n)
        .map(|i| KDPoint::new_with_id(i + 1, &[0., 0.]))
        .collect();
    let mut packed = Vec::new();
    for (i, row) in weights.iter().enumerate().take(n) {
        packed.extend_from_slice(&row[..i]);
    }
    let distances = DistanceMatrix::new(n, packed, cities.clone()).unwrap();
    TspProblem::new(cities, distances)
}

#[test]
fn insertion_small_sizes_and_known_routes() {
    for n in 0..=5 {
        let problem = matrix_problem(n);
        for strategy in STRATEGIES {
            let first = solve(Solvers::Insertion, &problem, &options(strategy)).unwrap();
            let second = solve(Solvers::Insertion, &problem, &options(strategy)).unwrap();
            validate_tour(first.route(), &problem.cities).unwrap();
            assert!(first.total.is_finite());
            assert_eq!(first.route(), second.route());
        }
    }
    let problem = matrix_problem(5);
    for (strategy, expected) in [
        (InsertionStrategy::Nearest, &[4, 2, 1, 3, 5][..]),
        (InsertionStrategy::Farthest, &[1, 3, 5, 4, 2][..]),
        (InsertionStrategy::Cheapest, &[4, 2, 1, 3, 5][..]),
    ] {
        assert_eq!(
            solve(Solvers::Insertion, &problem, &options(strategy))
                .unwrap()
                .route(),
            expected
        );
    }
}

#[test]
fn insertion_ties_use_city_ids_not_input_order() {
    let cities: Vec<_> = [40, 10, 30, 20]
        .into_iter()
        .map(|id| KDPoint::new_with_id(id, &[0., 0.]))
        .collect();
    let distances = DistanceMatrix::new(4, vec![1.; 6], cities.clone()).unwrap();
    let problem = TspProblem::new(cities, distances);
    for strategy in STRATEGIES {
        let solution = solve(Solvers::Insertion, &problem, &options(strategy)).unwrap();
        assert_eq!(solution.route(), &[10, 40, 30, 20]);
    }
}

#[test]
fn insertion_fixtures_and_pipeline() {
    for fixture in ["berlin52.tsp", "gr17.tsp", "a280.tsp"] {
        let problem = tsplib::read_from_file(&Path::new("tests/fixtures").join(fixture))
            .unwrap()
            .to_problem(DistanceBackend::Auto)
            .unwrap();
        for strategy in STRATEGIES {
            let opts = options(strategy);
            let first = solve(Solvers::Insertion, &problem, &opts).unwrap();
            let second = solve(Solvers::Insertion, &problem, &opts).unwrap();
            validate_tour(first.route(), &problem.cities).unwrap();
            assert!(first.total.is_finite());
            assert_eq!(first.route(), second.route());
            let stages = [
                pipeline::PipelineStage::new(Solvers::Insertion, opts, problem.clone(), None),
                pipeline::PipelineStage::new(
                    Solvers::TwoOpt,
                    AppOptions::default(),
                    problem.clone(),
                    None,
                ),
            ];
            let polished = pipeline::run_pipeline(&stages).unwrap();
            assert!(polished.total <= first.total);
        }
    }
}

#[test]
fn insertion_toml_and_registration() {
    assert_eq!("ins".parse::<Solvers>().unwrap(), Solvers::Insertion);
    assert!(!Solvers::Insertion.auto_expand_with_nn());
    assert!(!Solvers::Insertion.auto_expand_with_shuffle());
    for (name, strategy) in [
        ("nearest", InsertionStrategy::Nearest),
        ("farthest", InsertionStrategy::Farthest),
        ("cheapest", InsertionStrategy::Cheapest),
    ] {
        let text = format!("[[stage]]\nsolver = 'ins'\n[stage.insertion]\nstrategy = '{name}'");
        let parsed = load_pipeline_config(&text, AppOptions::default()).unwrap();
        assert_eq!(parsed[0].1.insertion.as_ref().unwrap().strategy, strategy);
    }
    for value in ["'invalid'", "123"] {
        let text = format!("[[stage]]\nsolver = 'ins'\n[stage.insertion]\nstrategy = {value}");
        assert!(load_pipeline_config(&text, AppOptions::default()).is_err());
    }
    assert!(
        load_pipeline_config(
            "[[stage]]\nsolver = 'nn'\n[stage.insertion]\nstrategy = 'nearest'",
            AppOptions::default()
        )
        .is_err()
    );
}

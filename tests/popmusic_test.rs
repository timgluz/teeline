/// Integration tests for the POPMUSIC decomposition solver.
use std::path::Path;
use std::str::FromStr;
use teeline::tsp::{
    AppOptions, Solvers, TspProblem, pipeline, solve_problem, solve_with_context, tsplib,
};

fn load_problem(relative: &str) -> TspProblem {
    let path = Path::new(relative);
    let data =
        tsplib::read_from_file(path).unwrap_or_else(|e| panic!("failed to read {relative}: {e}"));
    let cities = data.cities().to_vec();
    let distances = data
        .distance_matrix()
        .unwrap_or_else(|e| panic!("failed to build the distance matrix for {relative}: {e}"));
    TspProblem::new(cities, distances)
}

fn is_valid_tour(route: &[usize], problem: &TspProblem) -> bool {
    let mut expected: Vec<usize> = problem.cities.iter().map(|c| c.id).collect();
    expected.sort_unstable();
    let mut got = route.to_vec();
    got.sort_unstable();
    got == expected
}

#[test]
fn aliases_resolve_to_popmusic() {
    for alias in ["popmusic", "pop-music"] {
        assert_eq!(
            Solvers::from_str(alias).unwrap(),
            Solvers::Popmusic,
            "alias {alias:?} must resolve to Popmusic"
        );
    }
}

#[test]
fn berlin52_returns_a_valid_tour_with_matching_cost() {
    let problem = load_problem("tests/fixtures/berlin52.tsp");
    let solution = solve_problem(Solvers::Popmusic, &problem, &AppOptions::default()).unwrap();

    assert!(is_valid_tour(solution.route(), &problem));
    assert!(solution.total.is_finite());
    assert_eq!(
        solution.total,
        problem.distances.tour_length(solution.route()),
        "reported cost must equal the recomputed tour length"
    );
}

#[test]
fn gr17_explicit_returns_a_valid_tour() {
    let problem = load_problem("tests/fixtures/gr17.tsp");
    let solution = solve_problem(Solvers::Popmusic, &problem, &AppOptions::default()).unwrap();

    assert!(is_valid_tour(solution.route(), &problem));
    assert!(solution.total.is_finite());
    assert_eq!(
        solution.total,
        problem.distances.tour_length(solution.route()),
        "EXPLICIT cost must come from the matrix, not the derived layout"
    );
}

#[test]
fn a280_returns_a_valid_tour_with_matching_cost() {
    let problem = load_problem("tests/fixtures/a280.tsp");
    let solution = solve_problem(Solvers::Popmusic, &problem, &AppOptions::default()).unwrap();

    assert!(is_valid_tour(solution.route(), &problem));
    assert!(solution.total.is_finite());
    assert_eq!(
        solution.total,
        problem.distances.tour_length(solution.route())
    );
}

#[test]
fn pipeline_nn_then_popmusic_is_no_worse_than_nn() {
    for fixture in ["tests/fixtures/berlin52.tsp", "tests/fixtures/a280.tsp"] {
        let problem = load_problem(fixture);
        let nn = pipeline::PipelineStage::new(
            Solvers::NearestNeighbor,
            AppOptions::default(),
            problem.clone(),
            None,
        )
        .solve(None)
        .unwrap();

        let stages = [
            pipeline::PipelineStage::new(
                Solvers::NearestNeighbor,
                AppOptions::default(),
                problem.clone(),
                None,
            ),
            pipeline::PipelineStage::new(Solvers::Popmusic, AppOptions::default(), problem, None),
        ];
        let result = pipeline::run_pipeline(&stages).unwrap();

        assert!(
            result.total <= nn.total * 1.001,
            "{fixture}: POPMUSIC must keep the seed (nn={} got={})",
            nn.total,
            result.total
        );
    }
}

#[test]
fn warm_start_tour_is_consumed() {
    let problem = load_problem("tests/fixtures/a280.tsp");
    let seed: Vec<usize> = problem.cities.iter().rev().map(|c| c.id).collect();
    let seed_cost = problem.distances.tour_length(&seed);
    let solution = solve_with_context(
        Solvers::Popmusic,
        &problem,
        &AppOptions::default(),
        None,
        Some(&seed),
    )
    .unwrap();

    assert!(is_valid_tour(solution.route(), &problem));
    assert!(
        solution.total <= seed_cost + 1e-3,
        "POPMUSIC must not worsen the warm start (seed={seed_cost} got={})",
        solution.total
    );
}

#[test]
fn deterministic_across_runs() {
    let problem = load_problem("tests/fixtures/berlin52.tsp");
    let first = solve_problem(Solvers::Popmusic, &problem, &AppOptions::default()).unwrap();
    let second = solve_problem(Solvers::Popmusic, &problem, &AppOptions::default()).unwrap();

    assert_eq!(first.route(), second.route());
    assert_eq!(first.total, second.total);
}

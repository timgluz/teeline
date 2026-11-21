/// Integration tests for the memetic EAX solver.
use std::path::Path;
use std::str::FromStr;
use teeline::tsp::{
    AppOptions, EaxStrategy, MemeticEaxOptions, Solvers, TspProblem, pipeline, solve_problem,
    tsplib,
};

const BERLIN52_OPT: f32 = 7542.0;
const KROA100_OPT: f32 = 21282.0;

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

fn small_options() -> MemeticEaxOptions {
    MemeticEaxOptions {
        epochs: 10,
        pop_size: 20,
        children_per_pair: 5,
        strategy: EaxStrategy::Hybrid,
        stall_epochs: 5,
        candidate_k: 10,
        seed: Some(1),
    }
}

fn options() -> AppOptions {
    AppOptions {
        memetic_eax: Some(small_options()),
        ..AppOptions::default()
    }
}

#[test]
fn aliases_resolve_to_memetic_eax() {
    for alias in ["eax", "memetic", "memetic_eax"] {
        assert_eq!(
            Solvers::from_str(alias).unwrap(),
            Solvers::MemeticEax,
            "alias {alias:?} must resolve to MemeticEax"
        );
    }
}

#[test]
fn solve_problem_returns_a_valid_tour_with_matching_cost() {
    let problem = load_problem("tests/fixtures/berlin52.tsp");
    let solution = solve_problem(Solvers::MemeticEax, &problem, &options()).unwrap();

    assert!(
        is_valid_tour(solution.route(), &problem),
        "tour must contain every city exactly once"
    );
    assert!(
        (solution.total - problem.distances.tour_length(solution.route())).abs() < 1e-3,
        "solution.total must equal the recomputed tour length"
    );
}

#[test]
fn pipeline_nn_then_eax_is_no_worse_than_nn() {
    let problem = load_problem("tests/fixtures/berlin52.tsp");
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
        pipeline::PipelineStage::new(Solvers::MemeticEax, options(), problem, None),
    ];
    let piped = pipeline::run_pipeline(&stages).unwrap();

    assert!(
        piped.total <= nn.total + 1e-3,
        "eax ({:.1}) must keep the nn warm start ({:.1})",
        piped.total,
        nn.total
    );
}

#[test]
fn berlin52_is_within_a_sane_quality_bound() {
    let problem = load_problem("tests/fixtures/berlin52.tsp");
    let solution = solve_problem(Solvers::MemeticEax, &problem, &options()).unwrap();
    assert!(
        solution.total < 1.10 * BERLIN52_OPT,
        "eax berlin52 {:.1} must be within 10% of {BERLIN52_OPT}",
        solution.total
    );
}

#[test]
fn kroa100_is_within_a_sane_quality_bound() {
    let problem = load_problem("data/tsplib/kroA100.tsp");
    let solution = solve_problem(Solvers::MemeticEax, &problem, &options()).unwrap();
    assert!(
        solution.total < 1.10 * KROA100_OPT,
        "eax kroA100 {:.1} must be within 10% of {KROA100_OPT}",
        solution.total
    );
}

#[test]
fn atsp_input_is_rejected_before_any_solver_runs() {
    let input = "NAME: atsp\nTYPE: ATSP\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: FULL_MATRIX\nEDGE_WEIGHT_SECTION\n0 1 2\n3 0 4\n5 6 0\nEOF\n";
    let err = tsplib::read_from_str(input).unwrap_err();
    assert!(err.contains("ATSP"), "expected ATSP rejection, got: {err}");
}

/// Integration tests for the Space-Filling Curve constructive solver.
use std::path::Path;
use teeline::tsp::{
    AppOptions, SfcOptions, Solvers, TspProblem, kdtree, pipeline, space_filling_curve, tsplib,
};

/// Published TSPLIB optima for the instances used here.
const BERLIN52_OPT: f32 = 7542.0;
const ATT532_OPT: f32 = 27686.0;

fn load_tsp(fixture: &str) -> tsplib::TspLibData {
    let path = Path::new("tests/fixtures").join(fixture);
    tsplib::read_from_file(&path).unwrap_or_else(|e| panic!("failed to read {fixture}: {e}"))
}

/// Uses the instance's own `EDGE_WEIGHT_TYPE` (ATT for att532), not EUC_2D.
fn load_problem(fixture: &str) -> TspProblem {
    let data = load_tsp(fixture);
    let cities = data.cities().to_vec();
    let distances = data
        .distance_matrix()
        .unwrap_or_else(|e| panic!("failed to build distance matrix for {fixture}: {e}"));
    TspProblem::new(cities, distances)
}

fn is_valid_tour(route: &[usize], cities: &[kdtree::KDPoint]) -> bool {
    let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
    expected.sort_unstable();
    let mut got = route.to_vec();
    got.sort_unstable();
    got == expected
}

#[test]
fn sfc_berlin52_valid_and_within_quality_bound() {
    let problem = load_problem("berlin52.tsp");
    let sol = space_filling_curve::solve(&problem, &SfcOptions::default(), None, None);
    assert!(
        is_valid_tour(sol.route(), &problem.cities),
        "tour must contain every city exactly once"
    );
    assert!(
        sol.total < 1.5 * BERLIN52_OPT,
        "sfc berlin52 {:.1} must be below 1.5 × {BERLIN52_OPT}",
        sol.total
    );
}

#[test]
fn sfc_att532_multi_orientation_within_quality_bound() {
    let problem = load_problem("att532.tsp");
    let opts = SfcOptions {
        multi_orientation: 8,
        ..SfcOptions::default()
    };
    let sol = space_filling_curve::solve(&problem, &opts, None, None);
    assert!(
        is_valid_tour(sol.route(), &problem.cities),
        "tour must contain every city exactly once"
    );
    assert!(
        sol.total < 1.5 * ATT532_OPT,
        "sfc att532 (multi-orientation) {:.1} must be below 1.5 × {ATT532_OPT}",
        sol.total
    );
}

#[test]
fn sfc_pipelines_are_no_worse_than_sfc_alone() {
    let solo_problem = load_problem("berlin52.tsp");
    let solo = space_filling_curve::solve(&solo_problem, &SfcOptions::default(), None, None);

    for (solver, label) in [(Solvers::TwoOpt, "2opt"), (Solvers::LinKernighan, "lk")] {
        let problem = load_problem("berlin52.tsp");
        let stages = [
            pipeline::PipelineStage::new(
                Solvers::SpaceFillingCurve,
                AppOptions::default(),
                problem.clone(),
                None,
            ),
            pipeline::PipelineStage::new(solver, AppOptions::default(), problem, None),
        ];
        let piped = pipeline::run_pipeline(&stages).unwrap();
        assert!(
            piped.total <= solo.total + 1e-3,
            "sfc -> {label} ({:.1}) must be no worse than sfc alone ({:.1})",
            piped.total,
            solo.total
        );
    }
}

use std::path::{Path, PathBuf};

use teeline::config::{IdentityProvider, resolve_config_file, select_pipeline_source};
use teeline::tsp::{
    Solvers, TspProblem,
    pipeline::{PipelineStage, run_pipeline},
    tsplib,
};

const BERLIN52: &str = "tests/fixtures/berlin52.tsp";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(format!("tests/fixtures/{name}"))
}

fn berlin52_stages(stage_configs: Vec<(Solvers, teeline::tsp::AppOptions)>) -> Vec<PipelineStage> {
    let tsp = tsplib::read_from_file(Path::new(BERLIN52)).unwrap();
    let problem = TspProblem::new(tsp.cities().to_vec(), tsp.distance_matrix().unwrap());
    stage_configs
        .into_iter()
        .map(|(solver, options)| PipelineStage::new(solver, options, problem.clone(), None))
        .collect()
}

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn test_pipeline_config_runs_and_produces_valid_output() {
    let p = fixture("pipeline_nn_2opt.toml");
    let stage_configs = resolve_config_file(&p, &IdentityProvider).unwrap();
    let stages = berlin52_stages(stage_configs);
    let solution = run_pipeline(&stages).unwrap();
    assert!(solution.total > 0.0);
    assert_eq!(solution.route().len(), 52);
}

#[test]
fn test_pipeline_config_sa_stage_epochs_applied() {
    let p = fixture("pipeline_global_nn_sa.toml");
    let stage_configs = resolve_config_file(&p, &IdentityProvider).unwrap();
    // SA stage should have epochs=50 in its [stage.sa] options
    let sa_stage = stage_configs
        .iter()
        .find(|(s, _)| *s == Solvers::SimulatedAnnealing);
    assert!(sa_stage.is_some(), "expected SA stage in fixture");
    let (_, opts) = sa_stage.unwrap();
    assert_eq!(opts.sa.as_ref().unwrap().heuristic.epochs, 50);
    // Also verify it runs to completion
    let stages = berlin52_stages(stage_configs);
    let solution = run_pipeline(&stages).unwrap();
    assert_eq!(solution.route().len(), 52);
}

// ---------------------------------------------------------------------------
// Mutual-exclusion errors
// ---------------------------------------------------------------------------

#[test]
fn test_pipeline_config_and_steps_mutually_exclusive() {
    let err = select_pipeline_source(
        Some(Path::new("any.toml")),
        Some(&["nn".to_string(), "2opt".to_string()]),
    )
    .unwrap_err();
    assert!(err.contains("mutually exclusive"), "got: {err}");
}

#[test]
fn test_pipeline_neither_config_nor_steps_errors() {
    let err = select_pipeline_source(None, None).unwrap_err();
    assert!(
        err.contains("--config") || err.contains("--steps"),
        "got: {err}"
    );
}

// ---------------------------------------------------------------------------
// Missing / unreadable config file
// ---------------------------------------------------------------------------

#[test]
fn test_pipeline_missing_config_file_errors() {
    let err = resolve_config_file(
        Path::new("/tmp/teeline_does_not_exist_xyz.toml"),
        &IdentityProvider,
    )
    .unwrap_err();
    assert!(err.contains("teeline_does_not_exist_xyz"), "got: {err}");
}

// ---------------------------------------------------------------------------
// Config validation errors
// ---------------------------------------------------------------------------

#[test]
fn test_pipeline_unknown_key_in_stage_errors() {
    let p = fixture("pipeline_unknown_key.toml");
    let err = resolve_config_file(&p, &IdentityProvider).unwrap_err();
    assert!(err.contains("epoch"), "got: {err}");
}

#[test]
fn test_pipeline_cooling_rate_zero_errors() {
    let p = fixture("pipeline_bad_cooling_rate.toml");
    let err = resolve_config_file(&p, &IdentityProvider).unwrap_err();
    assert!(err.contains("cooling_rate"), "got: {err}");
}

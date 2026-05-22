use std::sync::mpsc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::{validate_tour, Solution, SolverOptions, Solvers};

#[derive(Clone, Debug)]
pub struct PipelineStage {
    pub solver: Solvers,
    pub options: SolverOptions, // fully resolved at config-load time
}

/// Run a pipeline of stages. `progress_tx` and `initial_tour` are injected at
/// runtime per stage and cannot be set via config.
pub fn solve(
    stages: &[PipelineStage],
    cities: &[KDPoint],
    distances: &DistanceMatrix,
    progress_tx: Option<mpsc::Sender<ProgressMessage>>,
) -> Result<Solution, String> {
    assert!(!stages.is_empty(), "pipeline: stages must not be empty — validate at call site");

    let mut seed: Option<Vec<usize>> = None;
    let mut last_solution: Option<Solution> = None;

    for (i, stage) in stages.iter().enumerate() {
        if let Some(ref t) = seed
            && let Err(e) = validate_tour(t, cities)
        {
            tracing::warn!("pipeline stage {i}: invalid seed ({e}); using default seeding");
            seed = None;
        }

        let mut stage_opts = stage.options.clone();
        stage_opts.progress_tx = progress_tx.clone();
        stage_opts.initial_tour = seed.take();

        tracing::info!(stage = i, solver = ?stage.solver, "pipeline: stage starting");

        let solution = super::solve(stage.solver, cities, distances, &stage_opts)
            .map_err(|e| format!("pipeline stage {i} ({:?}): {e}", stage.solver))?;

        tracing::info!(stage = i, cost = solution.total, "pipeline: stage complete");

        seed = Some(solution.route().to_vec());
        last_solution = Some(solution);
    }

    Ok(last_solution.expect("loop executed — asserted non-empty above"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, SolverOptions, Solvers};

    fn small_cities() -> Vec<KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
            vec![0.5, 0.5],
        ])
    }

    fn stage(solver: Solvers) -> PipelineStage {
        PipelineStage { solver, options: SolverOptions::default() }
    }

    fn stage_with(solver: Solvers, options: SolverOptions) -> PipelineStage {
        PipelineStage { solver, options }
    }

    #[test]
    fn test_pipeline_single_step_nn_produces_valid_tour() {
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let stages = [stage(Solvers::NearestNeighbor)];

        let result = solve(&stages, &cities, &dm, None).unwrap();

        assert_eq!(result.len(), cities.len());
        let mut seen = result.route().to_vec();
        seen.sort();
        let expected: Vec<usize> = {
            let mut ids: Vec<usize> = cities.iter().map(|c| c.id).collect();
            ids.sort();
            ids
        };
        assert_eq!(seen, expected);
    }

    #[test]
    fn test_pipeline_two_steps_nn_then_2opt_no_worse_than_nn() {
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let opts = SolverOptions { epochs: 100, ..SolverOptions::default() };

        let nn_result = super::super::solve(
            Solvers::NearestNeighbor, &cities, &dm, &opts,
        ).unwrap();

        let stages = [stage(Solvers::NearestNeighbor), stage_with(Solvers::TwoOpt, opts)];
        let pipeline_result = solve(&stages, &cities, &dm, None).unwrap();

        // 2-opt cannot worsen a tour
        assert!(pipeline_result.total <= nn_result.total * 1.001);
    }

    #[test]
    fn test_pipeline_stage_opts_prevent_recursion() {
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let stages = [stage(Solvers::TwoOpt)];

        let result = solve(&stages, &cities, &dm, None);
        assert!(result.is_ok());
    }
}

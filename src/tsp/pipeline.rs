use std::sync::mpsc;
use std::time::Instant;

use super::progress::ProgressMessage;
use super::{AppOptions, Solution, Solvers, TspProblem, validate_tour};

/// One stage's result within a pipeline run: the solved tour plus the wall-clock
/// time that stage took. Lives alongside `PipelineStage` (the stage's inputs) —
/// no `solver` field here, since callers already know which stage produced which
/// outcome by index/zip with the input stage list.
pub struct StageOutcome {
    pub solution: Solution,
    pub duration_ms: u64,
}

pub struct PipelineStage {
    pub solver: Solvers,
    pub options: AppOptions,
    pub problem: TspProblem,
    pub progress_tx: Option<mpsc::Sender<ProgressMessage>>,
}

impl PipelineStage {
    pub fn new(
        solver: Solvers,
        options: AppOptions,
        problem: TspProblem,
        progress_tx: Option<mpsc::Sender<ProgressMessage>>,
    ) -> Self {
        PipelineStage {
            solver,
            options,
            problem,
            progress_tx,
        }
    }

    pub fn solve(&self, init_tour: Option<&[usize]>) -> Result<Solution, String> {
        super::solve_with_context(
            self.solver,
            &self.problem,
            &self.options,
            self.progress_tx.clone(),
            init_tour,
        )
    }
}

/// Runs `stages` in order, warm-starting each stage from the previous stage's
/// tour, and returns every stage's outcome (solved tour + wall-clock duration).
/// Shared by `run_pipeline` (below) and any caller that needs per-stage detail
/// (e.g. the API's pipeline endpoint).
pub fn run_pipeline_stages(stages: &[PipelineStage]) -> Result<Vec<StageOutcome>, String> {
    if stages.is_empty() {
        return Err("pipeline has no stages".into());
    }
    let mut seed: Option<Vec<usize>> = None;
    let mut outcomes = Vec::with_capacity(stages.len());
    for stage in stages {
        if let Some(ref t) = seed
            && let Err(e) = validate_tour(t, &stage.problem.cities)
        {
            tracing::warn!("pipeline: invalid seed ({e}); using default seeding");
            seed = None;
        }
        tracing::info!(solver = ?stage.solver, "pipeline: stage starting");
        let start = Instant::now();
        let solution = stage.solve(seed.as_deref())?;
        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        validate_tour(solution.route(), &stage.problem.cities)
            .map_err(|e| format!("stage {:?} invalid tour: {e}", stage.solver))?;
        tracing::info!(cost = solution.total, "pipeline: stage complete");
        seed = Some(solution.route().to_vec());
        outcomes.push(StageOutcome {
            solution,
            duration_ms,
        });
    }
    Ok(outcomes)
}

pub fn run_pipeline(stages: &[PipelineStage]) -> Result<Solution, String> {
    Ok(run_pipeline_stages(stages)?
        .pop()
        .expect("run_pipeline_stages errors on empty input")
        .solution)
}

/// Non-fatal composition warnings for a pipeline's solver sequence (issue #66's
/// original design table). Purely a static lint over `solvers` — derivable from
/// the request alone, never depends on how a stage actually solves.
pub fn stage_warnings(solvers: &[Solvers]) -> Vec<String> {
    let mut warnings = Vec::new();
    let last = solvers.len().saturating_sub(1);
    for (i, solver) in solvers.iter().enumerate() {
        if i > 0 {
            match solver {
                Solvers::NearestNeighbor => warnings.push(format!(
                    "nn at stage {i} discards the warm-start seed from the previous stage"
                )),
                Solvers::GreedyEdge => warnings.push(format!(
                    "greedy_edge at stage {i} discards the warm-start seed from the previous \
                     stage (it always rebuilds from scratch)"
                )),
                _ => {}
            }
        }
        if i != last {
            match solver {
                Solvers::BellmanKarp => warnings.push(format!(
                    "BellmanKarp at stage {i} ignores the warm-start seed entirely (the exact \
                     DP has no use for a partial/seed tour) and its optimal result will be \
                     superseded by later stages"
                )),
                // Unlike BellmanKarp, BranchBound *does* consume the warm-start seed — it uses
                // it to prime the pruning upper bound (see branch_bound::solve) — so the warning
                // wording must not claim it ignores the seed the way BellmanKarp does.
                Solvers::BranchBound => warnings.push(format!(
                    "BranchBound at stage {i} only uses the warm-start seed to prime its \
                     pruning bound, not as a tour to refine, and its optimal result will be \
                     superseded by later stages"
                )),
                _ => {}
            }
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{AppOptions, Solvers, TspProblem, distance_matrix, kdtree};

    fn small_cities() -> Vec<kdtree::KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
            vec![0.5, 0.5],
        ])
    }

    fn make_stage(solver: Solvers, problem: TspProblem) -> PipelineStage {
        PipelineStage::new(solver, AppOptions::default(), problem, None)
    }

    #[test]
    fn test_pipeline_single_step_nn_produces_valid_tour() {
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities.clone(), dm);
        let stages = [make_stage(Solvers::NearestNeighbor, problem)];

        let result = run_pipeline(&stages).unwrap();

        assert_eq!(result.len(), cities.len());
        let mut seen = result.route().to_vec();
        seen.sort();
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        assert_eq!(seen, expected);
    }

    #[test]
    fn test_pipeline_two_steps_nn_then_2opt_no_worse_than_nn() {
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities.clone(), dm);

        let nn_result = make_stage(Solvers::NearestNeighbor, problem.clone())
            .solve(None)
            .unwrap();

        let stages = [
            make_stage(Solvers::NearestNeighbor, problem.clone()),
            make_stage(Solvers::TwoOpt, problem.clone()),
        ];
        let pipeline_result = run_pipeline(&stages).unwrap();

        assert!(pipeline_result.total <= nn_result.total * 1.001);
    }

    #[test]
    fn test_pipeline_empty_stages_errors() {
        assert!(run_pipeline(&[]).is_err());
    }

    #[test]
    fn test_pipeline_stage_twoopt_produces_valid_tour() {
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);
        let stages = [make_stage(Solvers::TwoOpt, problem)];

        let result = run_pipeline(&stages);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_pipeline_stages_matches_run_pipeline_final_cost() {
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);

        let stages = [
            make_stage(Solvers::NearestNeighbor, problem.clone()),
            make_stage(Solvers::TwoOpt, problem.clone()),
        ];
        let outcomes = run_pipeline_stages(&stages).unwrap();
        assert_eq!(outcomes.len(), stages.len());

        let stages_again = [
            make_stage(Solvers::NearestNeighbor, problem.clone()),
            make_stage(Solvers::TwoOpt, problem),
        ];
        let final_solution = run_pipeline(&stages_again).unwrap();

        assert_eq!(
            outcomes.last().unwrap().solution.total,
            final_solution.total
        );
    }

    #[test]
    fn test_run_pipeline_stages_errors_on_stage_failure() {
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);
        let stages = [make_stage(Solvers::Unspecified, problem)];

        assert!(run_pipeline_stages(&stages).is_err());
        assert!(run_pipeline(&stages).is_err());
    }

    #[test]
    fn test_stage_warnings_flags_nn_mid_pipeline() {
        let warnings =
            stage_warnings(&[Solvers::TwoOpt, Solvers::NearestNeighbor, Solvers::TwoOpt]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("nn at stage 1"));
    }

    #[test]
    fn test_stage_warnings_flags_exact_solver_non_terminal() {
        let warnings = stage_warnings(&[Solvers::BranchBound, Solvers::TwoOpt]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("stage 0"));
    }

    #[test]
    fn test_stage_warnings_empty_for_well_formed_pipeline() {
        let warnings = stage_warnings(&[
            Solvers::NearestNeighbor,
            Solvers::TwoOpt,
            Solvers::SimulatedAnnealing,
        ]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_stage_warnings_exact_solver_in_terminal_position_is_fine() {
        let warnings = stage_warnings(&[Solvers::NearestNeighbor, Solvers::BranchBound]);
        assert!(warnings.is_empty());
    }
}

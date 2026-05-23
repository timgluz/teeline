use std::sync::mpsc;

use super::progress::ProgressMessage;
use super::{validate_tour, AppOptions, Solution, Solvers, TspProblem};

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
        PipelineStage { solver, options, problem, progress_tx }
    }

    pub fn solve(&self) -> Result<Solution, String> {
        super::solve_with_context(
            self.solver,
            &self.problem,
            &self.options,
            self.progress_tx.clone(),
        )
    }
}

pub fn run_pipeline(stages: &mut [PipelineStage]) -> Result<Solution, String> {
    if stages.is_empty() {
        return Err("pipeline has no stages".into());
    }
    let mut seed: Option<Vec<usize>> = None;
    for stage in stages.iter_mut() {
        if let Some(ref t) = seed {
            if let Err(e) = validate_tour(t, &stage.problem.cities) {
                tracing::warn!("pipeline: invalid seed ({e}); using default seeding");
                seed = None;
            }
        }
        stage.problem.initial_tour = seed.clone();

        tracing::info!(solver = ?stage.solver, "pipeline: stage starting");

        let solution = stage.solve()?;
        validate_tour(solution.route(), &stage.problem.cities)
            .map_err(|e| format!("stage {:?} invalid tour: {e}", stage.solver))?;

        tracing::info!(cost = solution.total, "pipeline: stage complete");

        seed = Some(solution.route().to_vec());
    }

    let last = stages.last().unwrap();
    let route = seed.unwrap();
    Ok(Solution::new(&route, &last.problem))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, AppOptions, Solvers, TspProblem};

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
        let mut stages = [make_stage(Solvers::NearestNeighbor, problem)];

        let result = run_pipeline(&mut stages).unwrap();

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
            .solve()
            .unwrap();

        let mut stages = [
            make_stage(Solvers::NearestNeighbor, problem.clone()),
            make_stage(Solvers::TwoOpt, problem.clone()),
        ];
        let pipeline_result = run_pipeline(&mut stages).unwrap();

        assert!(pipeline_result.total <= nn_result.total * 1.001);
    }

    #[test]
    fn test_pipeline_empty_stages_errors() {
        assert!(run_pipeline(&mut []).is_err());
    }

    #[test]
    fn test_pipeline_stage_twoopt_produces_valid_tour() {
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);
        let mut stages = [make_stage(Solvers::TwoOpt, problem)];

        let result = run_pipeline(&mut stages);
        assert!(result.is_ok());
    }
}

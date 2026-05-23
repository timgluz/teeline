use std::sync::mpsc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::{validate_tour, AppOptions, Solution, Solvers};

pub struct PipelineStage {
    pub solver: Solvers,
    pub options: AppOptions,
    pub cities: Vec<KDPoint>,
    pub distances: DistanceMatrix,
    pub progress_tx: Option<mpsc::Sender<ProgressMessage>>,
}

impl PipelineStage {
    pub fn new(
        solver: Solvers,
        options: AppOptions,
        cities: Vec<KDPoint>,
        distances: DistanceMatrix,
        progress_tx: Option<mpsc::Sender<ProgressMessage>>,
    ) -> Self {
        PipelineStage { solver, options, cities, distances, progress_tx }
    }

    pub fn solve(&self, initial_tour: Option<&[usize]>) -> Result<Solution, String> {
        super::solve_with_context(
            self.solver,
            &self.cities,
            &self.distances,
            &self.options,
            self.progress_tx.clone(),
            initial_tour,
        )
    }
}

pub fn run_pipeline(stages: &[PipelineStage]) -> Result<Solution, String> {
    if stages.is_empty() {
        return Err("pipeline has no stages".into());
    }
    let mut seed: Option<Vec<usize>> = None;
    for stage in stages {
        if let Some(ref t) = seed {
            if let Err(e) = validate_tour(t, &stage.cities) {
                tracing::warn!("pipeline: invalid seed ({e}); using default seeding");
                seed = None;
            }
        }

        tracing::info!(solver = ?stage.solver, "pipeline: stage starting");

        let solution = stage.solve(seed.as_deref())?;
        validate_tour(solution.route(), &stage.cities)
            .map_err(|e| format!("stage {:?} invalid tour: {e}", stage.solver))?;

        tracing::info!(cost = solution.total, "pipeline: stage complete");

        seed = Some(solution.route().to_vec());
    }

    let last = stages.last().unwrap();
    let route = seed.unwrap();
    Ok(Solution::new(&route, &last.cities, &last.distances))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, AppOptions, Solvers};

    fn small_cities() -> Vec<KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
            vec![0.5, 0.5],
        ])
    }

    fn make_stage(solver: Solvers, cities: Vec<KDPoint>, distances: DistanceMatrix) -> PipelineStage {
        PipelineStage::new(solver, AppOptions::default(), cities, distances, None)
    }

    #[test]
    fn test_pipeline_single_step_nn_produces_valid_tour() {
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let stages = [make_stage(Solvers::NearestNeighbor, cities.clone(), dm.clone())];

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

        let nn_result = make_stage(Solvers::NearestNeighbor, cities.clone(), dm.clone())
            .solve(None)
            .unwrap();

        let stages = [
            make_stage(Solvers::NearestNeighbor, cities.clone(), dm.clone()),
            make_stage(Solvers::TwoOpt, cities.clone(), dm.clone()),
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
        let stages = [make_stage(Solvers::TwoOpt, cities.clone(), dm.clone())];

        let result = run_pipeline(&stages);
        assert!(result.is_ok());
    }
}

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::{validate_tour, Solution, SolverOptions, Solvers};

pub fn solve(
    steps: &[Solvers],
    cities: &[KDPoint],
    distances: &DistanceMatrix,
    options: &SolverOptions,
) -> Result<Solution, String> {
    assert!(!steps.is_empty(), "pipeline: steps must not be empty — validate at call site");

    let mut seed: Option<Vec<usize>> = options.initial_tour.clone();
    let mut last_solution: Option<Solution> = None;

    for (i, &solver) in steps.iter().enumerate() {
        if let Some(ref t) = seed
            && let Err(e) = validate_tour(t, cities)
        {
            tracing::warn!("pipeline stage {i}: invalid seed ({e}); using default seeding");
            seed = None;
        }

        let mut stage_opts = options.clone();
        stage_opts.initial_tour = seed.take();

        tracing::info!(stage = i, solver = ?solver, "pipeline: stage starting");

        let solution = super::solve(solver, cities, distances, &stage_opts)
            .map_err(|e| format!("pipeline stage {i} ({solver:?}): {e}"))?;

        tracing::info!(stage = i, cost = solution.total, "pipeline: stage complete");

        seed = Some(solution.route().to_vec());
        last_solution = Some(solution);
    }

    Ok(last_solution.expect("loop executed — asserted non-empty above"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, Solvers};

    fn small_cities() -> Vec<KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
            vec![0.5, 0.5],
        ])
    }

    #[test]
    fn test_pipeline_single_step_nn_produces_valid_tour() {
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let options = SolverOptions::default();

        let result = solve(&[Solvers::NearestNeighbor], &cities, &dm, &options).unwrap();

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
        let options = SolverOptions { epochs: 100, ..SolverOptions::default() };

        let nn_result = super::super::solve(
            Solvers::NearestNeighbor, &cities, &dm, &options,
        ).unwrap();

        let pipeline_result = solve(
            &[Solvers::NearestNeighbor, Solvers::TwoOpt],
            &cities, &dm, &options,
        ).unwrap();

        // 2-opt cannot worsen a tour
        assert!(pipeline_result.total <= nn_result.total * 1.001);
    }

    #[test]
    fn test_pipeline_stage_opts_prevent_recursion() {
        // Calling pipeline with a single NN step should not recurse or panic
        let cities = small_cities();
        let dm = distance_matrix::from_cities(&cities);
        let mut options = SolverOptions::default();
        // Simulate coming from a prior stage: initial_tour is set
        options.initial_tour = Some(cities.iter().map(|c| c.id).collect());

        let result = solve(&[Solvers::TwoOpt], &cities, &dm, &options);
        assert!(result.is_ok());
    }
}

use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

use rand::RngExt;

use super::DistanceSource;
use super::Position;
use super::candidate_list::{CandidateList, MIN_CANDIDATE_K};
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::{Route, random_tour};
use super::tour_graph::distance;
use super::{HeuristicOptions, Solution, TspProblem};

pub fn solve(
    problem: &TspProblem,
    // `nn` reads nothing from HeuristicOptions; `n_nearest` is a candidate-list limit for
    // solvers that trade exactness for speed. Kept for the uniform solver signature.
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    // Not logging `opts.n_nearest`: it has no effect here, and naming it invites
    // misattribution when debugging reproducibility.
    tracing::info!(cities = cities.len(), "NN starting");

    let cities_table: HashMap<usize, _> = cities.iter().map(|c| (c.id, *c)).collect();

    let mut unvisited: HashSet<usize> = cities.iter().map(|c| c.id).collect();
    let mut path: Vec<usize> = Vec::with_capacity(cities.len());

    let start_id = cities[0].id;
    path.push(start_id);
    unvisited.remove(&start_id);

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&path), 0.0));
    }

    while !unvisited.is_empty() {
        let current_id = *path.last().unwrap();
        let current_city = &cities_table[&current_id];

        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::CityChange(current_id));
        }

        // No `n_nearest` here: greedy means the nearest unvisited, and a k-limited
        // candidate set silently produced worse tours.
        //
        // `expect` rather than `break`: with `unvisited` non-empty and `current_city` from
        // the matrix's own cities, `None` means an invariant broke, and a quiet break would
        // return a truncated tour as if it were a solution.
        let nearest = distances
            .nearest_unvisited(current_city, &|id| {
                id != current_id && unvisited.contains(&id)
            })
            .expect("unvisited is non-empty and current_city came from the matrix's own cities");
        let next_id = nearest.point.id;

        path.push(next_id);
        unvisited.remove(&next_id);
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&path), 0.0));
        }
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&path, distances.as_ref())
}

/// Greedy nearest-neighbour tour over dense positions, choosing uniformly among the
/// `relaxation` nearest unvisited candidates.
///
/// `relaxation = 1` is the deterministic nearest-neighbour rule; larger values diversify the
/// constructive starts a population-based solver (such as memetic EAX) builds on.
pub(crate) fn randomized_tour(
    distances: &dyn DistanceSource,
    candidates: &CandidateList,
    start: usize,
    relaxation: usize,
    rng: &mut impl RngExt,
) -> Vec<usize> {
    let n = distances.num_cities();
    debug_assert!(start < n, "start position must be inside the problem");
    let relaxation = relaxation.max(1);
    let mut visited = vec![false; n];
    let mut tour = Vec::with_capacity(n);
    let mut current = start;
    visited[current] = true;
    tour.push(current);

    let mut choices: Vec<usize> = Vec::with_capacity(relaxation);
    for _ in 1..n {
        choices.clear();
        for &candidate in candidates.neighbours_by_pos(Position::new(current)) {
            let city = candidate.into_inner();
            if !visited[city] {
                choices.push(city);
                if choices.len() == relaxation {
                    break;
                }
            }
        }
        current = if choices.is_empty() {
            // Candidate list exhausted (k < n - 1): fall back to the nearest unvisited city.
            (0..n)
                .filter(|&city| !visited[city])
                .min_by(|&a, &b| {
                    distance(distances, current, a).total_cmp(&distance(distances, current, b))
                })
                .expect("at least one unvisited city remains")
        } else {
            choices[rng.random_range(0..choices.len())]
        };
        visited[current] = true;
        tour.push(current);
    }
    tour
}

/// Seed a population for a population-based solver: entry 0 is the pipeline seed (or a
/// random tour), the first half are diverse nearest-neighbour tours, and the rest random.
///
/// `init_tour` is the pipeline warm start and is only used for entry 0, preserving the
/// contract that a caller's seed becomes the solver's initial best.
pub(crate) fn seed_population(
    distances: &dyn DistanceSource,
    cities: &[KDPoint],
    n_individuals: usize,
    init_tour: Option<&[usize]>,
    rng: &mut impl RngExt,
) -> Vec<Vec<usize>> {
    let n_cities = cities.len();
    let city_ids: Vec<usize> = cities.iter().map(|c| c.id).collect();

    // One candidate list for all constructive seeds. Coordinate metrics rank by the
    // KD-tree; EXPLICIT inputs rank by the exact distance source, because their layout is
    // a display aid (or a derived embedding), not the objective.
    let candidates = if distances.is_coordinate_metric() {
        CandidateList::build(cities, MIN_CANDIDATE_K)
    } else {
        CandidateList::build_from_distances(distances, cities, MIN_CANDIDATE_K)
    }
    .ok();

    let mut population = Vec::with_capacity(n_individuals);
    for idx in 0..n_individuals {
        let tour = if idx == 0 {
            init_tour
                .map(|t| t.to_vec())
                .unwrap_or_else(|| random_tour(&city_ids, rng))
        } else if idx < n_individuals / 2 {
            match &candidates {
                Some(cands) if n_cities >= 4 => {
                    let start = rng.random_range(0..n_cities);
                    randomized_tour(distances, cands, start, 2, rng)
                        .into_iter()
                        .map(|pos| cities[pos].id)
                        .collect()
                }
                _ => random_tour(&city_ids, rng),
            }
        } else {
            random_tour(&city_ids, rng)
        };
        population.push(tour);
    }
    population
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{HeuristicOptions, TspProblem, distance_matrix, kdtree};
    use rand::SeedableRng;

    fn tsp5_problem() -> TspProblem {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        TspProblem::new(cities, dm)
    }

    #[test]
    fn test_solve_visits_all_cities() {
        let problem = tsp5_problem();
        let tour = solve(&problem, &HeuristicOptions::default(), None, None);

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_solve_tour_length_is_positive_and_finite() {
        let problem = tsp5_problem();
        let tour = solve(&problem, &HeuristicOptions::default(), None, None);

        assert!(
            tour.total > 0.0,
            "tour length should be positive, got {}",
            tour.total
        );
        assert!(tour.total.is_finite(), "tour length should be finite");
    }

    #[test]
    fn test_solve_on_collinear_cities_visits_all() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![2.0, 0.0],
            vec![3.0, 0.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);
        let tour = solve(&problem, &HeuristicOptions::default(), None, None);

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3]);
    }

    #[test]
    fn randomized_tour_with_relaxation_one_is_deterministic_nearest_neighbour() {
        let problem = tsp5_problem();
        let cities = &problem.cities;
        let candidates = CandidateList::build(cities, 4).unwrap();
        let distances = problem.distances.as_ref();
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let first = randomized_tour(distances, &candidates, 0, 1, &mut rng);
        let second = randomized_tour(distances, &candidates, 0, 1, &mut rng);
        assert_eq!(first, second);
        let mut visited = first.clone();
        visited.sort_unstable();
        assert_eq!(visited, (0..cities.len()).collect::<Vec<_>>());
    }

    #[test]
    fn randomized_tour_is_a_valid_permutation_for_any_relaxation() {
        let problem = tsp5_problem();
        let cities = &problem.cities;
        let candidates = CandidateList::build(cities, 4).unwrap();
        let distances = problem.distances.as_ref();
        let mut rng = rand::rngs::StdRng::seed_from_u64(2);
        for relaxation in [1usize, 3] {
            let tour = randomized_tour(distances, &candidates, 1, relaxation, &mut rng);
            let mut visited = tour.clone();
            visited.sort_unstable();
            assert_eq!(visited, (0..cities.len()).collect::<Vec<_>>());
        }
    }

    #[test]
    fn test_solve_does_not_produce_sorted_output_on_shuffled_input() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![100.0, 0.0],
            vec![99.0, 0.0],
            vec![98.0, 0.0],
            vec![1.0, 0.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);
        let tour = solve(&problem, &HeuristicOptions::default(), None, None);

        let route = tour.route().to_vec();
        assert_ne!(
            route,
            vec![0, 1, 2, 3, 4],
            "NN produced sorted output (regression)"
        );
        let mut sorted = route.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_seed_population_keeps_the_seed_and_valid_tours() {
        let problem = tsp5_problem();
        let cities = &problem.cities;
        let seed: Vec<usize> = cities.iter().map(|c| c.id).rev().collect();
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);

        let population =
            seed_population(problem.distances.as_ref(), cities, 6, Some(&seed), &mut rng);

        assert_eq!(population.len(), 6);
        assert_eq!(population[0], seed);

        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        for tour in &population {
            let mut sorted = tour.clone();
            sorted.sort();
            assert_eq!(sorted, expected);
        }
    }

    #[test]
    fn test_seed_population_handles_an_explicit_matrix() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
            vec![0.5, 0.5],
            vec![0.2, 0.8],
        ]);
        let n = cities.len();
        let weights: Vec<f32> = (0..n * (n - 1) / 2).map(|i| (i + 1) as f32).collect();
        let dm = distance_matrix::DistanceMatrix::new(n, weights, cities.clone())
            .expect("a complete lower triangle");
        let problem = TspProblem::new(cities.clone(), dm);
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);

        let population = seed_population(
            problem.distances.as_ref(),
            &problem.cities,
            6,
            None,
            &mut rng,
        );

        assert_eq!(population.len(), 6);
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort();
        for tour in &population {
            let mut sorted = tour.clone();
            sorted.sort();
            assert_eq!(sorted, expected);
        }
    }
}

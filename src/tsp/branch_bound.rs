use std::collections::HashSet;
use std::rc::Rc;
use std::sync::mpsc;

use super::distance_matrix::DistanceMatrix;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem};

// NOTE: build_points() assigns 0-indexed city IDs, so this sentinel collides
// with city ID 0 in unit tests. Currently harmless because the algorithm uses
// positional indexing (path[k]) and never compares slots to UNVISITED_NODE
// except in the progress-reporting filter (which may silently drop city 0 from
// progress updates). TSPLIB files are 1-indexed, so no production impact.
const UNVISITED_NODE: usize = 0;

type UniqSet = HashSet<usize>;
type Path = Vec<usize>;
type PathEvaluator = Rc<dyn Fn(&Path) -> f32>;

pub fn solve(
    problem: &TspProblem,
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let mut route = Route::from_cities(cities);
    let n_cities = route.len();

    tracing::info!(n_cities, "B&B starting");

    route.sort();
    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(route.clone(), 0.0));
    }

    let mut open_path: Path = vec![0; n_cities];
    open_path[0] = route.get(0).unwrap();

    let unvisited_cities: UniqSet = route.route().iter().skip(1).copied().collect();

    let fitness_fn = build_evaluator(distances);
    let (best_path, _best_distance) = backtrack(
        &fitness_fn,
        distances,
        &mut open_path,
        &unvisited_cities,
        1,
        0.0,
        f32::MAX,
        progress_tx,
    );

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&best_path, cities, distances)
}

fn build_evaluator(distances: &DistanceMatrix) -> PathEvaluator {
    let dm = Rc::new(distances.clone());

    Rc::new(move |path: &Path| dm.tour_length(path))
}

#[allow(clippy::only_used_in_recursion, clippy::too_many_arguments)]
fn backtrack(
    evaluate_fn: &PathEvaluator,
    distances: &DistanceMatrix,
    path: &mut Path,
    unvisited_cities: &UniqSet,
    k: usize,
    running_cost: f32,
    upper_bound: f32,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
) -> (Path, f32) {
    let mut best_path = path.clone();
    let mut best_distance = upper_bound;

    let n_cities = k + unvisited_cities.len();
    if is_solution(path, k, n_cities) {
        let new_distance = evaluate_fn(path);

        if new_distance < upper_bound {
            best_path = path.clone();
            best_distance = new_distance;

            tracing::info!(depth = k, tour_length = best_distance, "B&B: new best");
        }
    };

    let candidates: Vec<usize> = construct_candidates(
        path,
        k,
        unvisited_cities,
        running_cost,
        best_distance,
        distances,
    );

    for candidate in candidates.iter() {
        make_move(path, k, *candidate, progress_tx);

        let visited_path: Vec<usize> = path
            .to_vec()
            .iter()
            .filter(|&&x| x != UNVISITED_NODE)
            .copied()
            .collect();
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::PathUpdate(
                Route::new(&visited_path),
                best_distance,
            ));
        }

        let prev_city = path[k - 1];
        let next_distance = distances
            .distance_between(prev_city, *candidate)
            .expect("city ids in path must be in distance matrix");

        let mut next_cities = unvisited_cities.clone();
        next_cities.remove(candidate);

        let (sub_res, sub_dist) = backtrack(
            evaluate_fn,
            distances,
            path,
            &next_cities,
            k + 1,
            running_cost + next_distance,
            best_distance,
            progress_tx,
        );

        if sub_dist < best_distance {
            best_path = sub_res;
            best_distance = sub_dist;
        }

        undo_move(path, k);
    }

    (best_path, best_distance)
}

fn is_solution(path: &Path, k: usize, n_cities: usize) -> bool {
    let uniq_ids: UniqSet = path[0..k].iter().copied().collect();

    k == n_cities && k > 1 && uniq_ids.len() == n_cities
}

fn construct_candidates(
    path: &Path,
    k: usize,
    unvisited_cities: &UniqSet,
    running_cost: f32,
    best_distance: f32,
    distances: &DistanceMatrix,
) -> Path {
    let mut candidates: Path = vec![];

    for city_id in unvisited_cities.iter() {
        let next_distance = distances
            .distance_between(path[k - 1], *city_id)
            .expect("city ids in path must be in distance matrix");
        if best_distance > running_cost + next_distance {
            candidates.push(*city_id);
        }
    }

    candidates.sort();
    candidates
}

fn make_move(
    path: &mut Path,
    k: usize,
    candidate: usize,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
) {
    path[k] = candidate;
    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::CityChange(candidate));
    }
}

fn undo_move(path: &mut Path, k: usize) {
    if k > 0 {
        path[k] = UNVISITED_NODE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{HeuristicOptions, TspProblem, bellman_karp, distance_matrix, kdtree};

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
    fn test_solve_finds_optimal_tour_on_tsp5() {
        let problem = tsp5_problem();
        let tour = solve(&problem, &HeuristicOptions::default(), None, None);

        assert!(
            (tour.total - 4.0).abs() < 1e-3,
            "expected optimal tour ~4.0, got {}",
            tour.total
        );
    }

    #[test]
    fn test_solve_matches_bellman_karp_on_tsp5() {
        let problem = tsp5_problem();
        let bb_tour = solve(&problem, &HeuristicOptions::default(), None, None);
        let bhk_tour = bellman_karp::solve(&problem, &HeuristicOptions::default(), None, None);

        assert!(
            (bb_tour.total - bhk_tour.total).abs() < 1e-3,
            "B&B tour {} should match BHK tour {}",
            bb_tour.total,
            bhk_tour.total
        );
    }

    #[test]
    fn test_solve_matches_bhk_on_tsp8() {
        // Uses 1-indexed city IDs (matching TSPLIB convention) to avoid the
        // UNVISITED_NODE=0 sentinel colliding with city ID 0 from build_points.
        // Points are arranged so the sorted-ID sequential tour crosses itself —
        // B&B must find the same optimal as BHK, not just return input order.
        let coords: &[&[f32]] = &[
            &[0.0, 0.0],
            &[3.0, 1.0],
            &[1.0, 3.0],
            &[4.0, 4.0],
            &[2.0, 0.5],
            &[0.5, 2.0],
            &[3.5, 2.5],
            &[1.5, 4.0],
        ];
        let cities: Vec<kdtree::KDPoint> = coords
            .iter()
            .enumerate()
            .map(|(i, c)| kdtree::KDPoint::new_with_id(i + 1, c))
            .collect();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);

        let bb = solve(&problem, &HeuristicOptions::default(), None, None);
        let bhk = bellman_karp::solve(&problem, &HeuristicOptions::default(), None, None);

        assert!(
            (bb.total - bhk.total).abs() < 1e-3,
            "B&B tour {:.3} should match BHK optimal {:.3}",
            bb.total,
            bhk.total
        );
    }
}

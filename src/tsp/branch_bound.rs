use std::collections::HashSet;
use std::rc::Rc;
use std::sync::mpsc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution};

const UNVISITED_NODE: usize = 0;

type UniqSet = HashSet<usize>;
type Path = Vec<usize>;
type PathEvaluator = Rc<dyn Fn(&Path) -> f32>;

pub fn solve(
    cities: &[KDPoint],
    distances: &DistanceMatrix,
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _initial_tour: Option<&[usize]>,
) -> Solution {
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
    Solution::new(&best_path, cities, distances)
}

fn build_evaluator(distances: &DistanceMatrix) -> PathEvaluator {
    let dm = Rc::new(distances.clone());

    Rc::new(move |path: &Path| dm.tour_length(path))
}

#[allow(clippy::only_used_in_recursion)]
fn backtrack(
    evaluate_fn: &PathEvaluator,
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
        evaluate_fn,
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
        let next_distance = evaluate_fn(&vec![prev_city, *candidate]);

        let mut next_cities = unvisited_cities.clone();
        next_cities.remove(candidate);

        let (sub_res, sub_dist) = backtrack(
            evaluate_fn,
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
    evaluate_fn: &PathEvaluator,
) -> Path {
    let mut candidates: Path = vec![];

    for city_id in unvisited_cities.iter() {
        let next_distance = evaluate_fn(&vec![path[k - 1], *city_id]);
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
    use crate::tsp::{bellman_karp, distance_matrix, kdtree, HeuristicOptions};

    fn tsp5_cities() -> Vec<kdtree::KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ])
    }

    #[test]
    fn test_solve_visits_all_cities() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let tour = solve(&cities, &dm, &HeuristicOptions::default(), None, None);

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_solve_finds_optimal_tour_on_tsp5() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let tour = solve(&cities, &dm, &HeuristicOptions::default(), None, None);

        assert!(
            (tour.total - 4.0).abs() < 1e-3,
            "expected optimal tour ~4.0, got {}",
            tour.total
        );
    }

    #[test]
    fn test_solve_matches_bellman_karp_on_tsp5() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let bb_tour = solve(&cities, &dm, &HeuristicOptions::default(), None, None);
        let bhk_tour = bellman_karp::solve(&cities, &dm, &HeuristicOptions::default(), None, None);

        assert!(
            (bb_tour.total - bhk_tour.total).abs() < 1e-3,
            "B&B tour {} should match BHK tour {}",
            bb_tour.total,
            bhk_tour.total
        );
    }
}

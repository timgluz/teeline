use std::collections::HashSet;
use std::rc::Rc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::route::Route;
use super::{Solution, SolverOptions};

const UNVISITED_NODE: usize = 0;

type UniqSet = HashSet<usize>;
type Path = Vec<usize>;
type PathEvaluator = Rc<dyn Fn(&Path) -> f32>;

// TODO: add better strategy or constraints for Bounding step
pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    let mut route = Route::from_cities(cities);
    let n_cities = route.len();

    // we will start from city with smallest ID
    route.sort();

    let mut open_path: Path = vec![0; n_cities];
    open_path[0] = route.get(0).unwrap();

    // at the beginning all cities the except the first city are unvisited
    let unvisited_cities: UniqSet = route.route().iter().skip(1).map(|x| x.clone()).collect();

    let fitness_fn = build_evaluator(cities);
    let (best_path, _best_distance) = backtrack(
        &fitness_fn,
        &mut open_path,
        &unvisited_cities,
        1, // we start backtracking from second city
        0.0,
        f32::MAX,
        options,
    );

    Solution::new(&best_path, cities)
}

fn build_evaluator(cities: &[KDPoint]) -> PathEvaluator {
    let dm = Rc::new(DistanceMatrix::from_cities(cities).expect("Failed to build distance matrix"));

    return Rc::new(move |path: &Path| dm.tour_length(path));
}

fn backtrack(
    evaluate_fn: &PathEvaluator,
    path: &mut Path,
    unvisited_cities: &UniqSet,
    k: usize,
    running_cost: f32,
    upper_bound: f32,
    options: &SolverOptions,
) -> (Path, f32) {
    let mut best_path = path.clone();
    let mut best_distance = upper_bound;

    let n_cities = k + unvisited_cities.len();
    if is_solution(path.as_ref(), k, n_cities) {
        let new_distance = evaluate_fn(path);

        if new_distance < upper_bound {
            best_path = path.clone();
            best_distance = new_distance;

            if options.verbose {
                println!("B&B: epoch.{:?}, new best distance {:?}", k, best_distance);
            }
        }
    };

    let candidates: Vec<usize> = construct_candidates(
        &path,
        k,
        &unvisited_cities,
        running_cost,
        best_distance,
        evaluate_fn,
    );

    for candidate in candidates.iter() {
        make_move(path, k, candidate.clone());
        let prev_city = path[k - 1];
        let next_distance = evaluate_fn(&vec![prev_city, candidate.clone()]);

        let mut next_cities = unvisited_cities.clone();
        next_cities.remove(candidate);

        let (sub_res, sub_dist) = backtrack(
            evaluate_fn,
            path,
            &next_cities,
            k + 1,
            running_cost + next_distance,
            best_distance,
            options,
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
    let uniq_ids: UniqSet = path[0..k].iter().map(|c| c.clone()).collect();

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
    let mut candidates: Path = vec![]; // always start from city.id 0

    for city_id in unvisited_cities.iter() {
        let next_distance = evaluate_fn(&vec![path[k - 1], city_id.clone()]);
        // simple pruning
        if best_distance > running_cost + next_distance {
            candidates.push(city_id.clone());
        }
    }

    candidates.sort(); // try to keep lexikographic order
    return candidates;
}

fn make_move(path: &mut Path, k: usize, candidate: usize) {
    path[k] = candidate
}

fn undo_move(path: &mut Path, k: usize) {
    // we wouldnt change the first city
    if k > 0 {
        path[k] = UNVISITED_NODE;
    }
}

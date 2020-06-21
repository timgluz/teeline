use std::collections::HashSet;
use std::rc::Rc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::route::Route;
use super::tour::Tour;

const UNVISITED_NODE: usize = 0;

type UniqSet = HashSet<usize>;
type Path = Vec<usize>;
type PathEvaluator = Rc<dyn Fn(&Path) -> f32>;

pub fn solve(cities: &[KDPoint]) -> Tour {
    let route = Route::from_cities(cities);
    let n_cities = route.len();

    // todo: rename tour_length -> distance
    // todo: tour_length should return number of cities on final track
    let mut open_path: Path = vec![0; n_cities];
    let mut unvisited_cities: UniqSet = route.route().iter().map(|x| x.clone()).collect();
    unvisited_cities.remove(&0); // we start always from 0

    let fitness_fn = build_evaluator(cities);
    let (best_path, _best_distance) = backtrack(
        &fitness_fn,
        &mut open_path,
        &unvisited_cities,
        1,
        0.0,
        f32::MAX,
    );

    Tour::new(&best_path, cities)
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
) -> (Path, f32) {
    let mut best_path = path.clone();
    let mut best_distance = upper_bound;

    let n_cities = k + unvisited_cities.len();
    if is_solution(path.as_ref(), k, n_cities) {
        let new_distance = evaluate_fn(path);

        if new_distance < upper_bound {
            best_path = path.clone();
            best_distance = new_distance;
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
    path[k] = if k == 0 {
        // alwaws start from city_id=0
        0
    } else {
        candidate
    }
}

fn undo_move(path: &mut Path, k: usize) {
    if k > 0 {
        path[k] = UNVISITED_NODE;
    }
}

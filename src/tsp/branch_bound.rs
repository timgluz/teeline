use std::collections::HashSet;
use std::rc::Rc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::route::Route;
use super::tour::Tour;

const UNVISITED_NODE: usize = 0;
const MAX_EPOCH: usize = 100;

type TabuList = HashSet<usize>;
type Path = Vec<usize>;
type PathEvaluator = Rc<dyn Fn(&Path) -> f32>;

pub fn solve(cities: &[KDPoint]) -> Tour {
    let route = Route::from_cities(cities);
    let n_cities = route.len();

    // todo: rename tour_length -> distance
    // todo: tour_length should return number of cities on final track
    let mut open_path: Path = vec![0; n_cities];
    let mut unvisited_cities: TabuList = route.route()[1..].iter().map(|x| x.clone()).collect();
    unvisited_cities.remove(&0); // we start always from 0

    let fitness_fn = build_evaluator(cities);
    let (best_path, best_distance) = backtrack(
        &fitness_fn,
        &mut open_path,
        &unvisited_cities,
        1,
        f32::MAX,
        n_cities,
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
    unvisited_cities: &TabuList,
    k: usize,
    upper_bound: f32,
    n_cities: usize,
) -> (Path, f32) {
    let mut epoch = 0;
    let mut best_path = path.clone();
    let mut best_distance = upper_bound;

    if is_solution(path.as_ref(), k, n_cities) {
        let new_distance = evaluate_fn(path);

        if upper_bound > new_distance {
            best_path = path.clone();
            best_distance = new_distance;
        }
    };

    let candidates: Vec<usize> = construct_candidates(&path, k, &unvisited_cities, best_distance);

    println!("Current path: {:?}", path[0..k].to_vec());
    for candidate in candidates.iter() {
        make_move(path, k, candidate.clone());
        println!("At city: {:?}, trying: {:?}", path[k - 1], candidate);

        let mut next_cities = unvisited_cities.clone();
        next_cities.remove(candidate);

        let (sub_res, sub_dist) = backtrack(
            evaluate_fn,
            path,
            &next_cities,
            k + 1,
            best_distance,
            n_cities,
        );

        if best_distance > sub_dist {
            println!("Found better distance: {:?}", sub_dist);
            best_path = sub_res;
            best_distance = sub_dist;
        }

        undo_move(path, k);

        epoch += 1;
        if epoch > MAX_EPOCH {
            break;
        }
    }

    (best_path, best_distance)
}

fn is_solution(path: &Path, k: usize, n_cities: usize) -> bool {
    // TODO: check that items are unique
    k == n_cities && k > 1 && path[k - 1] != 0
}

fn construct_candidates(
    path: &Path,
    k: usize,
    unvisited_cities: &TabuList,
    best_distance: f32,
) -> Path {
    let mut candidates: Path = vec![]; // always start from city.id 0
    let visited_cities = path[0..k].to_vec();

    for city_id in unvisited_cities.iter() {
        // TODO also prune bad branches
        candidates.push(city_id.clone());
    }

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

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::mpsc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::{self, KDPoint, KDTree};
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
    opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
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

    // Seed the upper bound from the provided heuristic tour so the very first
    // descent prunes aggressively. Without this, the bound starts at f32::MAX
    // and proof-of-optimality requires exhausting nearly all branches.
    let initial_upper_bound = init_tour
        .map(|t| distances.tour_length(t))
        .unwrap_or(f32::MAX);

    let n_nearest = opts.n_nearest;
    // Gate: only build KD-tree when n_nearest actually restricts the candidate
    // set. When n_nearest >= n_cities every city is a candidate anyway — skip
    // the overhead and keep exact B&B behaviour.
    let kd_tree: Option<KDTree> = if n_nearest > 0 && n_nearest < n_cities {
        Some(kdtree::from_cities(cities))
    } else {
        None
    };
    let city_map: HashMap<usize, KDPoint> =
        cities.iter().map(|p| (p.id, *p)).collect();

    let fitness_fn = build_evaluator(distances);
    let (best_path, best_distance) = backtrack(
        &fitness_fn,
        distances,
        &city_map,
        &mut open_path,
        &unvisited_cities,
        1,
        0.0,
        initial_upper_bound,
        progress_tx,
        kd_tree.as_ref(),
        n_nearest,
    );

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }

    // If backtracking found no strictly better tour than the seed, best_path is
    // still the partial scratch buffer. Return the seed tour as the result.
    let result_path = if best_distance < initial_upper_bound {
        best_path
    } else if let Some(t) = init_tour {
        t.to_vec()
    } else {
        best_path
    };
    Solution::from_parts(&result_path, cities, distances)
}

fn build_evaluator(distances: &DistanceMatrix) -> PathEvaluator {
    let dm = Rc::new(distances.clone());

    Rc::new(move |path: &Path| dm.tour_length(path))
}

#[allow(clippy::only_used_in_recursion, clippy::too_many_arguments)]
fn backtrack(
    evaluate_fn: &PathEvaluator,
    distances: &DistanceMatrix,
    city_map: &HashMap<usize, KDPoint>,
    path: &mut Path,
    unvisited_cities: &UniqSet,
    k: usize,
    running_cost: f32,
    upper_bound: f32,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    kd: Option<&KDTree>,
    n_nearest: usize,
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
        city_map,
        kd,
        n_nearest,
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
            city_map,
            path,
            &next_cities,
            k + 1,
            running_cost + next_distance,
            best_distance,
            progress_tx,
            kd,
            n_nearest,
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
    city_map: &HashMap<usize, KDPoint>,
    kd: Option<&KDTree>,
    n_nearest: usize,
) -> Path {
    if unvisited_cities.is_empty() {
        return vec![];
    }

    // When a KD-tree is available, restrict candidates to the n_nearest
    // geometrically-closest unvisited cities. Fall back to the full unvisited
    // set only if zero neighbors survive the intersection (last few cities in
    // the tour where all geometric neighbors are already visited).
    let candidate_ids: Vec<usize> = match kd {
        Some(tree) => {
            let current_id = path[k - 1];
            let current_pt = city_map
                .get(&current_id)
                .expect("current city must be in city_map");

            let restricted: Vec<usize> = tree
                .nearest(current_pt, n_nearest)
                .nearest()
                .iter()
                .map(|item| item.point.id)
                .filter(|id| unvisited_cities.contains(id))
                .collect();

            if restricted.is_empty() {
                unvisited_cities.iter().copied().collect()
            } else {
                restricted
            }
        }
        None => unvisited_cities.iter().copied().collect(),
    };

    // MST of {start_city} ∪ unvisited is a lower bound on the minimum cost of
    // completing the tour from any candidate through the remaining cities back
    // to the start. Computed on ALL remaining unvisited cities (not just the
    // restricted candidates) so the bound stays valid.
    let start_city = path[0];
    let mst_nodes: Vec<usize> = std::iter::once(start_city)
        .chain(unvisited_cities.iter().copied())
        .collect();
    let mst = prim_mst(&mst_nodes, distances);

    // Score each candidate by its lower bound, then sort ascending so the most
    // promising branches are explored first (tightens best_distance faster).
    let mut scored: Vec<(f32, usize)> = candidate_ids
        .iter()
        .filter_map(|&city_id| {
            let next_dist = distances
                .distance_between(path[k - 1], city_id)
                .expect("city ids in path must be in distance matrix");
            let lb = running_cost + next_dist + mst;
            if best_distance > lb { Some((lb, city_id)) } else { None }
        })
        .collect();
    scored.sort_by(|a, b| a.0.total_cmp(&b.0));
    scored.into_iter().map(|(_, id)| id).collect()
}

fn prim_mst(city_ids: &[usize], distances: &DistanceMatrix) -> f32 {
    let n = city_ids.len();
    if n <= 1 {
        return 0.0;
    }
    let mut in_tree = vec![false; n];
    let mut min_key = vec![f32::MAX; n];
    min_key[0] = 0.0;
    let mut total = 0.0;
    for _ in 0..n {
        let u = (0..n)
            .filter(|&i| !in_tree[i])
            .min_by(|&a, &b| min_key[a].total_cmp(&min_key[b]))
            .unwrap();
        in_tree[u] = true;
        total += min_key[u];
        for v in 0..n {
            if !in_tree[v] {
                let d = distances
                    .distance_between(city_ids[u], city_ids[v])
                    .expect("city ids in MST must be in distance matrix");
                if d < min_key[v] {
                    min_key[v] = d;
                }
            }
        }
    }
    total
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
        // n_nearest=0: no KD restriction — exact B&B.
        let opts = HeuristicOptions { n_nearest: 0, ..Default::default() };
        let tour = solve(&problem, &opts, None, None);

        assert!(
            (tour.total - 4.0).abs() < 1e-3,
            "expected optimal tour ~4.0, got {}",
            tour.total
        );
    }

    #[test]
    fn test_solve_matches_bellman_karp_on_tsp5() {
        let problem = tsp5_problem();
        let opts = HeuristicOptions { n_nearest: 0, ..Default::default() };
        let bb_tour = solve(&problem, &opts, None, None);
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

        // n_nearest=0: no KD restriction — exact B&B.
        let opts = HeuristicOptions { n_nearest: 0, ..Default::default() };
        let bb = solve(&problem, &opts, None, None);
        let bhk = bellman_karp::solve(&problem, &HeuristicOptions::default(), None, None);

        assert!(
            (bb.total - bhk.total).abs() < 1e-3,
            "B&B tour {:.3} should match BHK optimal {:.3}",
            bb.total,
            bhk.total
        );
    }

    #[test]
    fn test_solve_with_init_tour_finds_same_optimal() {
        // When a good init_tour is provided the upper bound is seeded tightly,
        // pruning more branches early. The result must still be the exact optimal.
        let problem = tsp5_problem();
        // n_nearest=0: no KD restriction — exact B&B.
        let opts = HeuristicOptions { n_nearest: 0, ..Default::default() };
        let without_seed = solve(&problem, &opts, None, None);

        // Use the unseeded result as the init_tour for the second run.
        let seed_route = without_seed.route().to_vec();
        let with_seed = solve(
            &problem,
            &opts,
            None,
            Some(&seed_route),
        );

        assert!(
            (with_seed.total - without_seed.total).abs() < 1e-3,
            "seeded B&B {:.3} should equal unseeded {:.3}",
            with_seed.total,
            without_seed.total
        );
    }

    #[test]
    fn test_prim_mst_triangle() {
        // 3 cities: right triangle with legs 3 and 4. MST picks the two shorter
        // edges (3+4=7) and excludes the hypotenuse (5). Uses 1-indexed IDs to
        // avoid UNVISITED_NODE=0 collision with city ID 0 from build_points.
        let cities: Vec<kdtree::KDPoint> = [
            [0.0f32, 0.0], [3.0, 0.0], [0.0, 4.0],
        ].iter().enumerate()
         .map(|(i, c)| kdtree::KDPoint::new_with_id(i + 1, c))
         .collect();
        let dm = distance_matrix::from_cities(&cities);
        let mst = prim_mst(&[1, 2, 3], &dm);
        assert!((mst - 7.0).abs() < 1e-3, "MST of 3-4-5 triangle should be 7, got {mst}");
    }

    #[test]
    fn test_n_nearest_3_finds_same_optimal_on_tsp5() {
        // n_nearest=3 < 5 cities — KD restriction is active, but tsp5 is
        // simple enough that the optimal successor is always within the 3
        // nearest, so the result must match exact B&B.
        let problem = tsp5_problem();
        let restricted_opts = HeuristicOptions { n_nearest: 3, ..Default::default() };
        let exact_opts = HeuristicOptions { n_nearest: 0, ..Default::default() };

        let restricted = solve(&problem, &restricted_opts, None, None);
        let exact = solve(&problem, &exact_opts, None, None);

        assert!(
            (restricted.total - exact.total).abs() < 1e-3,
            "n_nearest=3 tour {:.3} should match exact {:.3}",
            restricted.total,
            exact.total,
        );
    }

    #[test]
    fn test_n_nearest_visits_all_cities_on_tsp8() {
        // Same 8-city layout as test_solve_matches_bhk_on_tsp8.
        // With n_nearest=4, the KD restriction is active (4 < 8).
        let coords: &[&[f32]] = &[
            &[0.0, 0.0], &[3.0, 1.0], &[1.0, 3.0], &[4.0, 4.0],
            &[2.0, 0.5], &[0.5, 2.0], &[3.5, 2.5], &[1.5, 4.0],
        ];
        let cities: Vec<kdtree::KDPoint> = coords
            .iter()
            .enumerate()
            .map(|(i, c)| kdtree::KDPoint::new_with_id(i + 1, c))
            .collect();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);

        let restricted_opts = HeuristicOptions { n_nearest: 4, ..Default::default() };
        let tour = solve(&problem, &restricted_opts, None, None);

        // All 8 cities visited exactly once.
        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![1, 2, 3, 4, 5, 6, 7, 8]);

        // Gap vs unrestricted B&B should be < 10%.
        let exact_opts = HeuristicOptions { n_nearest: 0, ..Default::default() };
        let exact = solve(&problem, &exact_opts, None, None);
        let gap = (tour.total - exact.total) / exact.total;
        assert!(
            gap < 0.10,
            "n_nearest=4 gap {:.1}% should be < 10% vs exact {:.3}",
            gap * 100.0,
            exact.total,
        );
    }
}

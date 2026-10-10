use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem};

pub fn solve(
    problem: &TspProblem,
    // `nn` is the one solver that reads nothing from HeuristicOptions: the greedy step
    // needs the nearest unvisited city, and `n_nearest` is a candidate-list limit that
    // only makes sense for solvers trading exactness for speed. Underscored rather than
    // removed to keep the uniform `solve(problem, opts, tx, init)` signature every solver
    // shares, and so the signature does not change if it ever needs an option again.
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    // Deliberately does not log `opts.n_nearest`: the greedy step ignores it, and a log
    // naming a field with no effect on the result invites someone debugging a
    // reproducibility problem to blame it. Mirrors `greedy_edge::solve`, which also
    // ignores HeuristicOptions and logs only `cities`.
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

        // Greedy nearest neighbour means *the nearest unvisited city*, full stop.
        //
        // This deliberately does not use `opts.n_nearest`. That option bounds a
        // candidate list for algorithms that genuinely trade exactness for speed
        // (`lin_kernighan`, `branch_bound`), but using it here made the greedy step
        // approximate: "the nearest unvisited among the k nearest overall" is not the
        // nearest unvisited, and it silently produced worse tours. It was also the
        // first half of the run-to-run variance bug — the second half was the
        // HashSet-ordered tie-break that this call replaced with a total order.
        // `expect`, not a silent `break`: the loop guard says `unvisited` is non-empty and
        // `current_city` comes from the same city set the matrix was built from, so a `None`
        // here means one of those invariants is broken. Breaking quietly would truncate the
        // tour and return a shorter "solution" with no diagnostic — exactly the
        // silent-wrong-answer class of bug this rewrite exists to remove, so fail loudly.
        let nearest = distances
            .nearest_unvisited(current_city, |id| {
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
    Solution::from_parts(&path, cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{HeuristicOptions, TspProblem, distance_matrix, kdtree};

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
}

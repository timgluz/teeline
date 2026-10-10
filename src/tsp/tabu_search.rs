use std::collections::VecDeque;
use std::sync::mpsc;

use super::budget::Budget;
use super::distance_matrix::DistanceMatrix;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem};

pub fn solve(
    problem: &TspProblem,
    opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let tabu_capacity = cities.len();

    tracing::info!(epochs = opts.epochs, tabu_capacity, "tabu search starting");

    let mut tabu_list = TabuList::new(tabu_capacity);

    let mut best_route = init_tour
        .map(Route::new)
        .unwrap_or_else(|| Route::from_cities(cities));
    tabu_list.add(best_route.clone());

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(best_route.clone(), 0.0));
    }

    let mut u = best_route.clone();
    let mut best_distance = distances.tour_length(u.route());
    // Two translations of the old `update_terminate` convention, kept so that a caller who never asked
    // for the plateau stop gets exactly the run they got before: `epochs == 0` meant unbounded, and
    // `epoch > max_epochs` meant a budget of `epochs` admitted `epochs + 1` iterations. Both are
    // surprising, but changing either here would silently alter those results.
    //
    // `epochs == 0` together with `stagnation_epochs == 0` therefore never terminates. That is
    // pre-existing and left alone: unbounded is what the combination has always meant.
    let epoch_cap = if opts.epochs == 0 {
        usize::MAX
    } else {
        opts.epochs.saturating_add(1)
    };
    let mut budget = Budget::new(epoch_cap, opts.stagnation_epochs);
    let mut improved = true;

    while budget.record(improved) {
        let epoch = budget.index();
        let best_at_epoch_start = best_distance;
        let (local_best, local_distance) = select(distances, &u, &tabu_list);
        if local_distance < best_distance {
            best_route = local_best.clone();
            best_distance = local_distance;

            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::PathUpdate(
                    best_route.clone(),
                    best_distance,
                ));
            }

            tracing::info!(epoch, tour_length = local_distance, "tabu: new best");
        }

        tabu_list.add(u.clone());
        u = local_best;

        improved = best_distance < best_at_epoch_start;
    }

    if budget.converged() {
        tracing::info!(
            epoch = budget.epoch(),
            stagnation_epochs = budget.stale_epochs(),
            "tabu: converged, no improvement for the stagnation limit"
        );
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(best_route.route(), cities, distances)
}

fn select(distances: &DistanceMatrix, route: &Route, tabu_list: &TabuList) -> (Route, f32) {
    let local_best = distances.tour_length(route.route());

    let mut candidate = route.random_successor();
    let mut candidate_distance = distances.tour_length(candidate.route());

    for _ in 0..route.len() {
        if candidate_distance < local_best && !tabu_list.contains(&candidate) {
            break;
        }

        candidate = route.random_successor();
        candidate_distance = distances.tour_length(candidate.route());
    }

    (candidate, candidate_distance)
}

struct TabuList {
    pub capacity: usize,
    items: VecDeque<Route>,
}

impl TabuList {
    pub fn new(capacity: usize) -> Self {
        TabuList {
            capacity,
            items: VecDeque::with_capacity(capacity),
        }
    }

    pub fn add(&mut self, route: Route) {
        if self.items.len() >= self.capacity {
            self.items.pop_back();
        }
        self.items.push_front(route);
    }

    pub fn contains(&self, route: &Route) -> bool {
        self.items.contains(route)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::route::Route;
    use crate::tsp::{HeuristicOptions, TspProblem, distance_matrix, kdtree, kdtree::KDPoint};

    fn tsp5_cities() -> Vec<KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ])
    }

    #[test]
    fn test_tabu_list_does_not_contain_unseen_route() {
        let tabu = TabuList::new(5);
        assert!(!tabu.contains(&Route::new(&[0, 1, 2])));
    }

    #[test]
    fn test_tabu_list_contains_added_route() {
        let mut tabu = TabuList::new(5);
        let route = Route::new(&[0, 1, 2]);
        tabu.add(route.clone());
        assert!(tabu.contains(&route));
    }

    #[test]
    fn test_tabu_list_evicts_oldest_when_full() {
        let mut tabu = TabuList::new(2);
        let r1 = Route::new(&[0, 1, 2]);
        let r2 = Route::new(&[1, 0, 2]);
        let r3 = Route::new(&[2, 1, 0]);
        tabu.add(r1.clone());
        tabu.add(r2.clone());
        tabu.add(r3.clone());
        assert!(!tabu.contains(&r1), "oldest route should have been evicted");
        assert!(tabu.contains(&r2));
        assert!(tabu.contains(&r3));
    }

    #[test]
    fn test_tabu_list_capacity_is_set_correctly() {
        let tabu = TabuList::new(7);
        assert_eq!(tabu.capacity, 7);
    }

    #[test]
    fn test_tabu_respects_initial_tour() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let optimal: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let optimal_cost = dm.tour_length(&optimal);
        let opts = HeuristicOptions {
            epochs: 1,
            ..HeuristicOptions::default()
        };
        let problem = TspProblem::new(cities, dm);
        let result = solve(&problem, &opts, None, Some(&optimal));
        assert!((result.total - optimal_cost).abs() < 1e-4);
    }

    #[test]
    fn test_solve_visits_all_cities() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities.clone(), dm);
        let opts = HeuristicOptions {
            epochs: 200,
            ..HeuristicOptions::default()
        };
        let tour = solve(&problem, &opts, None, None);

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_solve_tour_length_is_positive() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);
        let opts = HeuristicOptions {
            epochs: 200,
            ..HeuristicOptions::default()
        };
        let tour = solve(&problem, &opts, None, None);

        assert!(tour.total > 0.0, "tour length must be positive");
    }
}

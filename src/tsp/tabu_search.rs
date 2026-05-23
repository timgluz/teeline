use std::collections::VecDeque;
use std::sync::mpsc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution};

pub fn solve(
    cities: &[KDPoint],
    distances: &DistanceMatrix,
    opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    initial_tour: Option<&[usize]>,
) -> Solution {
    let tabu_capacity = cities.len();

    tracing::info!(epochs = opts.epochs, tabu_capacity, "tabu search starting");

    let mut tabu_list = TabuList::new(tabu_capacity);

    let mut best_route = initial_tour
        .map(Route::new)
        .unwrap_or_else(|| Route::from_cities(cities));
    tabu_list.add(best_route.clone());

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(best_route.clone(), 0.0));
    }

    let mut u = best_route.clone();
    let mut best_distance = distances.tour_length(u.route());
    let mut done = false;
    let mut epoch = 0;
    while !done {
        let (local_best, local_distance) = select(distances, &u, &tabu_list);
        if local_distance < best_distance {
            best_route = local_best.clone();
            best_distance = local_distance;

            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::PathUpdate(best_route.clone(), best_distance));
            }

            tracing::info!(epoch, tour_length = local_distance, "tabu: new best");
        }

        tabu_list.add(u.clone());
        u = local_best;

        epoch += 1;
        done = update_terminate(epoch, opts.epochs);
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::new(best_route.route(), cities, distances)
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

fn update_terminate(epoch: usize, max_epochs: usize) -> bool {
    max_epochs > 0 && epoch > max_epochs
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
    use crate::tsp::{distance_matrix, kdtree, HeuristicOptions};
    use crate::tsp::route::Route;

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
        let opts = HeuristicOptions { epochs: 1, ..HeuristicOptions::default() };
        let result = solve(&cities, &dm, &opts, None, Some(&optimal));
        assert!((result.total - optimal_cost).abs() < 1e-4);
    }

    #[test]
    fn test_solve_visits_all_cities() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let opts = HeuristicOptions { epochs: 200, ..HeuristicOptions::default() };
        let tour = solve(&cities, &dm, &opts, None, None);

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_solve_tour_length_is_positive() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let opts = HeuristicOptions { epochs: 200, ..HeuristicOptions::default() };
        let tour = solve(&cities, &dm, &opts, None, None);

        assert!(tour.total > 0.0, "tour length must be positive");
    }
}

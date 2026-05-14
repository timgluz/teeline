use std::collections::VecDeque;

use super::kdtree::KDPoint;
use super::progress::{send_progress, ProgressMessage};
use super::route::Route;
use super::{total_distance, Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    let tabu_capacity = cities.len();

    tracing::info!(epochs = options.epochs, tabu_capacity, "tabu search starting");

    let mut tabu_list = TabuList::new(tabu_capacity);

    let mut best_route = Route::from_cities(cities);
    tabu_list.add(best_route.clone());

    send_progress(ProgressMessage::PathUpdate(best_route.clone(), 0.0));

    let mut u = best_route.clone();
    let mut best_distance = distance(cities, &u);
    let mut done = false;
    let mut epoch = 0;
    while !done {
        let (local_best, local_distance) = select(cities, &u, &tabu_list);
        if local_distance < best_distance {
            best_route = local_best.clone();
            best_distance = local_distance;

            send_progress(ProgressMessage::PathUpdate(
                best_route.clone(),
                best_distance,
            ));

            tracing::info!(epoch, tour_length = local_distance, "tabu: new best");
        }

        // refine tabu list
        tabu_list.add(u.clone());
        u = local_best; // continue search from local best

        epoch += 1;
        done = update_terminate(epoch, options.epochs);
    }

    send_progress(ProgressMessage::Done);
    Solution::new(best_route.route(), cities)
}

fn select(cities: &[KDPoint], route: &Route, tabu_list: &TabuList) -> (Route, f32) {
    let local_best = distance(cities, route);

    let mut candidate = route.random_successor();
    let mut candidate_distance = distance(cities, &candidate);

    // try to local best
    for _ in 0..route.len() {
        if candidate_distance < local_best && !tabu_list.contains(&candidate) {
            break;
        }

        candidate = route.random_successor();
        candidate_distance = distance(cities, &candidate);
    }

    (candidate, candidate_distance)
}

fn distance(cities: &[KDPoint], route: &Route) -> f32 {
    total_distance(cities, route.route())
}

fn update_terminate(epoch: usize, max_epochs: usize) -> bool {
    max_epochs > 0 && epoch > max_epochs
}

struct TabuList {
    pub capacity: usize, // number of max items we are going to block
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
        // if queue is full drop the oldest item
        if self.items.len() >= self.capacity {
            self.items.pop_back();
        }

        // add new item at the beginning
        self.items.push_front(route);
    }

    pub fn contains(&self, route: &Route) -> bool {
        self.items.contains(route)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::kdtree;
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
        tabu.add(r3.clone()); // r1 (oldest, at back) should be evicted
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
    fn test_solve_visits_all_cities() {
        let cities = tsp5_cities();
        let mut options = SolverOptions::default();
        options.epochs = 200;
        let tour = solve(&cities, &options);

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_solve_tour_length_is_positive() {
        let cities = tsp5_cities();
        let mut options = SolverOptions::default();
        options.epochs = 200;
        let tour = solve(&cities, &options);

        assert!(tour.total > 0.0, "tour length must be positive");
    }
}

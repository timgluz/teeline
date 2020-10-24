use std::collections::VecDeque;

use super::kdtree::KDPoint;
use super::progress::{send_progress, ProgressMessage};
use super::route::Route;
use super::{total_distance, Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    let tabu_capacity = cities.len();

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

            if options.verbose {
                println!(
                    "Tabusearch: epoch.{:?} new best {:?}",
                    epoch, local_distance
                );
            }
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

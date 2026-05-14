use std::collections::HashMap;

use super::kdtree::{self, KDPoint};
use super::progress::{send_progress, ProgressMessage};
use super::route::Route;
use super::{Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    tracing::info!(n_nearest = options.n_nearest, cities = cities.len(), "NN starting");

    let search_tree = kdtree::from_cities(cities);
    let n_nearest = options.n_nearest;

    let cities_table: HashMap<usize, KDPoint> = cities.iter().map(|c| (c.id, c.clone())).collect();
    let mut path: Vec<usize> = cities.iter().map(|c| c.id).collect();

    send_progress(ProgressMessage::PathUpdate(Route::new(&path), 0.0));
    // run optimization round
    for i in 0..(path.len() - 1) {
        let id1 = path[i];
        let city1 = cities_table[&id1].clone();
        send_progress(ProgressMessage::CityChange(id1));

        let frontier = search_tree.nearest(&city1, n_nearest);

        let id2 = path[i + 1];
        let current_distance = city1.distance(&cities_table[&id2]);

        let search_result = frontier.nearest();
        if search_result.is_empty() {
            tracing::debug!(city_id = id1, "NN: no nearest found");
            continue;
        }

        let closest_item = search_result.first().unwrap();
        let next_distance = closest_item.distance;

        if next_distance < current_distance {
            let nearest_city_id = closest_item.point.id;
            if let Some(nearest_pos) = path.iter().position(|&x| x == nearest_city_id) {
                tracing::debug!(from = id2, to = nearest_city_id, "NN: swap");
                path.swap(i + 1, nearest_pos);

                send_progress(ProgressMessage::PathUpdate(Route::new(&path), 0.0));
            }
        }
    }

    send_progress(ProgressMessage::Done);
    Solution::new(&path, cities)
}

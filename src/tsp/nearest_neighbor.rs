use std::collections::HashMap;

use super::kdtree::{self, KDPoint};
use super::{Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    let search_tree = kdtree::from_cities(&cities);
    let n_nearest = options.n_nearest;

    let cities_table: HashMap<usize, KDPoint> = cities.iter().map(|c| (c.id, c.clone())).collect();
    let mut route: Vec<usize> = cities.iter().map(|c| c.id).collect();

    // run optimization round
    for i in 0..(route.len() - 1) {
        let id1 = route[i];
        let city1 = cities_table[&id1].clone();
        let frontier = search_tree.nearest(&city1, n_nearest);

        let id2 = route[i + 1];
        let current_distance = city1.distance(&cities_table[&id2]);

        let search_result = frontier.nearest();
        if search_result.is_empty() {
            if options.verbose {
                println!("No nearest for city: #{:?}", id1);
            }

            continue;
        }

        let closest_item = search_result.first().unwrap();
        let next_distance = closest_item.distance;

        if next_distance < current_distance {
            let nearest_city_id = closest_item.point.id;
            if let Some(nearest_pos) = route.iter().position(|&x| x == nearest_city_id) {
                route.swap(i + 1, nearest_pos);
            }
        }
    }

    let tour = Solution::new(&route, cities);

    tour
}

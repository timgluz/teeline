use std::collections::{HashMap, HashSet};
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
    _initial_tour: Option<&[usize]>,
) -> Solution {
    tracing::info!(n_nearest = opts.n_nearest, cities = cities.len(), "NN starting");

    let n_nearest = opts.n_nearest;
    let cities_table: HashMap<usize, KDPoint> = cities.iter().map(|c| (c.id, c.clone())).collect();

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

        let frontier = distances.nearest(current_city, n_nearest);
        let next_id = frontier
            .nearest()
            .iter()
            .find(|item| unvisited.contains(&item.point.id))
            .map(|item| item.point.id)
            .unwrap_or_else(|| {
                *unvisited
                    .iter()
                    .min_by(|&&a, &&b| {
                        let da = distances.distance_between(current_id, a).unwrap_or(f32::MAX);
                        let db = distances.distance_between(current_id, b).unwrap_or(f32::MAX);
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .expect("unvisited is non-empty")
            });

        path.push(next_id);
        unvisited.remove(&next_id);
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&path), 0.0));
        }
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::new(&path, cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, HeuristicOptions};

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
    fn test_solve_visits_all_cities() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let tour = solve(&cities, &dm, &HeuristicOptions::default(), None, None);

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_solve_tour_length_is_positive_and_finite() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let tour = solve(&cities, &dm, &HeuristicOptions::default(), None, None);

        assert!(tour.total > 0.0, "tour length should be positive, got {}", tour.total);
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
        let tour = solve(&cities, &dm, &HeuristicOptions::default(), None, None);

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
        let tour = solve(&cities, &dm, &HeuristicOptions::default(), None, None);

        let route = tour.route().to_vec();
        assert_ne!(route, vec![0, 1, 2, 3, 4], "NN produced sorted output (regression)");
        let mut sorted = route.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
    }
}

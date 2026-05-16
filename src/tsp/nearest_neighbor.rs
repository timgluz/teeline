use std::collections::{HashMap, HashSet};

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], distances: &DistanceMatrix, options: &SolverOptions) -> Solution {
    tracing::info!(n_nearest = options.n_nearest, cities = cities.len(), "NN starting");

    let n_nearest = options.n_nearest;
    let cities_table: HashMap<usize, KDPoint> = cities.iter().map(|c| (c.id, c.clone())).collect();

    let mut unvisited: HashSet<usize> = cities.iter().map(|c| c.id).collect();
    let mut path: Vec<usize> = Vec::with_capacity(cities.len());

    // Start from the first city in the input order.
    let start_id = cities[0].id;
    path.push(start_id);
    unvisited.remove(&start_id);

    options.send_progress(ProgressMessage::PathUpdate(Route::new(&path), 0.0));

    while !unvisited.is_empty() {
        let current_id = *path.last().unwrap();
        let current_city = &cities_table[&current_id];

        options.send_progress(ProgressMessage::CityChange(current_id));

        // Find the nearest unvisited city.  Check the n_nearest frontier first (fast path);
        // fall back to a linear scan over `unvisited` when all frontier cities are already
        // visited (common late in the tour when n_nearest is small).
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
        options.send_progress(ProgressMessage::PathUpdate(Route::new(&path), 0.0));
    }

    options.send_progress(ProgressMessage::Done);
    Solution::new(&path, cities, distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree};

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
        let options = SolverOptions::default();
        let tour = solve(&cities, &dm, &options);

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_solve_tour_length_is_positive_and_finite() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let options = SolverOptions::default();
        let tour = solve(&cities, &dm, &options);

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
        let options = SolverOptions::default();
        let tour = solve(&cities, &dm, &options);

        let mut visited: Vec<usize> = tour.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3]);
    }

    // Regression: the old algorithm never tracked visited cities, so it could
    // revisit already-traversed nodes and degrade into a near-sorted walk.
    // With 1-indexed city IDs (as in TSPLIB files), the first city's "nearest"
    // was always city_id+1 due to a position/id mismatch in distance_matrix::nearest,
    // making every swap a no-op.  This test catches both regressions.
    #[test]
    fn test_solve_does_not_produce_sorted_output_on_shuffled_input() {
        // Cities placed so the greedy NN order is NOT 0→1→2→3→4.
        // Optimal greedy path from 0: 0 → 4 → 3 → 2 → 1 (or similar clustering).
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],   // 0 – far from 1
            vec![100.0, 0.0], // 1 – far from 0
            vec![99.0, 0.0],  // 2 – near 1
            vec![98.0, 0.0],  // 3 – near 1 and 2
            vec![1.0, 0.0],   // 4 – near 0
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let options = SolverOptions::default();
        let tour = solve(&cities, &dm, &options);

        let route = tour.route().to_vec();
        // The greedy tour must NOT be [0,1,2,3,4] (sorted) — city 4 is much closer to 0.
        assert_ne!(route, vec![0, 1, 2, 3, 4], "NN produced sorted output (regression)");
        // All cities must still be visited exactly once.
        let mut sorted = route.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
    }
}

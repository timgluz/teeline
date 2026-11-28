//! Bellman–Held–Karp exact solver.
//!
//! The exponential recurrence lives in the shared [`held_karp_path`] primitive, which is
//! also POPMUSIC's exact window sub-solver. This module only adapts a TSP problem to it:
//! the anchor city is both the path start and the end, so the closed tour becomes a
//! fixed-endpoint path through every other city.

use super::held_karp::held_karp_path;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Position, Solution, TspProblem};
use std::sync::mpsc;

pub fn solve(
    problem: &TspProblem,
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n_cities = cities.len();
    let n_others = n_cities - 1;
    let anchor = Position::new(n_others);
    let stride = n_others + 2;

    tracing::info!(n_cities, interior = n_others, "BHK starting");

    if let Some(tx) = progress_tx {
        for i in 0..n_others {
            if let Some(city_id) = distances.pos2city_id(Position::new(i)) {
                let _ = tx.send(ProgressMessage::CityChange(city_id));
            }
        }
    }

    // Local index `i < n_others` is interior position `i`; both `n_others` and
    // `n_others + 1` are the anchor. The DP's distance table is the only place BHK still
    // touches the distance source, which keeps its inner loop identical to POPMUSIC's.
    let local_position = |local: usize| {
        if local < n_others {
            Position::new(local)
        } else {
            anchor
        }
    };
    let mut matrix = vec![0.0f32; stride * stride];
    for a in 0..stride {
        for b in 0..stride {
            matrix[a * stride + b] = distances
                .distance_by_pos(local_position(a), local_position(b))
                .unwrap_or(f32::INFINITY);
        }
    }

    let (cost, interior) = held_karp_path(&matrix, n_others)
        .expect("a complete distance source must admit a finite Hamiltonian path");
    tracing::info!(cost, "BHK: optimal tour found");

    let mut route: Vec<usize> = Vec::with_capacity(n_cities);
    route.push(
        distances
            .pos2city_id(anchor)
            .expect("the anchor position came from the problem"),
    );
    for local in interior {
        route.push(
            distances
                .pos2city_id(Position::new(local))
                .expect("an interior local index is a valid position"),
        );
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&route), cost));
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&route, distances.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{HeuristicOptions, TspProblem, distance_matrix, kdtree};

    fn tsp_5_1_problem() -> TspProblem {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);
        let dm = distance_matrix::from_cities_unrounded(&cities);
        TspProblem::new(cities, dm)
    }

    #[test]
    fn test_solve_returns_all_cities() {
        let problem = tsp_5_1_problem();
        let solution = solve(&problem, &HeuristicOptions::default(), None, None);

        let mut visited: Vec<usize> = solution.route().to_vec();
        visited.sort();
        assert_eq!(
            visited,
            vec![0, 1, 2, 3, 4],
            "every city must appear exactly once"
        );
    }

    #[test]
    fn test_solve_finds_optimal_tour_length() {
        let problem = tsp_5_1_problem();
        let solution = solve(&problem, &HeuristicOptions::default(), None, None);

        assert!(
            (solution.total - 4.0).abs() < 1e-3,
            "expected tour length ~4.0, got {}",
            solution.total
        );
    }

    #[test]
    fn test_route_reconstruction_on_larger_instance() {
        // Regression: reconstruction used f32::EPSILON as the comparison tolerance, which
        // is far too tight for summed f32 distances on 8+ cities and used to leave
        // repeated cities in the route.
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
        let dm = distance_matrix::from_cities_unrounded(&cities);
        let problem = TspProblem::new(cities, dm);

        let solution = solve(&problem, &HeuristicOptions::default(), None, None);

        let mut route = solution.route().to_vec();
        route.sort();
        assert_eq!(
            route,
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            "BHK must visit every city exactly once; got {:?}",
            solution.route()
        );
    }

    #[test]
    fn test_solve_with_3_cities() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![3.0, 0.0], vec![0.0, 4.0]]);
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities, dm);
        let solution = solve(&problem, &HeuristicOptions::default(), None, None);

        let mut visited: Vec<usize> = solution.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2]);

        assert!(
            (solution.total - 12.0).abs() < 1e-3,
            "got {}",
            solution.total
        );
    }

    #[test]
    fn test_solve_is_deterministic() {
        let problem = tsp_5_1_problem();
        let first = solve(&problem, &HeuristicOptions::default(), None, None);
        let second = solve(&problem, &HeuristicOptions::default(), None, None);
        assert_eq!(first.route(), second.route());
        assert_eq!(first.total, second.total);
    }
}

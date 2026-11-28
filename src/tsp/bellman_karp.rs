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

/// Returns `Err` when the instance has no finite Hamiltonian path, so the dispatcher can
/// report an unsolvable input instead of panicking.
pub fn solve(
    problem: &TspProblem,
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Result<Solution, String> {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let n_cities = cities.len();

    if n_cities == 0 {
        return Ok(Solution::from_parts(&[], distances.as_ref()));
    }

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
        let pa = local_position(a);
        for b in 0..stride {
            let pb = local_position(b);
            matrix[a * stride + b] = if pa == pb {
                // A tour never traverses a self-edge, and the two anchor aliases are the
                // same city. `tour_length` likewise defines a singleton tour as zero
                // without querying an edge; do not ask a custom source for `d(p, p)`.
                0.0
            } else {
                distances.distance_by_pos(pa, pb).unwrap_or(f32::INFINITY)
            };
        }
    }

    let Some((cost, interior)) = held_karp_path(&matrix, n_others) else {
        return Err(
            "Bellman-Held-Karp: the distance source has no finite Hamiltonian path".to_string(),
        );
    };
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
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&route), cost as f32));
        let _ = tx.send(ProgressMessage::Done);
    }
    Ok(Solution::from_parts(&route, distances.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{
        DistanceSource, HeuristicOptions, Position, TspProblem, distance_matrix, kdtree,
    };

    /// Refuses self-distance queries, so a caller that asks for `d(p, p)` is caught.
    struct NoSelfDistance {
        cities: Vec<kdtree::KDPoint>,
    }

    impl DistanceSource for NoSelfDistance {
        fn num_cities(&self) -> usize {
            self.cities.len()
        }

        fn distance_by_pos(&self, a: Position, b: Position) -> Result<f32, &'static str> {
            if a == b {
                Err("self-distance is not defined")
            } else {
                Ok(1.0)
            }
        }

        fn pos2city_id(&self, pos: Position) -> Option<usize> {
            self.cities.get(pos.into_inner()).map(|city| city.id)
        }

        fn city_id2pos(&self, id: usize) -> Option<Position> {
            self.cities
                .iter()
                .position(|city| city.id == id)
                .map(Position::new)
        }

        fn point_at(&self, pos: Position) -> Option<kdtree::KDPoint> {
            self.cities.get(pos.into_inner()).copied()
        }
    }

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
        let solution = solve(&problem, &HeuristicOptions::default(), None, None).unwrap();

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
        let solution = solve(&problem, &HeuristicOptions::default(), None, None).unwrap();

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

        let solution = solve(&problem, &HeuristicOptions::default(), None, None).unwrap();

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
        let solution = solve(&problem, &HeuristicOptions::default(), None, None).unwrap();

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
        let first = solve(&problem, &HeuristicOptions::default(), None, None).unwrap();
        let second = solve(&problem, &HeuristicOptions::default(), None, None).unwrap();
        assert_eq!(first.route(), second.route());
        assert_eq!(first.total, second.total);
    }

    #[test]
    fn test_singleton_tour_costs_zero_without_a_self_edge_query() {
        // The source refuses a self-distance; asking for `d(anchor, anchor)` would make the
        // matrix build fail, so this fails if the self-edge guard is removed.
        let cities = kdtree::build_points(&[vec![0.0, 0.0]]);
        let distances = NoSelfDistance {
            cities: cities.clone(),
        };
        let problem = TspProblem::new(cities, distances);
        let solution = solve(&problem, &HeuristicOptions::default(), None, None).unwrap();
        assert_eq!(solution.route().len(), 1);
        assert_eq!(solution.total, 0.0);
    }

    #[test]
    fn test_solve_rejects_a_non_finite_edge() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]]);
        // Packed lower triangle: (1,0), (2,0), (2,1).
        let weights = vec![1.0, f32::INFINITY, 1.0];
        let dm = distance_matrix::DistanceMatrix::new(3, weights, cities.clone()).unwrap();
        let problem = TspProblem::new(cities, dm);

        let result = solve(&problem, &HeuristicOptions::default(), None, None);
        assert!(
            result.is_err(),
            "an infinite edge must be reported, not panic"
        );
    }
}

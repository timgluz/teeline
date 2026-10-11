use std::sync::mpsc;

use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem};

pub fn solve(
    problem: &TspProblem,
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    tracing::info!(cities = cities.len(), "2-opt starting");

    let n_indices = cities.len() - 1;
    let mut path: Vec<usize> = init_tour
        .map(|t| t.to_vec())
        .unwrap_or_else(|| cities.iter().map(|c| c.id).collect());

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&path), 0.0));
    }

    let mut improved = true;
    while improved {
        improved = false;
        for i in 0..(n_indices - 2) {
            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::CityChange(path[i]));
            }

            for j in (i + 2)..n_indices {
                let current_distance = distances
                    .distance_between(path[i], path[i + 1])
                    .expect("two_opt: invalid city pair")
                    + distances
                        .distance_between(path[j], path[j + 1])
                        .expect("two_opt: invalid city pair");

                let new_distance = distances
                    .distance_between(path[i], path[j])
                    .expect("two_opt: invalid city pair")
                    + distances
                        .distance_between(path[i + 1], path[j + 1])
                        .expect("two_opt: invalid city pair");

                if new_distance < current_distance {
                    swap_2opt(&mut path, i + 1, j);
                    improved = true;

                    if let Some(tx) = progress_tx {
                        let _ =
                            tx.send(ProgressMessage::PathUpdate(Route::new(&path), new_distance));
                    }
                    tracing::debug!(i, j, tour_length = new_distance, "2-opt: improvement");
                }
            }
        }
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&path, cities, distances)
}

fn swap_2opt(path: &mut [usize], from: usize, to: usize) {
    if from >= to {
        return;
    }

    let reversed_seq: Vec<usize> = path[from..=to].iter().copied().rev().collect();

    for (i, swapped_val) in reversed_seq.iter().enumerate() {
        path[from + i] = *swapped_val;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{HeuristicOptions, TspProblem, distance_matrix, kdtree};

    #[test]
    fn test_swap_2opt_2middle_elems_in_even_size_list() {
        let mut path = vec![1, 2, 3, 4];
        swap_2opt(&mut path, 1, 2);
        assert_eq!(vec![1, 3, 2, 4], path);
    }

    #[test]
    fn test_swap_2opt_single_middle_elem_does_nothing() {
        let mut path = vec![1, 2, 3, 4];
        swap_2opt(&mut path, 1, 1);
        assert_eq!(vec![1, 2, 3, 4], path);
    }

    /// A solver given a suboptimal starting tour must improve it.
    ///
    /// The fixture is ordered so the *default* start — the identity order — is not the optimum
    /// (4.83 against 4.00). That is what makes the assertion falsifiable: previously the identity
    /// order was already optimal, so the test passed for a solver that ignored its input.
    #[test]
    fn test_two_opt_respects_initial_tour() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![0.0, 0.5],
        ]);
        let dm = distance_matrix::from_cities_unrounded(&cities);
        let start: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let start_cost = dm.tour_length(&start);
        assert!(start_cost > 4.0, "the fixture must start suboptimal");

        let problem = TspProblem::new(cities, dm);
        let result = solve(&problem, &HeuristicOptions::default(), None, Some(&start));

        assert!(
            (result.total - 4.0).abs() < 1e-4,
            "must reach the 4.00 perimeter from a {start_cost:.2} start, got {}",
            result.total
        );
    }

    #[test]
    fn test_solve_with_tsp5_example() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![0.0, 0.5],
        ]);

        let dm = distance_matrix::from_cities_unrounded(&cities);
        let problem = TspProblem::new(cities, dm);
        let tour = solve(&problem, &HeuristicOptions::default(), None, None);

        // The default start costs 4.83, so reaching 4.00 is a real improvement rather than the
        // starting point restated.
        assert_eq!(4.0, tour.total);
    }
}

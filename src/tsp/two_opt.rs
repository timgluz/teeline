use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], distances: &DistanceMatrix, options: &SolverOptions) -> Solution {
    tracing::info!(cities = cities.len(), "2-opt starting");

    let n_indices = cities.len() - 1;
    let mut path: Vec<usize> = cities.iter().map(|c| c.id).collect();

    options.send_progress(ProgressMessage::PathUpdate(Route::new(&path), 0.0));

    let mut improved = true;
    while improved {
        improved = false;
        for i in 0..(n_indices - 2) {
            options.send_progress(ProgressMessage::CityChange(path[i]));

            for j in (i + 2)..n_indices {
                let current_distance =
                    distances.distance_between(path[i], path[i + 1]).expect("two_opt: invalid city pair")
                    + distances.distance_between(path[j], path[j + 1]).expect("two_opt: invalid city pair");

                let new_distance =
                    distances.distance_between(path[i], path[j]).expect("two_opt: invalid city pair")
                    + distances.distance_between(path[i + 1], path[j + 1]).expect("two_opt: invalid city pair");

                if new_distance < current_distance {
                    swap_2opt(&mut path, i + 1, j);
                    improved = true;

                    options.send_progress(ProgressMessage::PathUpdate(Route::new(&path), new_distance));
                    tracing::debug!(i, j, tour_length = new_distance, "2-opt: improvement");
                }
            }
        }
    }

    options.send_progress(ProgressMessage::Done);
    Solution::new(&path, cities, distances)
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
    use crate::tsp::{distance_matrix, kdtree};

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

    #[test]
    fn test_solve_with_tsp5_example() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);

        let dm = distance_matrix::from_cities(&cities);
        let default_opts = SolverOptions::default();
        let tour = solve(&cities, &dm, &default_opts);
        assert_eq!(4.0, tour.total);
        assert_eq!(&[0, 1, 2, 3, 4], tour.route());
    }
}

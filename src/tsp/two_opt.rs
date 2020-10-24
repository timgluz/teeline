use super::kdtree::KDPoint;
use super::progress::{send_progress, ProgressMessage};
use super::route::Route;
use super::{city_table_from_vec, Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    let n_indices = cities.len() - 1;
    let cities_table = city_table_from_vec(cities);
    let mut path: Vec<usize> = cities.iter().map(|c| c.id).collect();

    send_progress(ProgressMessage::PathUpdate(Route::new(&path), 0.0));

    let mut improved = true;
    while improved {
        improved = false;
        for i in 0..(n_indices - 2) {
            send_progress(ProgressMessage::CityChange(path[i]));

            for j in (i + 2)..n_indices {
                let current_distance = cities_table[&path[i]].distance(&cities_table[&path[i + 1]])
                    + cities_table[&path[j]].distance(&cities_table[&path[j + 1]]);

                let new_distance = cities_table[&path[i]].distance(&cities_table[&path[j]])
                    + cities_table[&path[i + 1]].distance(&cities_table[&path[j + 1]]);

                if new_distance < current_distance {
                    swap_2opt(&mut path, i + 1, j);
                    improved = true;

                    send_progress(ProgressMessage::PathUpdate(Route::new(&path), new_distance));

                    if options.verbose {
                        println!(
                            "2OPT: cities(i: {:?}, j: {:?}) new best {:?}",
                            i, j, new_distance
                        );
                    }
                }
            }
        }
    }

    send_progress(ProgressMessage::Done);
    Solution::new(&path, cities)
}

fn swap_2opt(path: &mut Vec<usize>, from: usize, to: usize) {
    if from >= to {
        return; // ignore if from to are equal or wrong order
    }

    let reversed_seq: Vec<usize> = path[from..=to].iter().map(|x| x.clone()).rev().collect();

    for (i, swapped_val) in reversed_seq.iter().enumerate() {
        path[from + i] = swapped_val.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::kdtree;

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

        let default_opts = SolverOptions::default();
        let tour = solve(&cities, &default_opts);
        assert_eq!(4.0, tour.total);
        assert_eq!(&[0, 1, 2, 3, 4], tour.path());
    }
}

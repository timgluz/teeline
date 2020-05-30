use super::kdtree::KDPoint;
use super::tour::Tour;

pub fn solve(cities: &[KDPoint]) -> Tour {
    let n_indices = cities.len() - 1;
    let mut route: Vec<usize> = cities.iter().map(|c| c.id).collect();

    let mut improved = true;
    while improved {
        improved = false;
        for i in 0..(n_indices - 2) {
            for j in (i + 2)..n_indices {
                let current_distance = cities[route[i]].distance(&cities[route[i + 1]])
                    + cities[route[j]].distance(&cities[route[j + 1]]);

                let new_distance = cities[route[i]].distance(&cities[route[j]])
                    + cities[route[i + 1]].distance(&cities[route[j + 1]]);

                if new_distance < current_distance {
                    swap_2opt(&mut route, i + 1, j);
                    improved = true;
                }
            }
        }
    }

    Tour::new(&route, cities)
}

fn swap_2opt(route: &mut Vec<usize>, from: usize, to: usize) {
    if from >= to {
        return; // ignore if from to are equal or wrong order
    }

    let reversed_seq: Vec<usize> = route[from..=to].iter().map(|x| x.clone()).rev().collect();

    for (i, swapped_val) in reversed_seq.iter().enumerate() {
        route[from + i] = swapped_val.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::kdtree;

    #[test]
    fn test_swap_2opt_2middle_elems_in_even_size_list() {
        let mut route = vec![1, 2, 3, 4];

        swap_2opt(&mut route, 1, 2);

        assert_eq!(vec![1, 3, 2, 4], route);
    }

    #[test]
    fn test_swap_2opt_single_middle_elem_does_nothing() {
        let mut route = vec![1, 2, 3, 4];

        swap_2opt(&mut route, 1, 1);

        assert_eq!(vec![1, 2, 3, 4], route);
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

        let tour = solve(&cities);
        assert_eq!(4.0, tour.total);
        assert_eq!(&[0, 1, 2, 3, 4], tour.route());
    }
}

use std::sync::mpsc;

use super::DistanceSource;
use super::candidate_list;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem};

pub fn solve(
    problem: &TspProblem,
    opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    tracing::info!(cities = cities.len(), "2-opt starting");

    let mut path: Vec<usize> = init_tour
        .map(|t| t.to_vec())
        .unwrap_or_else(|| cities.iter().map(|c| c.id).collect());

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&path), 0.0));
    }

    if cities.len() < 4 {
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::Done);
        }
        return Solution::from_parts(&path, cities, distances.as_ref());
    }

    // A candidate list is a pruning heuristic: every move is still scored with the exact
    // distance source. EXPLICIT inputs carry placeholder coordinates, so they keep the full
    // scan instead of ranking meaningless neighbours.
    let candidates = if distances.has_coordinates() {
        Some(candidate_list::build(
            cities,
            opts.n_nearest.max(candidate_list::MIN_CANDIDATE_K),
        ))
    } else {
        None
    };

    let n_indices = cities.len() - 1;
    let mut improved = true;
    while improved {
        improved = false;
        // Best-improvement pass: the route is unchanged while candidates are evaluated, so
        // the position map stays valid for the whole pass.
        let pos = candidates
            .as_ref()
            .map(|_| candidate_list::positions(&path));
        let mut best: Option<(f32, usize, usize)> = None;

        for i in 0..(n_indices - 2) {
            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::CityChange(path[i]));
            }

            if let (Some(cands), Some(pos)) = (&candidates, &pos) {
                let js = candidate_list::candidate_positions(cands, pos, path[i], path[i + 1]);
                for j in js {
                    if j <= i + 1 || j >= n_indices {
                        continue;
                    }
                    consider_move(&path, distances.as_ref(), i, j, &mut best);
                }
            } else {
                for j in (i + 2)..n_indices {
                    consider_move(&path, distances.as_ref(), i, j, &mut best);
                }
            }
        }

        if let Some((_, i, j)) = best {
            swap_2opt(&mut path, i + 1, j);
            improved = true;
            let new_len = distances.tour_length(&path);
            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&path), new_len));
            }
            tracing::debug!(i, j, tour_length = new_len, "2-opt: improvement");
        }
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&path, cities, distances.as_ref())
}

/// Evaluate the 2-opt move between positions `i` and `j`, retaining the best negative delta.
fn consider_move(
    path: &[usize],
    distances: &dyn DistanceSource,
    i: usize,
    j: usize,
    best: &mut Option<(f32, usize, usize)>,
) {
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

    let delta = new_distance - current_distance;
    if delta >= -1e-6 {
        return;
    }

    let better = match best {
        None => true,
        Some((best_delta, best_i, best_j)) => delta
            .total_cmp(best_delta)
            .then_with(|| (i, j).cmp(&(*best_i, *best_j)))
            .is_lt(),
    };
    if better {
        *best = Some((delta, i, j));
    }
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

    #[test]
    fn test_solve_handles_small_inputs() {
        for n in 2..=3 {
            let cities =
                kdtree::build_points(&(0..n).map(|i| vec![i as f32, 0.0]).collect::<Vec<_>>());
            let dm = distance_matrix::from_cities_unrounded(&cities);
            let problem = TspProblem::new(cities, dm);
            let tour = solve(&problem, &HeuristicOptions::default(), None, None);
            assert_eq!(tour.route().len(), n);
        }
    }
}

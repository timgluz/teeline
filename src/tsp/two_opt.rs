use std::sync::mpsc;

use super::DistanceSource;
use super::candidate_list::{self, CandidateList};
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution, TspProblem, kdtree::KDPoint};

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
    // scan instead of ranking meaningless neighbours. When k covers every city there is
    // nothing to prune, so the full scan is also cheaper than building the table.
    let k = opts.n_nearest.max(candidate_list::MIN_CANDIDATE_K);
    let candidates = candidates_for(distances.as_ref(), cities, k);

    let n_indices = cities.len() - 1;
    match candidates {
        Some(candidates) => best_improvement_passes(
            &mut path,
            distances.as_ref(),
            &candidates,
            n_indices,
            progress_tx,
        ),
        None => first_improvement_passes(&mut path, distances.as_ref(), n_indices, progress_tx),
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&path, cities, distances.as_ref())
}

/// A candidate list is a pruning heuristic: every move is still scored with the exact distance
/// source. EXPLICIT inputs keep the full scan instead of building a list from a layout, and when
/// `k` covers every city there is nothing to prune.
fn candidates_for(
    distances: &dyn DistanceSource,
    cities: &[KDPoint],
    k: usize,
) -> Option<CandidateList> {
    if distances.is_coordinate_metric() && k < cities.len().saturating_sub(1) {
        CandidateList::build(cities, k)
    } else {
        None
    }
}

/// Best-improvement pass over candidate endpoints.
///
/// The route is unchanged while candidates are evaluated, so one position map stays valid for
/// the whole pass. A candidate list keeps each pass at O(n·k); the alternative, first
/// improvement, would need to repair the map after every accepted swap.
fn best_improvement_passes(
    path: &mut [usize],
    distances: &dyn DistanceSource,
    candidates: &CandidateList,
    n_indices: usize,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
) {
    let mut improved = true;
    let mut js = Vec::new();
    while improved {
        improved = false;
        let route_positions = candidates.route_positions(path);
        let mut best: Option<(f32, usize, usize)> = None;

        for i in 0..(n_indices - 2) {
            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::CityChange(path[i]));
            }

            candidate_list::candidate_positions_into(
                candidates,
                &route_positions,
                path.len(),
                path[i],
                path[i + 1],
                &mut js,
            );
            for &j in &js {
                if j <= i + 1 || j >= n_indices {
                    continue;
                }
                consider_move(path, distances, i, j, &mut best);
            }
        }

        if let Some((_, i, j)) = best {
            swap_2opt(path, i + 1, j);
            improved = true;
            report_improvement(path, distances, i, j, progress_tx);
        }
    }
}

/// First-improvement full scan, used when no candidate list is available.
///
/// Applying a swap immediately keeps the original 2-opt shape: multiple improvements can land
/// in one pass, which matters on EXPLICIT instances where every move is a full scan anyway.
fn first_improvement_passes(
    path: &mut [usize],
    distances: &dyn DistanceSource,
    n_indices: usize,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
) {
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
                    swap_2opt(path, i + 1, j);
                    improved = true;
                    report_improvement(path, distances, i, j, progress_tx);
                }
            }
        }
    }
}

fn report_improvement(
    path: &[usize],
    distances: &dyn DistanceSource,
    i: usize,
    j: usize,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
) {
    if progress_tx.is_some() || tracing::enabled!(tracing::Level::DEBUG) {
        let new_len = distances.tour_length(path);
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::PathUpdate(Route::new(path), new_len));
        }
        tracing::debug!(i, j, tour_length = new_len, "2-opt: improvement");
    }
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
    path[from..=to].reverse();
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

    #[test]
    fn explicit_input_skips_the_candidate_list() {
        use crate::tsp::CityTable;

        let n = 6;
        let cities: Vec<kdtree::KDPoint> = (0..n)
            .map(|i| kdtree::KDPoint::new_with_id(i + 1, &[i as f32, 0.0]))
            .collect();
        let table: CityTable = cities
            .iter()
            .enumerate()
            .map(|(position, city)| (position, *city))
            .collect();
        let matrix =
            distance_matrix::DistanceMatrix::new(n, vec![1.0; n * (n - 1) / 2], table).unwrap();

        assert!(candidates_for(&matrix, &cities, 2).is_none());
    }
}

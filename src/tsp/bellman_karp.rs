/// Bellman-Help-Karp
///
/// exact solver using Dynamic Programming
/// source:
///
/// Draft of Chapter 7 (Dynamic Programming) for the planned book A Practical Guide to Discrete
/// Optimization, with D. Applegate, S. Dash, D. S. Johnson. December 29, 2014.
/// https://www.math.uwaterloo.ca/~bico/papers/papers.html
///
///
///
use std::sync::mpsc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{AppOptions, Solution};

type FlagSet = u64;
type DPTable = Vec<Vec<f32>>;

const UNKNOWN_DISTANCE: f32 = f32::MAX;

pub fn solve(
    cities: &[KDPoint],
    distances: &DistanceMatrix,
    opts: &AppOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _initial_tour: Option<&[usize]>,
) -> Solution {
    let h = opts.heuristic.as_ref().cloned().unwrap_or_default();
    let n_cities = cities.len();
    let n_others = n_cities - 1;
    let n_powersets = 1 << n_others;

    tracing::info!(n_cities, n_subsets = n_powersets, "BHK starting");

    let dists = distances;
    let mut opt = vec![vec![UNKNOWN_DISTANCE; n_powersets]; n_others];

    tracing::info!("BHK: initialising DP table");
    let last_pos = n_others;
    for i in 0..n_others {
        opt[i][1 << i] = dists
            .distance_by_pos(i, last_pos)
            .unwrap_or(UNKNOWN_DISTANCE);

        if let Some(city_id) = dists.pos2city_id(&i) {
            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::CityChange(city_id));
            }
        }
    }

    let selected_set = (1 << n_others) - 1;
    for city_pos in 0..n_others {
        solve_bhk(&mut opt, dists, selected_set, city_pos);
    }

    tracing::info!("BHK: DP complete");
    if h.verbose {
        show_table(&opt);
    }

    let optimal_distance = (0..n_others)
        .filter_map(|i| {
            let return_dist = dists.distance_by_pos(i, last_pos).ok()?;
            let sub_cost = opt[i][selected_set as usize];
            if sub_cost < UNKNOWN_DISTANCE && return_dist < UNKNOWN_DISTANCE {
                Some(sub_cost + return_dist)
            } else {
                None
            }
        })
        .fold(f32::MAX, f32::min);

    tracing::info!(optimal_distance, "BHK: optimal tour found");
    let route_vec = read_optimal_route(&opt, dists, n_cities, optimal_distance);

    let route = Route::new(route_vec.as_ref());
    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(route, 0.0));
        let _ = tx.send(ProgressMessage::Done);
    }

    Solution::new(&route_vec, cities, distances)
}

fn solve_bhk(
    opt: &mut DPTable,
    dm: &DistanceMatrix,
    selected_set: FlagSet,
    city_pos: usize,
) -> f32 {
    let mut best_val = f32::MAX;

    let selected_pos = selected_set as usize;
    if opt[city_pos][selected_pos] < UNKNOWN_DISTANCE {
        return opt[city_pos][selected_pos];
    }

    let rest_selected = selected_set & !(1 << city_pos);
    let n_other = opt.len();
    for i in 0..n_other {
        if (rest_selected & (1 << i)) == 0 {
            continue;
        }

        let step_dist = dm
            .distance_by_pos(i, city_pos)
            .expect("solve_bhk tried to access non-existent cities");
        let sub_val = solve_bhk(opt, dm, rest_selected, i) + step_dist;
        if sub_val < best_val {
            best_val = sub_val;
        }
    }

    opt[city_pos][selected_pos] = best_val;
    opt[city_pos][selected_pos]
}

fn read_optimal_route(opt: &DPTable, dm: &DistanceMatrix, n: usize, best_val: f32) -> Vec<usize> {
    let mut route_ids = vec![0; n];
    route_ids[0] = n - 1;

    let mut unread_set = (1 << (n - 1)) - 1;
    let mut left_dist = best_val;
    for i in 1..n {
        if left_dist <= 0.0 {
            break;
        }

        for (j, row) in opt[0..(n - 1)].iter().enumerate() {
            let step_dist = dm
                .distance_by_pos(j, route_ids[i - 1])
                .expect("step_dist points are out range");

            let cur_dist = row[unread_set] + step_dist;
            let is_unprocessed = (unread_set & (1 << j)) > 0;
            if is_unprocessed && approx(left_dist, cur_dist) {
                left_dist -= step_dist;
                route_ids[i] = j;

                unread_set &= !(1 << j);
                break;
            }
        }
    }

    let route: Vec<usize> = route_ids
        .iter()
        .map(|pos| dm.pos2city_id(pos).unwrap_or(0))
        .collect();

    route
}

fn approx(x1: f32, x2: f32) -> bool {
    (x1 - x2).abs() < f32::EPSILON
}

fn show_table(opt: &DPTable) {
    println!("=============================================");
    println!("Dynamic programming table");

    for row in opt.iter() {
        print!("| ");

        for val in row.iter() {
            let fval = if approx(*val, UNKNOWN_DISTANCE) {
                " - ".to_string()
            } else {
                format!("{:.2}", val)
            };

            print!("{:^10} |", fval);
        }

        println!(" |");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, AppOptions};

    fn tsp_5_1_cities() -> Vec<kdtree::KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ])
    }

    #[test]
    fn test_solve_returns_all_cities() {
        let cities = tsp_5_1_cities();
        let dm = distance_matrix::from_cities(&cities);
        let solution = solve(&cities, &dm, &AppOptions::default(), None, None);

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
        let cities = tsp_5_1_cities();
        let dm = distance_matrix::from_cities(&cities);
        let solution = solve(&cities, &dm, &AppOptions::default(), None, None);

        assert!(
            (solution.total - 4.0).abs() < 1e-3,
            "expected tour length ~4.0, got {}",
            solution.total
        );
    }

    #[test]
    fn test_solve_with_3_cities() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![3.0, 0.0],
            vec![0.0, 4.0],
        ]);
        let dm = distance_matrix::from_cities(&cities);
        let solution = solve(&cities, &dm, &AppOptions::default(), None, None);

        let mut visited: Vec<usize> = solution.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2]);

        assert!((solution.total - 12.0).abs() < 1e-3, "got {}", solution.total);
    }
}

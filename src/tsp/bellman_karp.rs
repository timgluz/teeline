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
use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::{send_progress, ProgressMessage};
use super::route::Route;
use super::{Solution, SolverOptions};

// 0-1 Set, where 1 means that city N is collected
type FlagSet = u64;
type DPTable = Vec<Vec<f32>>;

const UNKNOWN_DISTANCE: f32 = f32::MAX;

pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    let n_cities = cities.len();
    let n_others = n_cities - 1; // we start from last city
    let n_powersets = 1 << n_others;

    let dists = DistanceMatrix::from_cities(cities).unwrap();
    let mut opt = vec![vec![UNKNOWN_DISTANCE; n_powersets]; n_others];

    if options.verbose {
        println!("BHK: initializing the table with subresults");
    }
    // inialize tables first row with distance from first cities to other cities
    let last_pos = n_others;
    for i in 0..n_others {
        opt[i][1 << i] = dists
            .distance_by_pos(i, last_pos)
            .unwrap_or(UNKNOWN_DISTANCE);

        if let Some(city_id) = dists.pos2city_id(&i) {
            send_progress(ProgressMessage::CityChange(city_id));
        }
    }

    let selected_set = (1 << n_others) - 1;
    for city_pos in 0..n_others {
        solve_bhk(&mut opt, &dists, selected_set, city_pos);
    }

    if options.verbose {
        println!("BHK: done with calculations, preparing the result");
        show_table(&opt);
    }

    // Find the actual optimal tour distance: min over all ending cities of
    // (cost to visit all n_others cities ending at i) + (return edge to start city)
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

    let route_vec = read_optimal_route(&opt, &dists, n_cities, optimal_distance);

    // send final route to the visualizer
    let route = Route::new(route_vec.as_ref());
    send_progress(ProgressMessage::PathUpdate(route, 0.0));
    send_progress(ProgressMessage::Done);

    Solution::new(&route_vec, cities)
}

fn solve_bhk(
    opt: &mut DPTable,
    dm: &DistanceMatrix,
    selected_set: FlagSet,
    city_pos: usize,
) -> f32 {
    let mut best_val = f32::MAX;

    // distance is already calculated
    let selected_pos = selected_set as usize;
    if opt[city_pos][selected_pos] < UNKNOWN_DISTANCE {
        return opt[city_pos][selected_pos];
    }

    // rest_selected R = S \ t , all other than city_id
    let rest_selected = selected_set & !(1 << city_pos);
    let n_other = opt.len() - 1; // opt has n_city rows
    for i in 0..n_other {
        // if city i is not in rest_selected
        if (rest_selected & (1 << i)) == 0 {
            continue;
        }

        // solve sub problem
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
    use crate::tsp::kdtree;

    fn default_options() -> SolverOptions {
        SolverOptions::default()
    }

    // tsp_5_1 shape: five cities forming a rectangle with a midpoint on one side
    // optimal tour visits them in order and has total length 4.0
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
        let solution = solve(&cities, &default_options());

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
        let solution = solve(&cities, &default_options());

        // optimal tour: 0→1→2→3→4→0 = 0.5 + 0.5 + 1.0 + 1.0 + 1.0 = 4.0
        assert!(
            (solution.total - 4.0).abs() < 1e-3,
            "expected tour length ~4.0, got {}",
            solution.total
        );
    }

    #[test]
    fn test_solve_with_3_cities() {
        // simple triangle: right angle at origin
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![3.0, 0.0],
            vec![0.0, 4.0],
        ]);
        let solution = solve(&cities, &default_options());

        let mut visited: Vec<usize> = solution.route().to_vec();
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2]);

        // tour is 3 + 4 + 5 = 12.0
        assert!((solution.total - 12.0).abs() < 1e-3, "got {}", solution.total);
    }
}

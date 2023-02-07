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

/// TODO: fix bug with duplicated city.1, and missing city.4
/// replication: use ./data/discopt/tsp_5_1.tsp;
pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    let n_cities = cities.len();
    let n_others = n_cities - 1; // we start from last city
    let n_powersets = 1 << n_others;

    let dists = DistanceMatrix::from_cities(cities).unwrap();
    let mut opt = vec![vec![UNKNOWN_DISTANCE; n_powersets]; n_others];

    if options.verbose == true {
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
    let mut best_val = f32::MAX;
    for city_pos in 0..n_others {
        let sub_val = solve_bhk(&mut opt, &dists, selected_set, city_pos)
            + dists
                .distance_by_pos(city_pos, last_pos)
                .unwrap_or(UNKNOWN_DISTANCE);
        if sub_val < best_val {
            best_val = sub_val;
        }
    }

    if options.verbose == true {
        println!("BHK: done with calculations, preparing the result");
        show_table(&opt);
    }

    let route_vec = read_optimal_route(&opt, &dists, n_cities, best_val);

    // send final route to the visualizer
    let route = Route::new(route_vec.as_ref());
    send_progress(ProgressMessage::PathUpdate(route, 0.0));
    send_progress(ProgressMessage::Done);

    let tour = Solution::new(&route_vec, cities);

    tour
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
    let n_other = opt.len(); // opt has n_city rows
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

        for j in 0..(n - 1) {
            let step_dist = dm
                .distance_by_pos(j, route_ids[i - 1])
                .expect("step_dist points are out range");

            let cur_dist = opt[j][unread_set] + step_dist;
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
        assert_eq!(&[4, 0, 1, 2, 3], tour.route());
    }
}

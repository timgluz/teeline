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
use super::progress::{ProgressMessage, PublisherFn};
use super::route::Route;
use super::{Solution, SolverOptions};

// 0-1 Set, where 1 means that city N is collected
type FlagSet = u64;
type DPTable = Vec<Vec<f32>>;

pub fn solve(cities: &[KDPoint], options: &SolverOptions, progressfn: PublisherFn) -> Solution {
    let n_cities = cities.len();
    let n_others = n_cities - 1; // we start from last city
    let n_powersets = 1 << n_others;

    let dists = DistanceMatrix::from_cities(cities).unwrap();
    let mut opt = vec![vec![0.0; n_powersets]; n_others];

    if options.verbose == true {
        println!("BHK: initializing the table with subresults");
    }
    // inialize tables first row with distance from first cities to other cities
    let last_pos = n_others;
    for i in 0..n_others {
        opt[i][1 << i] = dists.distance_by_pos(i, last_pos).unwrap_or(0.0);

        progressfn(ProgressMessage::CityChange(i));
    }

    let selected_set = (1 << n_others) - 1;
    for city_pos in 0..n_others {
        solve_bhk(&mut opt, &dists, selected_set, city_pos);
    }

    if options.verbose == true {
        println!("BHK: done with calculations, preparing the result");
    }

    let route_vec = read_optimal_route(&opt, &dists, n_cities, f32::MAX);

    // send final route to the visualizer
    let route = Route::new(route_vec.as_ref());
    progressfn(ProgressMessage::PathUpdate(route, 0.0));

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
    if opt[city_pos][selected_pos] > 0.0 {
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

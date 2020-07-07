use super::distance_matrix::DistanceMatrix;
/// Bellman-Help-Karp
///
/// exact solver using Dynamic Programming
/// source:
///
/// Draft of Chapter 7 (Dynamic Programming) for the planned book A Practical Guide to Discrete
/// Optimization, with D. Applegate, S. Dash, D. S. Johnson. December 29, 2014.
/// https://www.math.uwaterloo.ca/~bico/papers/papers.html
///
use super::kdtree::KDPoint;
use super::tour::Tour;
use super::SolverOptions;

// 0-1 Set, where 1 means that city N is collected
type FlagSet = u64;
type DPTable = Vec<Vec<f32>>;

pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Tour {
    let n_cities = cities.len();
    let n_others = n_cities - 1;
    let n_powersets = 1 << n_others;

    let dists = DistanceMatrix::from_cities(cities).unwrap();
    let mut opt = vec![vec![0.0; n_powersets]; n_others];

    if options.verbose == true {
        println!("BHK: initializing the table with subresults");
    }
    // inialize tables first row with distance from city.o to city.i
    for i in 0..n_others {
        opt[i][1 << i] = dists.distance_between(i, n_others).unwrap_or(0.0);
    }

    let selected_set = (1 << n_others) - 1;
    for city_id in 0..n_others {
        solve_bhk(&mut opt, &dists, selected_set, city_id);
    }

    if options.verbose == true {
        println!("BHK: done with calculations, preparing the result");
    }

    let route_vec = read_optimal_route(&opt, &dists, n_cities, f32::MAX);
    let tour = Tour::new(&route_vec, cities);

    tour
}

fn solve_bhk(opt: &mut DPTable, dm: &DistanceMatrix, selected_set: FlagSet, city_id: usize) -> f32 {
    let mut best_val = f32::MAX;

    // distance is already calculated
    let selected_id = selected_set as usize;
    if opt[city_id][selected_id] > 0.0 {
        return opt[city_id][selected_id];
    }

    // rest_selected R = S \ t , all other than city_id
    let rest_selected = selected_set & !(1 << city_id);
    let n_other = opt.len() - 1; // opt has n_city rows
    for i in 0..n_other {
        // if city i is not in rest_selected
        if (rest_selected & (1 << i)) == 0 {
            continue;
        }

        // solve sub problem
        let step_dist = dm
            .distance_between(i, city_id)
            .expect("solve_bhk tried to access non-existent cities");
        let sub_val = solve_bhk(opt, dm, rest_selected, i) + step_dist;
        if sub_val < best_val {
            best_val = sub_val;
        }
    }

    opt[city_id][selected_id] = best_val;
    opt[city_id][selected_id]
}

fn read_optimal_route(opt: &DPTable, dm: &DistanceMatrix, n: usize, best_val: f32) -> Vec<usize> {
    let mut route = vec![0; n];
    route[0] = n - 1;

    let mut unread_set = (1 << (n - 1)) - 1;
    let mut left_dist = best_val;
    for i in 1..n {
        if left_dist <= 0.0 {
            break;
        }

        for j in 0..(n - 1) {
            let step_dist = dm
                .distance_between(j, route[i - 1])
                .expect("step_dist points are out range");

            let cur_dist = opt[j][unread_set] + step_dist;
            let is_unprocessed = (unread_set & (1 << j)) > 0;
            if is_unprocessed && approx(left_dist, cur_dist) {
                left_dist -= step_dist;
                route[i] = j;

                unread_set &= !(1 << j);
                break;
            }
        }
    }
    route
}

fn approx(x1: f32, x2: f32) -> bool {
    (x1 - x2).abs() < f32::EPSILON
}

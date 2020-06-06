extern crate rand;

use std::fmt::Debug;
use std::str::FromStr;

use teeline::tsp::{kdtree, nearest_neighbor, simulated_annealing, stochastic_hill, tour, two_opt};

fn main() {
    let n_points = read_value::<usize>();
    let cities = read_cities(n_points);

    //TODO: read params from commandline
    //let tour = nearest_neighbor::solve(&cities);
    //let tour = two_opt::solve(&cities);
    //let tour = stochastic_hill::solve(&cities);
    let tour = simulated_annealing::solve(&cities);

    print_solution(&tour, false);
}

fn print_solution(tour: &tour::Tour, is_optimized: bool) {
    let optimization_flag = if is_optimized { 1 } else { 0 };

    println!("{:.5} {}", tour.total, optimization_flag);
    for city_id in tour.route().iter() {
        print!("{} ", city_id);
    }

    print!("\n");
}

// k-opt
fn read_cities(n: usize) -> Vec<kdtree::KDPoint> {
    let mut rows = kdtree::PointMatrix::with_capacity(n);

    for _ in 0..n {
        rows.push(read_vector::<f32>());
    }

    let cities = kdtree::build_points(&rows);

    cities
}

fn read_value<T>() -> T
where
    T: FromStr,
    T::Err: Debug,
{
    let line = read_string();

    let res: T = line
        .trim()
        .parse::<T>()
        .expect("Failed to parse valur from stdin");

    res
}

fn read_vector<T>() -> Vec<T>
where
    T: FromStr,
    T::Err: Debug,
{
    let line = read_string();

    let res: Vec<T> = line
        .trim()
        .split_whitespace()
        .map(|token| token.parse::<T>().expect("Failed to parse vector row"))
        .collect();

    res
}

fn read_string() -> String {
    let mut buf = String::new();

    std::io::stdin()
        .read_line(&mut buf)
        .expect("Failed to read string from stding");

    buf
}

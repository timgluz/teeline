extern crate clap;
extern crate rand;

use std::fmt::Debug;
use std::str::FromStr;

use teeline::tsp::{self, kdtree, tour, Solvers};

fn main() {
    let n_points = read_value::<usize>();
    let cities = read_cities(n_points);

    let tour = solve(Solvers::TabuSearch, &cities);

    print_solution(&tour, false);
}

/// solves tsp for given cities by using solver
fn solve(algorithm: Solvers, cities: &[kdtree::KDPoint]) -> tour::Tour {
    match algorithm {
        Solvers::NearestNeighbor => tsp::nearest_neighbor::solve(cities),
        Solvers::TwoOpt => tsp::two_opt::solve(cities),
        Solvers::StochasticHill => tsp::stochastic_hill::solve(cities),
        Solvers::SimulatedAnnealing => tsp::simulated_annealing::solve(cities),
        Solvers::TabuSearch => tsp::tabu_search::solve(cities),
    }
}

/// prints output to stdin
fn print_solution(tour: &tour::Tour, is_optimized: bool) {
    let optimization_flag = if is_optimized { 1 } else { 0 };

    println!("{:.5} {}", tour.total, optimization_flag);
    for city_id in tour.route().iter() {
        print!("{} ", city_id);
    }

    print!("\n");
}

/// reads cities from STDIN
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

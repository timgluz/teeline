extern crate clap;
extern crate rand;

use clap::{arg_enum, App, Arg, ArgMatches};

use std::fmt::Debug;
use std::str::FromStr;

use teeline::tsp::{self, kdtree, tour, SolverOptions, Solvers};

fn main() {
    //process command-line params
    let args = App::new("Teeline")
        .version(tsp::VERSION)
        .author(tsp::AUTHOR)
        .about("Solver for Traveling Salesman problem")
        .arg(
            Arg::with_name("solver")
                .index(1)
                .help("specify an algorithm to use")
                .possible_values(&Solvers::variants())
                .required(true)
                .value_name("SOLVER_NAME")
                .case_insensitive(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("epochs")
                .long("epochs")
                .help("specify how many maximum iterations before stopping, 0 is forever")
                .required(false),
        )
        .arg(
            Arg::with_name("platoo_epochs")
                .long("platoo_epochs")
                .help("specify how many steps until stop searching on platoo")
                .required(false),
        )
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .help("allows solver to print out debug lines")
                .required(false),
        )
        .get_matches();

    let solver_type = Solvers::from_str(args.value_of("solver").unwrap_or("unspecified"))
        .expect("Unknown solver");

    let options = solver_options_from_args(&args);
    if options.verbose {
        println!("Selected solver: {:?}", solver_type);
    }

    // todo: read stdin only if FILEPATH is not given
    let n_points = read_value::<usize>();
    let cities = read_cities(n_points);

    let tour = solve(solver_type, &cities, &options);

    print_solution(&tour, false);
}

/// solves tsp for given cities by using solver
fn solve(algorithm: Solvers, cities: &[kdtree::KDPoint], options: &SolverOptions) -> tour::Tour {
    match algorithm {
        Solvers::BellmanKarp => tsp::bellman_karp::solve(cities, options),
        Solvers::BranchBound => tsp::branch_bound::solve(cities, options),
        Solvers::NearestNeighbor => tsp::nearest_neighbor::solve(cities),
        Solvers::TwoOpt => tsp::two_opt::solve(cities),
        Solvers::StochasticHill => tsp::stochastic_hill::solve(cities, options),
        Solvers::SimulatedAnnealing => tsp::simulated_annealing::solve(cities),
        Solvers::TabuSearch => tsp::tabu_search::solve(cities),
        Solvers::GeneticAlgorithm => tsp::genetic_algorithm::solve(cities),
        _ => panic!("Unspecified solver"),
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

fn solver_options_from_args(args: &ArgMatches) -> SolverOptions {
    let mut options = SolverOptions::default();

    if let Some(n_epochs_str) = args.value_of("epochs") {
        options.epochs = usize::from_str(n_epochs_str).unwrap_or(0);
    }

    if let Some(n_platoo_str) = args.value_of("platoo_epochs") {
        options.platoo_epochs = usize::from_str(n_platoo_str).unwrap_or(0);
    }

    if args.is_present("verbose") {
        options.verbose = true;
    }

    options
}

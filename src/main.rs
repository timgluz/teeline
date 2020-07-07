extern crate clap;
extern crate rand;

use clap::{App, Arg, ArgMatches};

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
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("platoo_epochs")
                .long("platoo_epochs")
                .help("specify how many steps until stop searching on platoo")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("n_nearest")
                .long("n_nearest")
                .help("specify how many nearest neighbors to look for")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("n_elite")
                .long("n_elite")
                .help("specify how many strongest individuals to pass directly to next gen")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("mutation_probability")
                .long("mutation_probability")
                .help("specify mutation_probability that swaps 2 cities on new individual")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("cooling_rate")
                .long("cooling_rate")
                .help("specify cooling rate")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("min_temperature")
                .long("min_temperature")
                .help("specify minimum temperature")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("max_temperature")
                .long("max_temperature")
                .help("specify the maximum temperature")
                .takes_value(true)
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
        Solvers::NearestNeighbor => tsp::nearest_neighbor::solve(cities, options),
        Solvers::TwoOpt => tsp::two_opt::solve(cities, options),
        Solvers::StochasticHill => tsp::stochastic_hill::solve(cities, options),
        Solvers::SimulatedAnnealing => tsp::simulated_annealing::solve(cities, options),
        Solvers::TabuSearch => tsp::tabu_search::solve(cities, options),
        Solvers::GeneticAlgorithm => tsp::genetic_algorithm::solve(cities, options),
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

    if args.is_present("verbose") {
        options.verbose = true;
    }

    if let Some(n_epochs_str) = args.value_of("epochs") {
        options.epochs = usize::from_str(n_epochs_str).unwrap_or(0);
    }

    if let Some(n_platoo_str) = args.value_of("platoo_epochs") {
        options.platoo_epochs = usize::from_str(n_platoo_str).unwrap_or(0);
    }

    if let Some(n_nearest_str) = args.value_of("n_nearest") {
        options.n_nearest = usize::from_str(n_nearest_str).unwrap_or(0);
    }

    if let Some(n_elite_str) = args.value_of("n_elite") {
        options.n_elite = usize::from_str(n_elite_str).unwrap_or(0);
    }

    if let Some(mutation_prob_str) = args.value_of("mutation_probability") {
        options.mutation_probability = f32::from_str(mutation_prob_str).unwrap_or(0.0);
    }

    if let Some(cooling_rate_str) = args.value_of("cooling_rate") {
        options.cooling_rate = f32::from_str(cooling_rate_str).unwrap_or(0.0);
    }

    if let Some(min_temperature_str) = args.value_of("min_temperature") {
        options.min_temperature = f32::from_str(min_temperature_str).unwrap_or(0.0);
    }

    if let Some(max_temperature_str) = args.value_of("max_temperature") {
        options.max_temperature = f32::from_str(max_temperature_str).unwrap_or(0.0);
    }

    options
}

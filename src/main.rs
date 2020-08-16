extern crate clap;
extern crate lazy_static;
extern crate rand;
extern crate regex;
extern crate piston;
extern crate piston_window;

use clap::{App, Arg, ArgMatches};

use std::fmt::Debug;
use std::sync::mpsc;
use std::thread;

use std::path::Path;
use std::str::FromStr;

use teeline::tsp::{self, kdtree, progress, tsplib, Solution, SolverOptions, Solvers};

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
            Arg::with_name("input")
                .long("input")
                .short("i")
                .value_name("FILE_PATH")
                .help("filepath to input file, must be in TSPLIB format")
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
        .arg(
            Arg::with_name("disable_progress")
                .long("disable_progress")
                .help("Doesnt show any progress or visualization, default false")
                .required(false),
        )
        .get_matches();

    let solver_type = Solvers::from_str(args.value_of("solver").unwrap_or("unspecified"))
        .expect("Unknown solver");

    let options = solver_options_from_args(&args);
    if options.verbose {
        println!("Selected solver: {:?}", solver_type);
    }

    let tsp_data = if let Some(input_file_path) = args.value_of("input") {
        let file_path = Path::new(input_file_path);
        read_tsp_data_from_file(&file_path)
    } else {
        read_tsp_data_from_stdin()
    };

    if options.verbose {
        println!(
            "Problem details:\n\tname:{:?}\n\tcomment:{:?}\n\tcities:{:?}",
            tsp_data.name,
            tsp_data.comment,
            tsp_data.len()
        );
    }

    let (progress_publisher, progress_listener) = mpsc::channel();
    let mut progress_display = progress::ProgressPlot::new(tsp_data.cities(), options.clone());

    // start progress listener
    let handler1 = thread::spawn(move || {
        progress_display.run(progress_listener);
    });

    // execute solver
    let handler2 = thread::spawn(move || {
        let publisherfn = if options.show_progress {
            progress::build_publisher(progress_publisher.clone())
        } else {
            progress::build_dummy_publisher(options.verbose)
        };

        let tour = solve(
            solver_type,
            tsp_data.cities(),
            &options.clone(),
            publisherfn,
        );
        print_solution(&tour, false);

        // send listener that we are done
        progress_publisher
            .send(progress::ProgressMessage::Done)
            .unwrap();
    });

    // run threads
    handler1.join().expect("Progress Thread Failed");
    handler2.join().expect("Solver thread failed");
}

/// solves tsp for given cities by using solver
fn solve(
    algorithm: Solvers,
    cities: &[kdtree::KDPoint],
    options: &SolverOptions,
    publisherfn: progress::PublisherFn,
) -> Solution {
    match algorithm {
        Solvers::BellmanKarp => tsp::bellman_karp::solve(cities, options, publisherfn),
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
fn print_solution(tour: &Solution, is_optimized: bool) {
    let optimization_flag = if is_optimized { 1 } else { 0 };

    println!("{:.5} {}", tour.total, optimization_flag);
    for city_id in tour.route().iter() {
        print!("{} ", city_id);
    }

    print!("\n");
}

fn read_tsp_data_from_file(file_path: &Path) -> tsplib::TspLibData {
    if !file_path.exists() {
        eprintln!("File doesnt exists: {:?}", file_path);
        std::process::exit(1);
    }

    match tsplib::read_from_file(file_path) {
        Err(err_msg) => {
            eprintln!("Error in TSPLIB file: {:?}", err_msg);
            std::process::exit(1);
        }
        Ok(tsp_data_) => {
            return tsp_data_;
        }
    }
}

fn read_tsp_data_from_stdin() -> tsplib::TspLibData {
    match tsplib::read_from_stdin() {
        Err(err_msg) => {
            eprintln!("Failed to read TSPLIB file from STDIN: {:?}", err_msg);
            std::process::exit(1);
        }
        Ok(tsp_data_) => return tsp_data_,
    }
}

fn solver_options_from_args(args: &ArgMatches) -> SolverOptions {
    let mut options = SolverOptions::default();

    if args.is_present("verbose") {
        options.verbose = true;
    }

    if args.is_present("disable_progress") {
        options.show_progress = false;
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

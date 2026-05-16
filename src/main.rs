use clap::{Arg, ArgAction, ArgMatches, Command};

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread;

use teeline::tsp::{self, distance_matrix, progress, progress_eframe, tsplib, Solution, SolverOptions, Solvers};
use tracing_subscriber::EnvFilter;

fn main() {
    let solve_cmd = Command::new("solve")
        .about("Solve a TSP instance")
        .arg(
            Arg::new("solver")
                .index(1)
                .help("algorithm to use")
                .value_parser(Solvers::variants())
                .required(true)
                .value_name("SOLVER_NAME")
                .ignore_case(true),
        )
        .arg(
            Arg::new("epochs")
                .long("epochs")
                .help("maximum iterations before stopping, 0 is forever")
                .required(false),
        )
        .arg(
            Arg::new("platoo_epochs")
                .long("platoo_epochs")
                .help("steps until stop searching on plateau")
                .required(false),
        )
        .arg(
            Arg::new("n_nearest")
                .long("n_nearest")
                .help("nearest neighbours to look for")
                .required(false),
        )
        .arg(
            Arg::new("n_elite")
                .long("n_elite")
                .help("strongest individuals to pass directly to next generation")
                .required(false),
        )
        .arg(
            Arg::new("mutation_probability")
                .long("mutation_probability")
                .help("probability of swapping two cities on a new individual")
                .required(false),
        )
        .arg(
            Arg::new("cooling_rate")
                .long("cooling_rate")
                .help("cooling rate for simulated annealing")
                .required(false),
        )
        .arg(
            Arg::new("min_temperature")
                .long("min_temperature")
                .help("minimum temperature")
                .required(false),
        )
        .arg(
            Arg::new("max_temperature")
                .long("max_temperature")
                .help("maximum temperature")
                .required(false),
        )
        .arg(
            Arg::new("input")
                .long("input")
                .short('i')
                .value_name("FILE_PATH")
                .help("path to TSPLIB input file (reads stdin if omitted)")
                .required(false),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("print debug lines")
                .action(ArgAction::SetTrue)
                .required(false),
        )
        .arg(
            Arg::new("gui")
                .long("gui")
                .help("open the visualization window while solving")
                .action(ArgAction::SetTrue)
                .required(false),
        )
        .arg(
            Arg::new("optimal_tour")
                .long("optimal-tour")
                .value_name("FILE_PATH")
                .help("path to a .opt.tour file; overlays optimal route on visualization and prints gap")
                .required(false),
        );

    let convert_cmd = Command::new("convert")
        .about("Convert a DiscOpt coordinate file (or directory) to TSPLIB format")
        .arg(
            Arg::new("input")
                .long("input")
                .short('i')
                .value_name("PATH")
                .help("input file or directory")
                .required(true),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("PATH")
                .help("output file or directory (default: ./data/discopt/)")
                .required(false),
        );

    let cli = Command::new("Teeline")
        .version(tsp::VERSION)
        .author(tsp::AUTHOR)
        .about("Traveling Salesman Problem solver")
        .subcommand_required(true)
        .subcommand(solve_cmd)
        .subcommand(convert_cmd)
        .get_matches();

    match cli.subcommand() {
        Some(("solve", args)) => run_solve(args),
        Some(("convert", args)) => run_convert(args),
        _ => unreachable!("clap ensures a subcommand is always present"),
    }
}

fn run_solve(args: &ArgMatches) {
    let solver_type =
        Solvers::from_str(args.get_one::<String>("solver").map(|s| s.as_str()).unwrap_or("unspecified"))
            .expect("Unknown solver");

    let mut options = solver_options_from_args(args);

    let default_level = if options.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(format!("teeline={default_level}").parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!(algorithm = ?solver_type, "solver selected");

    let tsp_data = if let Some(input_file_path) = args.get_one::<String>("input") {
        let file_path = Path::new(input_file_path.as_str());
        read_tsp_data_from_file(file_path)
    } else {
        read_tsp_data_from_stdin()
    };

    tracing::info!(name = %tsp_data.name, comment = %tsp_data.comment,
                   cities = tsp_data.len(), "problem loaded");

    let cities = tsp_data.cities().to_vec();
    let distances = match tsp_data.distance_matrix() {
        Ok(dm) => dm,
        Err(e) => {
            eprintln!("Error building distance matrix: {e}");
            std::process::exit(1);
        }
    };

    let maybe_display: Option<progress_eframe::ProgressPlot>;
    if args.get_flag("gui") {
        let (display, tx) = progress_eframe::ProgressPlot::new_with_channel(&cities, 1024.0, 1024.0, 50.0);
        options.progress_tx = Some(tx);
        maybe_display = Some(display);
    } else {
        maybe_display = None;
    }

    let opt_tour = args
        .get_one::<String>("optimal_tour")
        .and_then(|p| match teeline::tsp::opt_tour::read_from_file(Path::new(p.as_str())) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("--optimal-tour: {e}");
                None
            }
        });

    if let Some(ref ot) = opt_tour {
        if let Some(ref tx) = options.progress_tx {
            if ot.dimension == tsp_data.len() {
                let _ = tx.send(progress::ProgressMessage::OptimalTour(ot.route.clone()));
            } else {
                eprintln!(
                    "--optimal-tour: dimension mismatch ({} vs {}); skipping visualization overlay",
                    ot.dimension,
                    tsp_data.len()
                );
            }
        }
    }

    let cities_for_solver = cities.clone();
    let distances_for_solver = distances.clone();
    let span = tracing::info_span!("solver", algorithm = ?solver_type);
    let solver_handle = thread::spawn(move || {
        let _enter = span.entered();
        let tour = tsp::solve(solver_type, &cities_for_solver, &distances_for_solver, &options);
        tracing::info!(tour_length = tour.total, "solver finished");
        print_solution(&tour, false);
        tour
    });

    if let Some(display) = maybe_display {
        display.run();
    }

    let tour = solver_handle.join().expect("Solver thread failed");

    if let Some(ot) = opt_tour {
        print_optimal_comparison(&tour, &distances, tsp_data.len(), &ot);
    }
}

fn run_convert(args: &ArgMatches) {
    let input = PathBuf::from(args.get_one::<String>("input").unwrap());
    let output = PathBuf::from(
        args.get_one::<String>("output")
            .map(|s| s.as_str())
            .unwrap_or("./data/discopt"),
    );

    if input.is_dir() {
        let (ok, errors) = tsp::convert::convert_dir(&input, &output);
        for (path, err) in &errors {
            eprintln!("error: {}: {}", path.display(), err);
        }
        println!("converted {ok}/{} files", ok + errors.len());
        if !errors.is_empty() {
            std::process::exit(1);
        }
    } else {
        let out_path = if output.is_dir() || output.extension().is_none() {
            let stem = input.file_stem().unwrap_or_default();
            output.join(stem).with_extension("tsp")
        } else {
            output
        };
        if let Err(e) = tsp::convert::convert_file(&input, &out_path) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}


fn print_solution(tour: &Solution, is_optimized: bool) {
    let optimization_flag = if is_optimized { 1 } else { 0 };
    println!("{:.5} {}", tour.total, optimization_flag);
    for city_id in tour.route().iter() {
        print!("{} ", city_id);
    }
    println!();
}

fn print_optimal_comparison(
    solver_tour: &Solution,
    distances: &distance_matrix::DistanceMatrix,
    n_cities: usize,
    opt_tour: &teeline::tsp::opt_tour::OptTour,
) {
    if opt_tour.dimension != n_cities {
        eprintln!(
            "--optimal-tour: dimension mismatch ({} vs {}); skipping comparison",
            opt_tour.dimension,
            n_cities
        );
        return;
    }

    let optimal_cost = distances.tour_length(&opt_tour.route);
    let solver_cost = solver_tour.total;
    let gap_pct = if optimal_cost > 0.0 {
        (solver_cost - optimal_cost) / optimal_cost * 100.0
    } else {
        0.0
    };

    eprintln!("--- Comparison ---");
    eprintln!("Optimal  : {:.5}  (from {})", optimal_cost, opt_tour.name);
    eprintln!("Solver   : {:.5}", solver_cost);
    if gap_pct.abs() < 0.001 {
        eprintln!("Gap      : 0.00 % (matches optimal)");
    } else {
        eprintln!("Gap      : {:+.2} %", gap_pct);
    }
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
        Ok(tsp_data_) => tsp_data_,
    }
}

fn read_tsp_data_from_stdin() -> tsplib::TspLibData {
    match tsplib::read_from_stdin() {
        Err(err_msg) => {
            eprintln!("Failed to read TSPLIB file from STDIN: {:?}", err_msg);
            std::process::exit(1);
        }
        Ok(tsp_data_) => tsp_data_,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal clap Command that mirrors the args read by `solver_options_from_args`.
    fn options_cmd() -> Command {
        Command::new("test")
            .arg(Arg::new("solver").index(1).required(true))
            .arg(Arg::new("verbose").long("verbose").action(ArgAction::SetTrue))
            .arg(Arg::new("gui").long("gui").action(ArgAction::SetTrue))
            .arg(Arg::new("epochs").long("epochs"))
            .arg(Arg::new("platoo_epochs").long("platoo_epochs"))
            .arg(Arg::new("n_nearest").long("n_nearest"))
            .arg(Arg::new("n_elite").long("n_elite"))
            .arg(Arg::new("mutation_probability").long("mutation_probability"))
            .arg(Arg::new("cooling_rate").long("cooling_rate"))
            .arg(Arg::new("min_temperature").long("min_temperature"))
            .arg(Arg::new("max_temperature").long("max_temperature"))
    }

    #[test]
    fn test_solver_options_defaults_preserved_when_no_args() {
        let args = options_cmd().get_matches_from(["test", "nn"]);
        let opts = solver_options_from_args(&args);
        let defaults = SolverOptions::default();
        assert!(!opts.verbose);
        assert!(opts.progress_tx.is_none());
        assert_eq!(opts.epochs, defaults.epochs);
        assert_eq!(opts.n_nearest, defaults.n_nearest);
    }

    #[test]
    fn test_solver_options_verbose_flag_sets_verbose() {
        let args = options_cmd().get_matches_from(["test", "nn", "--verbose"]);
        assert!(solver_options_from_args(&args).verbose);
    }

    #[test]
    fn test_solver_options_epochs_parsed() {
        let args = options_cmd().get_matches_from(["test", "nn", "--epochs", "500"]);
        assert_eq!(solver_options_from_args(&args).epochs, 500);
    }

    #[test]
    fn test_solver_options_invalid_epochs_defaults_to_zero() {
        let args = options_cmd().get_matches_from(["test", "nn", "--epochs", "bad"]);
        assert_eq!(solver_options_from_args(&args).epochs, 0);
    }

    #[test]
    fn test_solver_options_platoo_epochs_parsed() {
        let args = options_cmd().get_matches_from(["test", "nn", "--platoo_epochs", "200"]);
        assert_eq!(solver_options_from_args(&args).platoo_epochs, 200);
    }

    #[test]
    fn test_solver_options_n_nearest_parsed() {
        let args = options_cmd().get_matches_from(["test", "nn", "--n_nearest", "5"]);
        assert_eq!(solver_options_from_args(&args).n_nearest, 5);
    }

    #[test]
    fn test_solver_options_n_elite_parsed() {
        let args = options_cmd().get_matches_from(["test", "nn", "--n_elite", "7"]);
        assert_eq!(solver_options_from_args(&args).n_elite, 7);
    }

    #[test]
    fn test_solver_options_mutation_probability_parsed() {
        let args = options_cmd().get_matches_from(["test", "nn", "--mutation_probability", "0.05"]);
        let opts = solver_options_from_args(&args);
        assert!((opts.mutation_probability - 0.05).abs() < 1e-5);
    }

    #[test]
    fn test_solver_options_invalid_mutation_probability_defaults_to_zero() {
        let args = options_cmd().get_matches_from(["test", "nn", "--mutation_probability", "nope"]);
        assert!((solver_options_from_args(&args).mutation_probability - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_solver_options_cooling_rate_parsed() {
        let args = options_cmd().get_matches_from(["test", "nn", "--cooling_rate", "0.001"]);
        let opts = solver_options_from_args(&args);
        assert!((opts.cooling_rate - 0.001).abs() < 1e-6);
    }

    #[test]
    fn test_solver_options_temperature_bounds_parsed() {
        let args = options_cmd().get_matches_from([
            "test", "nn",
            "--min_temperature", "0.5",
            "--max_temperature", "500.0",
        ]);
        let opts = solver_options_from_args(&args);
        assert!((opts.min_temperature - 0.5).abs() < 1e-5);
        assert!((opts.max_temperature - 500.0).abs() < 1e-3);
    }
}

fn solver_options_from_args(args: &ArgMatches) -> SolverOptions {
    let mut options = SolverOptions::default();

    if args.get_flag("verbose") {
        options.verbose = true;
    }

    if let Some(n_epochs_str) = args.get_one::<String>("epochs") {
        options.epochs = usize::from_str(n_epochs_str).unwrap_or(0);
    }

    if let Some(n_platoo_str) = args.get_one::<String>("platoo_epochs") {
        options.platoo_epochs = usize::from_str(n_platoo_str).unwrap_or(0);
    }

    if let Some(n_nearest_str) = args.get_one::<String>("n_nearest") {
        options.n_nearest = usize::from_str(n_nearest_str).unwrap_or(0);
    }

    if let Some(n_elite_str) = args.get_one::<String>("n_elite") {
        options.n_elite = usize::from_str(n_elite_str).unwrap_or(0);
    }

    if let Some(mutation_prob_str) = args.get_one::<String>("mutation_probability") {
        options.mutation_probability = f32::from_str(mutation_prob_str).unwrap_or(0.0);
    }

    if let Some(cooling_rate_str) = args.get_one::<String>("cooling_rate") {
        options.cooling_rate = f32::from_str(cooling_rate_str).unwrap_or(0.0);
    }

    if let Some(min_temperature_str) = args.get_one::<String>("min_temperature") {
        options.min_temperature = f32::from_str(min_temperature_str).unwrap_or(0.0);
    }

    if let Some(max_temperature_str) = args.get_one::<String>("max_temperature") {
        options.max_temperature = f32::from_str(max_temperature_str).unwrap_or(0.0);
    }

    options
}

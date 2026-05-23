use clap::{Arg, ArgAction, ArgMatches, Command};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread;

use teeline::config::{
    resolve_config_file, select_pipeline_source, IdentityProvider, OptionsProvider, PipelineSource,
};
use teeline::tsp::{
    self, distance_matrix,
    pipeline::{run_pipeline, PipelineStage},
    progress, progress_eframe, tsplib, AppOptions, Solution, Solvers, TspProblem,
};
use tracing_subscriber::EnvFilter;

// ---------------------------------------------------------------------------
// CliArgsProvider — applies CLI tuning flags to a base AppOptions.
// Solver-aware: routes args to the right XOptions slot based on the active solver.
// ---------------------------------------------------------------------------

struct CliArgsProvider<'a> {
    args: &'a ArgMatches,
    solver: Solvers,
}

impl<'a> CliArgsProvider<'a> {
    fn new(args: &'a ArgMatches, solver: Solvers) -> Self {
        CliArgsProvider { args, solver }
    }
}

impl OptionsProvider for CliArgsProvider<'_> {
    fn provide(&self, mut base: AppOptions) -> Result<AppOptions, String> {
        use teeline::tsp::{CSOptions, FPAOptions, GAOptions, HeuristicOptions, SAOptions};
        match self.solver {
            Solvers::SimulatedAnnealing => {
                base.sa = Some(SAOptions::from_cli(self.args));
            }
            Solvers::GeneticAlgorithm => {
                base.ga = Some(GAOptions::from_cli(self.args));
            }
            Solvers::CuckooSearch => {
                base.cs = Some(CSOptions::from_cli(self.args));
            }
            Solvers::FlowerPollination => {
                base.fpa = Some(FPAOptions::from_cli(self.args));
            }
            _ => {
                base.heuristic = Some(HeuristicOptions::from_cli(self.args));
            }
        }
        Ok(base)
    }
}

fn solver_options_from_args(args: &ArgMatches, solver: Solvers) -> AppOptions {
    CliArgsProvider::new(args, solver)
        .provide(AppOptions::default())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

fn main() {
    let tuning = tuning_args();

    let solve_cmd = Command::new("solve")
        .about("Solve a TSP instance. Deterministic solvers (2opt, 3opt, tabu) auto-expand to \
                pipeline(nn, solver); stochastic solvers (sa, ga, pso, cs, fpa, stochastic_hill) \
                auto-expand to pipeline(shuffle, solver).")
        .arg(
            Arg::new("solver")
                .index(1)
                .help("algorithm to use, or preset: classic, fast, thorough")
                .value_parser(Solvers::variants())
                .required(true)
                .value_name("SOLVER_NAME")
                .ignore_case(true),
        )
        .arg(
            Arg::new("no_seed")
                .long("no-seed")
                .help("disable automatic warm-start; run solver from input city order")
                .action(ArgAction::SetTrue)
                .required(false),
        )
        .args(tuning.clone());

    let pipeline_cmd = Command::new("pipeline")
        .about("Chain multiple solvers; each stage warm-starts from the previous result. \
                Provide either --steps or --config (mutually exclusive).")
        .arg(
            Arg::new("steps")
                .long("steps")
                .value_delimiter(',')
                .value_name("SOLVERS")
                .help("comma-separated solver names to chain (e.g. nn,2opt,sa)")
                .required(false),
        )
        .arg(
            Arg::new("config")
                .long("config")
                .value_name("PATH")
                .help("path to a TOML config file with per-stage option overrides")
                .required(false),
        )
        .args(tuning);

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
        .subcommand(pipeline_cmd)
        .subcommand(convert_cmd)
        .get_matches();

    match cli.subcommand() {
        Some(("solve", args)) => run_solve(args),
        Some(("pipeline", args)) => run_pipeline_cmd(args),
        Some(("convert", args)) => run_convert(args),
        _ => unreachable!("clap ensures a subcommand is always present"),
    }
}

fn tuning_args() -> Vec<Arg> {
    vec![
        Arg::new("epochs")
            .long("epochs")
            .help("maximum iterations before stopping, 0 is forever")
            .required(false),
        Arg::new("platoo_epochs")
            .long("platoo_epochs")
            .help("steps until stop searching on plateau")
            .required(false),
        Arg::new("n_nearest")
            .long("n_nearest")
            .help("nearest neighbours to look for")
            .required(false),
        Arg::new("n_elite")
            .long("n_elite")
            .help("strongest individuals to pass directly to next generation")
            .required(false),
        Arg::new("mutation_probability")
            .long("mutation_probability")
            .help("probability of swapping two cities on a new individual")
            .required(false),
        Arg::new("cooling_rate")
            .long("cooling_rate")
            .help("cooling rate for simulated annealing")
            .required(false),
        Arg::new("min_temperature")
            .long("min_temperature")
            .help("minimum temperature")
            .required(false),
        Arg::new("max_temperature")
            .long("max_temperature")
            .help("maximum temperature")
            .required(false),
        Arg::new("input")
            .long("input")
            .short('i')
            .value_name("FILE_PATH")
            .help("path to TSPLIB input file (reads stdin if omitted)")
            .required(false),
        Arg::new("verbose")
            .long("verbose")
            .short('v')
            .help("print debug lines")
            .action(ArgAction::SetTrue)
            .required(false),
        Arg::new("gui")
            .long("gui")
            .help("open the visualization window while solving")
            .action(ArgAction::SetTrue)
            .required(false),
        Arg::new("optimal_tour")
            .long("optimal-tour")
            .value_name("FILE_PATH")
            .help("path to .opt.tour file; overlays optimal route and prints gap")
            .required(false),
    ]
}

// ---------------------------------------------------------------------------
// Subcommand handlers
// ---------------------------------------------------------------------------

fn resolve_preset(name: &str) -> Option<Vec<Solvers>> {
    match name {
        "classic" => Some(vec![
            Solvers::NearestNeighbor,
            Solvers::TwoOpt,
            Solvers::SimulatedAnnealing,
        ]),
        "fast" => Some(vec![Solvers::NearestNeighbor, Solvers::TwoOpt]),
        "thorough" => Some(vec![
            Solvers::NearestNeighbor,
            Solvers::ThreeOpt,
            Solvers::SimulatedAnnealing,
        ]),
        _ => None,
    }
}

fn run_solve(args: &ArgMatches) {
    let solver_name = args.get_one::<String>("solver").unwrap().as_str();
    let no_seed = args.get_flag("no_seed");

    if let Some(steps) = resolve_preset(solver_name) {
        run_as_pipeline(&steps, args);
        return;
    }

    let solver =
        Solvers::from_str(solver_name).expect("unknown solver — clap should have caught this");

    if !no_seed {
        if solver.auto_expand_with_nn() {
            run_as_pipeline(&[Solvers::NearestNeighbor, solver], args);
        } else if solver.auto_expand_with_shuffle() {
            run_as_pipeline(&[Solvers::RandomShuffle, solver], args);
        } else {
            run_as_pipeline(&[solver], args);
        }
    } else {
        run_as_pipeline(&[solver], args);
    }
}

fn run_pipeline_cmd(args: &ArgMatches) {
    let config_path: Option<PathBuf> = args.get_one::<String>("config").map(PathBuf::from);
    let steps_vec: Option<Vec<String>> = args
        .get_many::<String>("steps")
        .map(|m| m.cloned().collect());

    match select_pipeline_source(config_path.as_deref(), steps_vec.as_deref()) {
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        Ok(PipelineSource::Config(path)) => {
            let stage_configs = match resolve_config_file(&path, &IdentityProvider) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            };
            run_as_pipeline_stages(stage_configs, args);
        }
        Ok(PipelineSource::Steps(solvers)) => {
            run_as_pipeline(&solvers, args);
        }
    }
}

fn run_as_pipeline(steps: &[Solvers], args: &ArgMatches) {
    let stage_configs: Vec<(Solvers, AppOptions)> =
        steps.iter().map(|&s| (s, solver_options_from_args(args, s))).collect();
    run_as_pipeline_stages(stage_configs, args);
}

fn run_as_pipeline_stages(stage_configs: Vec<(Solvers, AppOptions)>, args: &ArgMatches) {
    let verbose = args.get_flag("verbose");
    let default_level = if verbose { "debug" } else { "info" };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(format!("teeline={default_level}").parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .try_init();

    for (i, (solver, _)) in stage_configs.iter().enumerate() {
        if i > 0 && *solver == Solvers::NearestNeighbor {
            eprintln!("warning: nn at pipeline stage {i} discards the warm-start seed");
        }
    }

    let tsp_data = if let Some(input_file_path) = args.get_one::<String>("input") {
        read_tsp_data_from_file(Path::new(input_file_path.as_str()))
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
    let progress_tx;
    if args.get_flag("gui") {
        let (display, tx) =
            progress_eframe::ProgressPlot::new_with_channel(&cities, 1024.0, 1024.0, 50.0);
        progress_tx = Some(tx);
        maybe_display = Some(display);
    } else {
        progress_tx = None;
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

    if let Some(ref ot) = opt_tour
        && let Some(ref tx) = progress_tx
    {
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

    let mut stages: Vec<PipelineStage> = stage_configs
        .into_iter()
        .map(|(solver, options)| {
            PipelineStage::new(solver, options, TspProblem::new(cities.clone(), distances.clone()), progress_tx.clone())
        })
        .collect();

    let n_stages = stages.len();
    let span = tracing::info_span!("solver", n_stages);
    let solver_handle = thread::spawn(move || {
        let _enter = span.entered();
        let tour = run_pipeline(&mut stages).expect("solver failed");
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

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

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
            opt_tour.dimension, n_cities
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use teeline::tsp::HeuristicOptions;

    /// Minimal clap Command that mirrors the args read by `solver_options_from_args`.
    fn options_cmd() -> Command {
        Command::new("test")
            .arg(Arg::new("solver").index(1).required(true))
            .arg(Arg::new("no_seed").long("no-seed").action(ArgAction::SetTrue))
            .args(tuning_args())
    }

    #[test]
    fn test_solver_options_defaults_preserved_when_no_args() {
        let args = options_cmd().get_matches_from(["test", "nn"]);
        let opts = solver_options_from_args(&args, Solvers::NearestNeighbor);
        let h = opts.heuristic.unwrap_or_default();
        assert!(!h.verbose);
        assert_eq!(h.epochs, HeuristicOptions::default().epochs);
        assert_eq!(h.n_nearest, HeuristicOptions::default().n_nearest);
    }

    #[test]
    fn test_solver_options_verbose_flag_sets_verbose() {
        let args = options_cmd().get_matches_from(["test", "nn", "--verbose"]);
        let opts = solver_options_from_args(&args, Solvers::NearestNeighbor);
        assert!(opts.heuristic.unwrap_or_default().verbose);
    }

    #[test]
    fn test_solver_options_epochs_parsed() {
        let args = options_cmd().get_matches_from(["test", "nn", "--epochs", "500"]);
        let opts = solver_options_from_args(&args, Solvers::NearestNeighbor);
        assert_eq!(opts.heuristic.unwrap_or_default().epochs, 500);
    }

    #[test]
    fn test_solver_options_invalid_epochs_keeps_default() {
        let args = options_cmd().get_matches_from(["test", "nn", "--epochs", "bad"]);
        let opts = solver_options_from_args(&args, Solvers::NearestNeighbor);
        assert_eq!(opts.heuristic.unwrap_or_default().epochs, HeuristicOptions::default().epochs);
    }

    #[test]
    fn test_solver_options_platoo_epochs_parsed() {
        let args = options_cmd().get_matches_from(["test", "nn", "--platoo_epochs", "200"]);
        let opts = solver_options_from_args(&args, Solvers::NearestNeighbor);
        assert_eq!(opts.heuristic.unwrap_or_default().platoo_epochs, 200);
    }

    #[test]
    fn test_solver_options_n_nearest_parsed() {
        let args = options_cmd().get_matches_from(["test", "nn", "--n_nearest", "5"]);
        let opts = solver_options_from_args(&args, Solvers::NearestNeighbor);
        assert_eq!(opts.heuristic.unwrap_or_default().n_nearest, 5);
    }

    #[test]
    fn test_solver_options_n_elite_parsed() {
        let args = options_cmd().get_matches_from(["test", "ga", "--n_elite", "7"]);
        let opts = solver_options_from_args(&args, Solvers::GeneticAlgorithm);
        assert_eq!(opts.ga.unwrap_or_default().n_elite, 7);
    }

    #[test]
    fn test_solver_options_mutation_probability_parsed() {
        let args = options_cmd()
            .get_matches_from(["test", "ga", "--mutation_probability", "0.05"]);
        let opts = solver_options_from_args(&args, Solvers::GeneticAlgorithm);
        assert!((opts.ga.unwrap_or_default().mutation_probability - 0.05).abs() < 1e-5);
    }

    #[test]
    fn test_solver_options_invalid_mutation_probability_keeps_default() {
        let args = options_cmd()
            .get_matches_from(["test", "ga", "--mutation_probability", "nope"]);
        let opts = solver_options_from_args(&args, Solvers::GeneticAlgorithm);
        use teeline::tsp::GAOptions;
        assert!((opts.ga.unwrap_or_default().mutation_probability
            - GAOptions::default().mutation_probability)
            .abs()
            < 1e-5);
    }

    #[test]
    fn test_solver_options_cooling_rate_parsed() {
        let args = options_cmd().get_matches_from(["test", "sa", "--cooling_rate", "0.001"]);
        let opts = solver_options_from_args(&args, Solvers::SimulatedAnnealing);
        assert!((opts.sa.unwrap_or_default().cooling_rate - 0.001).abs() < 1e-6);
    }

    #[test]
    fn test_solver_options_temperature_bounds_parsed() {
        let args = options_cmd().get_matches_from([
            "test",
            "sa",
            "--min_temperature",
            "0.5",
            "--max_temperature",
            "500.0",
        ]);
        let opts = solver_options_from_args(&args, Solvers::SimulatedAnnealing);
        let sa = opts.sa.unwrap_or_default();
        assert!((sa.min_temperature - 0.5).abs() < 1e-5);
        assert!((sa.max_temperature - 500.0).abs() < 1e-3);
    }

    #[test]
    fn test_cli_args_provider_overrides_base() {
        let args = options_cmd().get_matches_from(["test", "nn", "--epochs", "777"]);
        let base = AppOptions::default();
        let opts = CliArgsProvider::new(&args, Solvers::NearestNeighbor).provide(base).unwrap();
        assert_eq!(opts.heuristic.unwrap_or_default().epochs, 777);
    }

    #[test]
    fn test_cli_args_provider_no_args_uses_defaults() {
        let args = options_cmd().get_matches_from(["test", "nn"]);
        let base = AppOptions::default();
        let opts = CliArgsProvider::new(&args, Solvers::NearestNeighbor).provide(base).unwrap();
        assert_eq!(
            opts.heuristic.unwrap_or_default().epochs,
            HeuristicOptions::default().epochs
        );
    }

    #[test]
    fn test_resolve_preset_classic_returns_three_steps() {
        let steps = resolve_preset("classic").unwrap();
        assert_eq!(
            steps,
            vec![Solvers::NearestNeighbor, Solvers::TwoOpt, Solvers::SimulatedAnnealing]
        );
    }

    #[test]
    fn test_resolve_preset_fast_returns_two_steps() {
        let steps = resolve_preset("fast").unwrap();
        assert_eq!(steps, vec![Solvers::NearestNeighbor, Solvers::TwoOpt]);
    }

    #[test]
    fn test_resolve_preset_thorough_returns_three_steps() {
        let steps = resolve_preset("thorough").unwrap();
        assert_eq!(
            steps,
            vec![Solvers::NearestNeighbor, Solvers::ThreeOpt, Solvers::SimulatedAnnealing]
        );
    }

    #[test]
    fn test_resolve_preset_unknown_returns_none() {
        assert!(resolve_preset("nn").is_none());
        assert!(resolve_preset("sa").is_none());
        assert!(resolve_preset("bogus").is_none());
    }
}

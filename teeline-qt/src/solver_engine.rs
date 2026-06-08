use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc;
use std::time::Instant;

use serde_json::json;
use qtbridge::{QObjectHolder, invoke_method, qobject_impl};
use teeline::tsp::{
    self, AppOptions, GAOptions, CSOptions, FPAOptions, HeuristicOptions, SAOptions,
    Solvers, TspProblem,
    kdtree::KDPoint,
    pipeline::{PipelineStage, run_pipeline},
    progress::ProgressMessage,
    tsplib,
};

pub struct SolverEngine {
    selected_solver: String,
    running: bool,
    best_cost: f32,
    iteration: i32,
    elapsed_ms: i32,
    tour_json: String,
    solvers_json: String,
    opt_tour_route_json: String,
    comparison_json: String,
}

impl Default for SolverEngine {
    fn default() -> Self {
        Self {
            selected_solver: String::new(),
            running: false,
            best_cost: 0.0,
            iteration: 0,
            elapsed_ms: 0,
            tour_json: "[]".to_string(),
            solvers_json: build_solvers_json(),
            opt_tour_route_json: "[]".to_string(),
            comparison_json: String::new(),
        }
    }
}

fn build_solvers_json() -> String {
    let arr: Vec<serde_json::Value> = teeline::tsp::list_solvers()
        .iter()
        .map(|s| json!({
            "name":       s.name,
            "alias":      s.alias,
            "category":   s.category,
            "desc":       s.desc,
            "complexity": s.complexity,
            "hasOptions": s.has_options,
            "exact":      s.exact
        }))
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

#[qobject_impl(Singleton)]
impl SolverEngine {
    qproperty!("selectedSolver",    Member = selected_solver,    Write = set_selected_solver,    Notify = "selectedSolverChanged");
    qproperty!("running",           Member = running,            Write = set_running,            Notify = "runningChanged");
    qproperty!("bestCost",          Member = best_cost,          Write = set_best_cost,          Notify = "bestCostChanged");
    qproperty!("iteration",         Member = iteration,          Write = set_iteration,          Notify = "iterationChanged");
    qproperty!("elapsedMs",         Member = elapsed_ms,         Write = set_elapsed_ms,         Notify = "elapsedMsChanged");
    qproperty!("tourJson",          Member = tour_json,          Write = set_tour_json,          Notify = "tourJsonChanged");
    qproperty!("solversJson",       Member = solvers_json,       Write = set_solvers_json,       Notify = "solversJsonChanged");
    qproperty!("optTourRouteJson",  Member = opt_tour_route_json, Write = set_opt_tour_route_json, Notify = "optTourRouteJsonChanged");
    qproperty!("comparisonJson",    Member = comparison_json,    Write = set_comparison_json,    Notify = "comparisonJsonChanged");

    fn set_selected_solver(&mut self, v: String)      { self.selected_solver = v;      self.selected_solver_changed(); }
    fn set_running(&mut self, v: bool)                { self.running = v;               self.running_changed(); }
    fn set_best_cost(&mut self, v: f32)               { self.best_cost = v;             self.best_cost_changed(); }
    fn set_iteration(&mut self, v: i32)               { self.iteration = v;             self.iteration_changed(); }
    fn set_elapsed_ms(&mut self, v: i32)              { self.elapsed_ms = v;            self.elapsed_ms_changed(); }
    fn set_tour_json(&mut self, v: String)            { self.tour_json = v;             self.tour_json_changed(); }
    fn set_solvers_json(&mut self, v: String)         { self.solvers_json = v;          self.solvers_json_changed(); }
    fn set_opt_tour_route_json(&mut self, v: String)  { self.opt_tour_route_json = v;   self.opt_tour_route_json_changed(); }
    fn set_comparison_json(&mut self, v: String)      { self.comparison_json = v;       self.comparison_json_changed(); }

    #[qsignal] fn selected_solver_changed(&self);
    #[qsignal] fn running_changed(&self);
    #[qsignal] fn best_cost_changed(&self);
    #[qsignal] fn iteration_changed(&self);
    #[qsignal] fn elapsed_ms_changed(&self);
    #[qsignal] fn tour_json_changed(&self);
    #[qsignal] fn solvers_json_changed(&self);
    #[qsignal] fn opt_tour_route_json_changed(&self);
    #[qsignal] fn comparison_json_changed(&self);

    #[qslot]
    fn select_solver(&mut self, alias: String) {
        self.set_selected_solver(alias);
    }

    /// Reset state and launch the solver with default options.
    #[qslot]
    fn start_solve(&mut self, file_path: String) {
        self.launch(file_path, AppOptions::default());
    }

    /// Reset state and launch the solver with options parsed from a JSON object.
    /// The JSON keys map to the active solver's option fields.
    #[qslot]
    fn start_solve_with_opts(&mut self, file_path: String, opts_json: String) {
        let opts = build_app_options(&self.selected_solver, &opts_json);
        self.launch(file_path, opts);
    }

    /// Called on the Qt main thread by the progress-forwarding thread.
    #[qslot]
    fn on_progress_update(&mut self, tour_json: String, cost: f32, iteration: i32, elapsed_ms: i32) {
        self.set_tour_json(tour_json);
        self.set_best_cost(cost);
        self.set_iteration(iteration);
        self.set_elapsed_ms(elapsed_ms);
    }

    /// Called on the Qt main thread when the solver finishes (or errors).
    #[qslot]
    fn on_solve_done(&mut self, tour_json: String, cost: f32, elapsed_ms: i32, _error: String) {
        self.set_tour_json(tour_json);
        self.set_best_cost(cost);
        self.set_elapsed_ms(elapsed_ms);
        self.set_running(false);
    }

    /// Stop displaying updates (solver continues in background).
    #[qslot]
    fn cancel(&mut self) {
        self.set_running(false);
    }

    /// Called on the Qt main thread when comparison stats are ready.
    #[qslot]
    fn on_comparison_ready(&mut self, comparison_json: String) {
        self.set_comparison_json(comparison_json);
    }

    /// Run a JSON-described pipeline: `[{"solver":"nn"},{"solver":"2opt"},...]`.
    #[qslot]
    fn run_pipeline_stages(&mut self, file_path: String, stages_json: String) {
        self.set_running(true);
        self.set_best_cost(0.0);
        self.set_iteration(0);
        self.set_elapsed_ms(0);
        self.set_tour_json("[]".to_string());
        self.set_comparison_json(String::new());

        let inv_progress = self.get_qml_method_invoker();
        let inv_done = self.get_qml_method_invoker();
        let inv_cmp = self.get_qml_method_invoker();
        let opt_json = self.opt_tour_route_json.clone();

        std::thread::spawn(move || {
            let stage_specs: Vec<serde_json::Value> =
                serde_json::from_str(&stages_json).unwrap_or_default();

            let data = match tsplib::read_from_file(Path::new(&file_path)) {
                Ok(d) => d,
                Err(e) => {
                    invoke_method!(inv_done, "onSolveDone", "[]".to_string(), 0.0_f32, 0i32, e);
                    return;
                }
            };
            let cities = data.cities().to_vec();
            let distances = match data.distance_matrix() {
                Ok(d) => d,
                Err(e) => {
                    invoke_method!(inv_done, "onSolveDone", "[]".to_string(), 0.0_f32, 0i32, e.to_string());
                    return;
                }
            };
            let problem = TspProblem::new(cities, distances);
            let start = Instant::now();

            // Build pipeline stages — each gets the same problem + its own progress channel.
            let (tx, rx) = mpsc::channel::<ProgressMessage>();
            let inv2 = inv_progress;
            std::thread::spawn(move || {
                let mut epoch = 0i32;
                while let Ok(msg) = rx.recv() {
                    match msg {
                        ProgressMessage::PathUpdate(route, cost) => {
                            epoch += 1;
                            let tour = route_to_json(route.route());
                            let ms = start.elapsed().as_millis() as i32;
                            invoke_method!(inv2, "onProgressUpdate", tour, cost, epoch, ms);
                        }
                        ProgressMessage::EpochUpdate(n) => { epoch = n as i32; }
                        ProgressMessage::Done | ProgressMessage::OptimalTour(_) => break,
                        _ => {}
                    }
                }
            });

            let stages: Vec<PipelineStage> = stage_specs
                .iter()
                .filter_map(|s| {
                    let alias = s.get("solver")?.as_str()?;
                    let solver = Solvers::from_str(alias).ok()?;
                    let opts_str = s.get("opts")
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "{}".to_string());
                    let opts = build_app_options(alias, &opts_str);
                    Some(PipelineStage::new(solver, opts, problem.clone(), Some(tx.clone())))
                })
                .collect();

            if stages.is_empty() {
                invoke_method!(inv_done, "onSolveDone", "[]".to_string(), 0.0_f32, 0i32,
                               "No valid stages in pipeline".to_string());
                return;
            }

            match run_pipeline(&stages) {
                Ok(solution) => {
                    let tour = route_to_json(solution.route());
                    let ms = start.elapsed().as_millis() as i32;
                    invoke_method!(inv_done, "onSolveDone", tour, solution.total, ms, String::new());
                    let comp_json = make_comparison_json(&opt_json, solution.route(), &problem.cities);
                    if !comp_json.is_empty() {
                        invoke_method!(inv_cmp, "onComparisonReady", comp_json);
                    }
                }
                Err(e) => {
                    invoke_method!(inv_done, "onSolveDone", "[]".to_string(), 0.0_f32, 0i32, e);
                }
            }
        });
    }
}

impl SolverEngine {
    fn launch(&mut self, file_path: String, opts: AppOptions) {
        self.set_running(true);
        self.set_best_cost(0.0);
        self.set_iteration(0);
        self.set_elapsed_ms(0);
        self.set_tour_json("[]".to_string());
        self.set_comparison_json(String::new());

        let inv_progress = self.get_qml_method_invoker();
        let inv_done = self.get_qml_method_invoker();
        let inv_cmp = self.get_qml_method_invoker();
        let alias = self.selected_solver.clone();
        let opt_json = self.opt_tour_route_json.clone();

        std::thread::spawn(move || {
            let solver = match Solvers::from_str(&alias) {
                Ok(s) => s,
                Err(_) => {
                    invoke_method!(inv_done, "onSolveDone", "[]".to_string(), 0.0_f32, 0i32,
                                   format!("Unknown solver: {alias}"));
                    return;
                }
            };

            let data = match tsplib::read_from_file(Path::new(&file_path)) {
                Ok(d) => d,
                Err(e) => {
                    invoke_method!(inv_done, "onSolveDone", "[]".to_string(), 0.0_f32, 0i32, e);
                    return;
                }
            };

            let cities = data.cities().to_vec();
            let distances = match data.distance_matrix() {
                Ok(d) => d,
                Err(e) => {
                    invoke_method!(inv_done, "onSolveDone", "[]".to_string(), 0.0_f32, 0i32, e.to_string());
                    return;
                }
            };
            let problem = TspProblem::new(cities, distances);

            let (tx, rx) = mpsc::channel::<ProgressMessage>();
            let start = Instant::now();

            std::thread::spawn(move || {
                let mut epoch = 0i32;
                while let Ok(msg) = rx.recv() {
                    match msg {
                        ProgressMessage::PathUpdate(route, cost) => {
                            epoch += 1;
                            let tour = route_to_json(route.route());
                            let ms = start.elapsed().as_millis() as i32;
                            invoke_method!(inv_progress, "onProgressUpdate", tour, cost, epoch, ms);
                        }
                        ProgressMessage::EpochUpdate(n) => { epoch = n as i32; }
                        ProgressMessage::Done | ProgressMessage::OptimalTour(_) => break,
                        _ => {}
                    }
                }
            });

            match tsp::solve_with_context(solver, &problem, &opts, Some(tx), None) {
                Ok(solution) => {
                    let tour = route_to_json(solution.route());
                    let ms = start.elapsed().as_millis() as i32;
                    invoke_method!(inv_done, "onSolveDone", tour, solution.total, ms, String::new());
                    let comp_json = make_comparison_json(&opt_json, solution.route(), &problem.cities);
                    if !comp_json.is_empty() {
                        invoke_method!(inv_cmp, "onComparisonReady", comp_json);
                    }
                }
                Err(e) => {
                    invoke_method!(inv_done, "onSolveDone", "[]".to_string(), 0.0_f32, 0i32, e);
                }
            }
        });
    }
}

// ── Options construction ──────────────────────────────────────────────────────

fn get_f32(v: &serde_json::Value, key: &str, default: f32) -> f32 {
    v.get(key)
        .and_then(|x| x.as_f64())
        .map(|x| x as f32)
        .unwrap_or(default)
}

fn get_usize(v: &serde_json::Value, key: &str, default: usize) -> usize {
    v.get(key)
        .and_then(|x| x.as_u64())
        .map(|x| x as usize)
        .unwrap_or(default)
}

fn build_heuristic(v: &serde_json::Value) -> HeuristicOptions {
    let def = HeuristicOptions::default();
    HeuristicOptions {
        epochs:        get_usize(v, "epochs",        def.epochs),
        platoo_epochs: get_usize(v, "platoo_epochs", def.platoo_epochs),
        n_nearest:     get_usize(v, "n_nearest",     def.n_nearest),
        verbose:       false,
    }
}

fn build_app_options(alias: &str, json: &str) -> AppOptions {
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or(serde_json::Value::Object(Default::default()));
    let h = build_heuristic(&v);

    match alias {
        "sa" | "simulated_annealing" => {
            let def = SAOptions::default();
            AppOptions {
                sa: Some(SAOptions {
                    heuristic: h,
                    cooling_rate:    get_f32(&v, "cooling_rate",    def.cooling_rate),
                    min_temperature: get_f32(&v, "min_temperature", def.min_temperature),
                    max_temperature: get_f32(&v, "max_temperature", def.max_temperature),
                }),
                ..AppOptions::default()
            }
        }
        "ga" | "genetic_algorithm" => {
            let def = GAOptions::default();
            AppOptions {
                ga: Some(GAOptions {
                    heuristic: h,
                    mutation_probability: get_f32(&v, "mutation_probability", def.mutation_probability),
                    n_elite: get_usize(&v, "n_elite", def.n_elite),
                }),
                ..AppOptions::default()
            }
        }
        "cs" | "cuckoo_search" => {
            let def = CSOptions::default();
            AppOptions {
                cs: Some(CSOptions {
                    heuristic: h,
                    mutation_probability: get_f32(&v, "mutation_probability", def.mutation_probability),
                }),
                ..AppOptions::default()
            }
        }
        "fpa" | "flower_pollination" => {
            let def = FPAOptions::default();
            AppOptions {
                fpa: Some(FPAOptions {
                    heuristic: h,
                    mutation_probability: get_f32(&v, "mutation_probability", def.mutation_probability),
                }),
                ..AppOptions::default()
            }
        }
        // PSO and all others use HeuristicOptions
        _ => AppOptions {
            heuristic: Some(h),
            ..AppOptions::default()
        },
    }
}

fn make_comparison_json(opt_json: &str, solver: &[usize], cities: &[KDPoint]) -> String {
    if opt_json == "[]" || opt_json.is_empty() { return String::new(); }
    let opt: Vec<usize> = serde_json::from_str(opt_json).unwrap_or_default();
    if opt.is_empty() { return String::new(); }
    let s = teeline::tsp::compare_tours(solver, &opt, cities);
    serde_json::to_string(&json!({
        "optimalCost": s.optimal_cost,
        "solverCost":  s.solver_cost,
        "gapPct":      s.gap_pct,
        "sharedEdges": s.shared_edges,
        "solverOnlyEdges": s.solver_only_edges,
        "optimalOnlyEdges": s.optimal_only_edges
    })).unwrap_or_default()
}

fn route_to_json(route: &[usize]) -> String {
    let nums: Vec<String> = route.iter().map(|id| id.to_string()).collect();
    format!("[{}]", nums.join(","))
}

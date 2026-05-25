use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc;
use std::time::Instant;

use qtbridge::{QObjectHolder, invoke_method, qobject_impl};
use teeline::tsp::{
    self, AppOptions, Solvers, TspProblem,
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
        }
    }
}

#[qobject_impl(Singleton)]
impl SolverEngine {
    qproperty!("selectedSolver", Member = selected_solver, Write = set_selected_solver, Notify = "selectedSolverChanged");
    qproperty!("running",        Member = running,         Write = set_running,         Notify = "runningChanged");
    qproperty!("bestCost",       Member = best_cost,       Write = set_best_cost,       Notify = "bestCostChanged");
    qproperty!("iteration",      Member = iteration,       Write = set_iteration,       Notify = "iterationChanged");
    qproperty!("elapsedMs",      Member = elapsed_ms,      Write = set_elapsed_ms,      Notify = "elapsedMsChanged");
    qproperty!("tourJson",       Member = tour_json,       Write = set_tour_json,       Notify = "tourJsonChanged");

    fn set_selected_solver(&mut self, v: String)  { self.selected_solver = v;  self.selected_solver_changed(); }
    fn set_running(&mut self, v: bool)             { self.running = v;           self.running_changed(); }
    fn set_best_cost(&mut self, v: f32)            { self.best_cost = v;         self.best_cost_changed(); }
    fn set_iteration(&mut self, v: i32)            { self.iteration = v;         self.iteration_changed(); }
    fn set_elapsed_ms(&mut self, v: i32)           { self.elapsed_ms = v;        self.elapsed_ms_changed(); }
    fn set_tour_json(&mut self, v: String)         { self.tour_json = v;         self.tour_json_changed(); }

    #[qsignal] fn selected_solver_changed(&self);
    #[qsignal] fn running_changed(&self);
    #[qsignal] fn best_cost_changed(&self);
    #[qsignal] fn iteration_changed(&self);
    #[qsignal] fn elapsed_ms_changed(&self);
    #[qsignal] fn tour_json_changed(&self);

    #[qslot]
    fn select_solver(&mut self, alias: String) {
        self.set_selected_solver(alias);
    }

    /// Reset state and launch the solver on a background thread.
    #[qslot]
    fn start_solve(&mut self, file_path: String) {
        self.set_running(true);
        self.set_best_cost(0.0);
        self.set_iteration(0);
        self.set_elapsed_ms(0);
        self.set_tour_json("[]".to_string());

        // Two invokers: progress updates and the final done notification.
        let inv_progress = self.get_qml_method_invoker();
        let inv_done = self.get_qml_method_invoker();
        let alias = self.selected_solver.clone();

        std::thread::spawn(move || {
            let solver = match Solvers::from_str(&alias) {
                Ok(s) => s,
                Err(_) => {
                    invoke_method!(inv_done, "onSolveDone", "[]".to_string(), 0.0_f32, 0i32, format!("Unknown solver: {alias}"));
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
            let opts = AppOptions::default();

            let (tx, rx) = mpsc::channel::<ProgressMessage>();
            let start = Instant::now();

            // Progress-forwarding thread: receives solver updates, fires Qt slots.
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
                        ProgressMessage::EpochUpdate(n) => {
                            epoch = n as i32;
                        }
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
                }
                Err(e) => {
                    invoke_method!(inv_done, "onSolveDone", "[]".to_string(), 0.0_f32, 0i32, e);
                }
            }
        });
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

    /// Stop displaying updates (solver continues in background, harmlessly).
    #[qslot]
    fn cancel(&mut self) {
        self.set_running(false);
    }
}

fn route_to_json(route: &[usize]) -> String {
    let nums: Vec<String> = route.iter().map(|id| id.to_string()).collect();
    format!("[{}]", nums.join(","))
}

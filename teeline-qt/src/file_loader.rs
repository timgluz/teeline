use std::path::Path;
use teeline::tsp::{kdtree::KDPoint, opt_tour, tour_cost, tsplib};
use qtbridge::qobject_impl;

pub struct FileLoader {
    city_count: i32,
    problem_name: String,
    edge_weight_type: String,
    is_loaded: bool,
    error_message: String,
    cities_json: String,
    recent_files_json: String,
    file_path: String,
    // Optimal tour state
    has_opt_tour: bool,
    opt_tour_route_json: String,
    opt_tour_cost: f32,
    opt_tour_file_path: String,
    // Private cache — not exposed as QML property
    cached_cities: Vec<KDPoint>,
}

impl Default for FileLoader {
    fn default() -> Self {
        Self {
            city_count: 0,
            problem_name: String::new(),
            edge_weight_type: String::new(),
            is_loaded: false,
            error_message: String::new(),
            cities_json: "[]".to_string(),
            recent_files_json: load_recent_files(),
            file_path: String::new(),
            has_opt_tour: false,
            opt_tour_route_json: "[]".to_string(),
            opt_tour_cost: 0.0,
            opt_tour_file_path: String::new(),
            cached_cities: Vec::new(),
        }
    }
}

#[qobject_impl(Singleton)]
impl FileLoader {
    qproperty!("cityCount",          Member = city_count,          Write = set_city_count,          Notify = "cityCountChanged");
    qproperty!("problemName",        Member = problem_name,        Write = set_problem_name,        Notify = "problemNameChanged");
    qproperty!("edgeWeightType",     Member = edge_weight_type,    Write = set_edge_weight_type,    Notify = "edgeWeightTypeChanged");
    qproperty!("isLoaded",           Member = is_loaded,           Write = set_is_loaded,           Notify = "isLoadedChanged");
    qproperty!("errorMessage",       Member = error_message,       Write = set_error_message,       Notify = "errorMessageChanged");
    qproperty!("citiesJson",         Member = cities_json,         Write = set_cities_json,         Notify = "citiesJsonChanged");
    qproperty!("recentFilesJson",    Member = recent_files_json,   Write = set_recent_files_json,   Notify = "recentFilesJsonChanged");
    qproperty!("filePath",           Member = file_path,           Write = set_file_path,           Notify = "filePathChanged");
    qproperty!("hasOptTour",         Member = has_opt_tour,        Write = set_has_opt_tour,        Notify = "hasOptTourChanged");
    qproperty!("optTourRouteJson",   Member = opt_tour_route_json, Write = set_opt_tour_route_json, Notify = "optTourRouteJsonChanged");
    qproperty!("optTourCost",        Member = opt_tour_cost,       Write = set_opt_tour_cost,       Notify = "optTourCostChanged");
    qproperty!("optTourFilePath",    Member = opt_tour_file_path,  Write = set_opt_tour_file_path,  Notify = "optTourFilePathChanged");

    fn set_city_count(&mut self, v: i32)          { self.city_count = v;          self.city_count_changed(); }
    fn set_problem_name(&mut self, v: String)      { self.problem_name = v;        self.problem_name_changed(); }
    fn set_edge_weight_type(&mut self, v: String)  { self.edge_weight_type = v;    self.edge_weight_type_changed(); }
    fn set_is_loaded(&mut self, v: bool)           { self.is_loaded = v;           self.is_loaded_changed(); }
    fn set_error_message(&mut self, v: String)     { self.error_message = v;       self.error_message_changed(); }
    fn set_cities_json(&mut self, v: String)       { self.cities_json = v;         self.cities_json_changed(); }
    fn set_recent_files_json(&mut self, v: String) { self.recent_files_json = v;   self.recent_files_json_changed(); }
    fn set_file_path(&mut self, v: String)         { self.file_path = v;           self.file_path_changed(); }
    fn set_has_opt_tour(&mut self, v: bool)        { self.has_opt_tour = v;        self.has_opt_tour_changed(); }
    fn set_opt_tour_route_json(&mut self, v: String) { self.opt_tour_route_json = v; self.opt_tour_route_json_changed(); }
    fn set_opt_tour_cost(&mut self, v: f32)        { self.opt_tour_cost = v;       self.opt_tour_cost_changed(); }
    fn set_opt_tour_file_path(&mut self, v: String){ self.opt_tour_file_path = v;  self.opt_tour_file_path_changed(); }

    #[qsignal] fn city_count_changed(&self);
    #[qsignal] fn problem_name_changed(&self);
    #[qsignal] fn edge_weight_type_changed(&self);
    #[qsignal] fn is_loaded_changed(&self);
    #[qsignal] fn error_message_changed(&self);
    #[qsignal] fn cities_json_changed(&self);
    #[qsignal] fn recent_files_json_changed(&self);
    #[qsignal] fn file_path_changed(&self);
    #[qsignal] fn has_opt_tour_changed(&self);
    #[qsignal] fn opt_tour_route_json_changed(&self);
    #[qsignal] fn opt_tour_cost_changed(&self);
    #[qsignal] fn opt_tour_file_path_changed(&self);

    /// Load a TSPLIB file. `path` may be a filesystem path or a file:// URL.
    #[qslot]
    fn load_file(&mut self, path: String) {
        let clean = path
            .strip_prefix("file://")
            .unwrap_or(&path)
            .to_string();

        // Reset opt-tour state whenever a new problem is loaded
        self.cached_cities = Vec::new();
        self.set_has_opt_tour(false);
        self.set_opt_tour_route_json("[]".to_string());
        self.set_opt_tour_cost(0.0);
        self.set_opt_tour_file_path(String::new());

        eprintln!("[FileLoader] loading: {clean}");
        match tsplib::read_from_file(Path::new(&clean)) {
            Ok(data) => {
                let ew_type = if data.has_explicit_weights() { "EXPLICIT" } else { "EUC_2D" };
                self.cached_cities = data.cities().to_vec();
                self.set_problem_name(data.name.clone());
                self.set_city_count(data.len() as i32);
                self.set_edge_weight_type(ew_type.to_string());
                self.set_cities_json(cities_to_json(data.cities()));
                self.set_error_message(String::new());
                self.set_is_loaded(true);
                self.set_file_path(clean.clone());

                let updated = push_recent(clean.clone(), &self.recent_files_json);
                save_recent_files(&updated);
                self.set_recent_files_json(updated);

                // Auto-pair: load sibling .opt.tour if it exists
                if let (Some(parent), Some(stem)) = (
                    Path::new(&clean).parent(),
                    Path::new(&clean).file_stem(),
                ) {
                    let opt_path = parent.join(format!("{}.opt.tour", stem.to_string_lossy()));
                    if opt_path.exists() {
                        self.load_opt_tour(opt_path.to_string_lossy().into_owned());
                    }
                }
            }
            Err(e) => {
                self.set_error_message(e);
                self.set_is_loaded(false);
            }
        }
    }

    /// Load a known-optimal .opt.tour file and compute its cost against the loaded problem.
    #[qslot]
    fn load_opt_tour(&mut self, path: String) {
        let clean = path
            .strip_prefix("file://")
            .unwrap_or(&path)
            .to_string();

        if self.cached_cities.is_empty() {
            self.set_error_message("Load a .tsp file before loading an optimal tour".to_string());
            return;
        }

        match opt_tour::read_from_file(Path::new(&clean)) {
            Ok(opt) => {
                // Validate: dimension and city IDs must match the loaded problem
                if opt.route.len() != self.cached_cities.len() {
                    self.set_error_message(format!(
                        "opt.tour has {} cities but problem has {}",
                        opt.route.len(), self.cached_cities.len()
                    ));
                    return;
                }
                let valid_ids: std::collections::HashSet<usize> =
                    self.cached_cities.iter().map(|c| c.id).collect();
                if opt.route.iter().any(|id| !valid_ids.contains(id)) {
                    self.set_error_message("opt.tour contains city IDs not in the loaded problem".to_string());
                    return;
                }

                let cost = tour_cost(&opt.route, &self.cached_cities);
                self.set_opt_tour_route_json(route_to_json(&opt.route));
                self.set_opt_tour_cost(cost);
                self.set_opt_tour_file_path(clean);
                self.set_has_opt_tour(true);
                self.set_error_message(String::new());
            }
            Err(e) => {
                self.set_error_message(format!("opt.tour: {e}"));
            }
        }
    }
}

// ── Recent files persistence ──────────────────────────────────────────────

fn recent_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home)
        .join(".config")
        .join("teeline-qt")
        .join("recent.json")
}

fn load_recent_files() -> String {
    std::fs::read_to_string(recent_path()).unwrap_or_else(|_| "[]".to_string())
}

fn save_recent_files(json: &str) {
    let path = recent_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, json);
}

/// Push `path` to the front of the JSON recent-files list (max 10 entries).
fn push_recent(path: String, current_json: &str) -> String {
    let mut list: Vec<String> = serde_json::from_str(current_json).unwrap_or_default();
    list.retain(|p| p != &path);
    list.insert(0, path);
    list.truncate(10);
    serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string())
}

// ── Route / city serialisation ───────────────────────────────────────────

fn route_to_json(route: &[usize]) -> String {
    let nums: Vec<String> = route.iter().map(|id| id.to_string()).collect();
    format!("[{}]", nums.join(","))
}

// ── City serialisation ────────────────────────────────────────────────────

fn cities_to_json(cities: &[KDPoint]) -> String {
    if cities.is_empty() {
        return "[]".to_string();
    }
    let min_x = cities.iter().map(|c| c.x()).fold(f32::INFINITY, f32::min);
    let max_x = cities.iter().map(|c| c.x()).fold(f32::NEG_INFINITY, f32::max);
    let min_y = cities.iter().map(|c| c.y()).fold(f32::INFINITY, f32::min);
    let max_y = cities.iter().map(|c| c.y()).fold(f32::NEG_INFINITY, f32::max);
    let rx = (max_x - min_x).max(1.0);
    let ry = (max_y - min_y).max(1.0);

    let pts: Vec<String> = cities
        .iter()
        .map(|c| {
            let nx = (c.x() - min_x) / rx;
            let ny = (c.y() - min_y) / ry;
            format!("{{\"id\":{},\"x\":{:.4},\"y\":{:.4}}}", c.id, nx, ny)
        })
        .collect();
    format!("[{}]", pts.join(","))
}

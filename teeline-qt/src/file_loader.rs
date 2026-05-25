use std::path::Path;
use teeline::tsp::{kdtree::KDPoint, tsplib};
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
        }
    }
}

#[qobject_impl(Singleton)]
impl FileLoader {
    qproperty!("cityCount",        Member = city_count,        Write = set_city_count,        Notify = "cityCountChanged");
    qproperty!("problemName",      Member = problem_name,      Write = set_problem_name,      Notify = "problemNameChanged");
    qproperty!("edgeWeightType",   Member = edge_weight_type,  Write = set_edge_weight_type,  Notify = "edgeWeightTypeChanged");
    qproperty!("isLoaded",         Member = is_loaded,         Write = set_is_loaded,         Notify = "isLoadedChanged");
    qproperty!("errorMessage",     Member = error_message,     Write = set_error_message,     Notify = "errorMessageChanged");
    qproperty!("citiesJson",       Member = cities_json,       Write = set_cities_json,       Notify = "citiesJsonChanged");
    qproperty!("recentFilesJson",  Member = recent_files_json, Write = set_recent_files_json, Notify = "recentFilesJsonChanged");
    qproperty!("filePath",         Member = file_path,         Write = set_file_path,         Notify = "filePathChanged");

    fn set_city_count(&mut self, v: i32)       { self.city_count = v;        self.city_count_changed(); }
    fn set_problem_name(&mut self, v: String)   { self.problem_name = v;      self.problem_name_changed(); }
    fn set_edge_weight_type(&mut self, v: String) { self.edge_weight_type = v; self.edge_weight_type_changed(); }
    fn set_is_loaded(&mut self, v: bool)        { self.is_loaded = v;         self.is_loaded_changed(); }
    fn set_error_message(&mut self, v: String)  { self.error_message = v;     self.error_message_changed(); }
    fn set_cities_json(&mut self, v: String)    { self.cities_json = v;       self.cities_json_changed(); }
    fn set_recent_files_json(&mut self, v: String) { self.recent_files_json = v; self.recent_files_json_changed(); }
    fn set_file_path(&mut self, v: String)      { self.file_path = v;         self.file_path_changed(); }

    #[qsignal] fn city_count_changed(&self);
    #[qsignal] fn problem_name_changed(&self);
    #[qsignal] fn edge_weight_type_changed(&self);
    #[qsignal] fn is_loaded_changed(&self);
    #[qsignal] fn error_message_changed(&self);
    #[qsignal] fn cities_json_changed(&self);
    #[qsignal] fn recent_files_json_changed(&self);
    #[qsignal] fn file_path_changed(&self);

    /// Load a TSPLIB file. `path` may be a filesystem path or a file:// URL.
    #[qslot]
    fn load_file(&mut self, path: String) {
        let clean = path
            .strip_prefix("file://")
            .unwrap_or(&path)
            .to_string();

        eprintln!("[FileLoader] loading: {clean}");
        match tsplib::read_from_file(Path::new(&clean)) {
            Ok(data) => {
                let ew_type = if data.has_explicit_weights() { "EXPLICIT" } else { "EUC_2D" };
                self.set_problem_name(data.name.clone());
                self.set_city_count(data.len() as i32);
                self.set_edge_weight_type(ew_type.to_string());
                self.set_cities_json(cities_to_json(data.cities()));
                self.set_error_message(String::new());
                self.set_is_loaded(true);
                self.set_file_path(clean.clone());

                let updated = push_recent(clean, &self.recent_files_json);
                save_recent_files(&updated);
                self.set_recent_files_json(updated);
            }
            Err(e) => {
                self.set_error_message(e);
                self.set_is_loaded(false);
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

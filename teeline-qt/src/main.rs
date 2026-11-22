mod file_loader;
mod solver_engine;

use file_loader::FileLoader;
use qtbridge::{QApp, qobject};
use solver_engine::SolverEngine;

#[derive(Default)]
pub struct AppBackend {}

#[qobject(Singleton, ConvertToCamelCase)]
impl AppBackend {}

fn main() {
    QApp::new()
        .register::<AppBackend>()
        .register::<FileLoader>()
        .register::<SolverEngine>()
        .load_qml(include_bytes!("Main.qml"))
        .run();
}

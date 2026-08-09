mod file_loader;
mod solver_engine;

use file_loader::FileLoader;
use qtbridge::{QApp, qobject_impl};
use solver_engine::SolverEngine;

#[derive(Default)]
pub struct AppBackend {}

#[qobject_impl(Singleton)]
impl AppBackend {}

fn main() {
    QApp::new()
        .register::<AppBackend>()
        .register::<FileLoader>()
        .register::<SolverEngine>()
        .load_qml(include_bytes!("Main.qml"))
        .run();
}

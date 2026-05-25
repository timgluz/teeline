mod file_loader;

use file_loader::FileLoader;
use qtbridge::{qobject_impl, QApp};

#[derive(Default)]
pub struct AppBackend {}

#[qobject_impl(Singleton)]
impl AppBackend {}

fn main() {
    QApp::new()
        .register::<AppBackend>()
        .register::<FileLoader>()
        .load_qml(include_bytes!("Main.qml"))
        .run();
}

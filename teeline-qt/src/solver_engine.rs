use qtbridge::qobject_impl;

pub struct SolverEngine {
    selected_solver: String,
}

impl Default for SolverEngine {
    fn default() -> Self {
        Self {
            selected_solver: String::new(),
        }
    }
}

#[qobject_impl(Singleton)]
impl SolverEngine {
    qproperty!("selectedSolver", Member = selected_solver, Write = set_selected_solver, Notify = "selectedSolverChanged");

    fn set_selected_solver(&mut self, v: String) {
        self.selected_solver = v;
        self.selected_solver_changed();
    }

    #[qsignal]
    fn selected_solver_changed(&self);

    /// Called from QML to select a solver by alias.
    #[qslot]
    fn select_solver(&mut self, alias: String) {
        self.set_selected_solver(alias);
    }
}

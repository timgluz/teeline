use teeline::tsp::SolverOptions;

#[test]
fn solver_options_default_no_gui() {
    let opts = SolverOptions::default();
    assert_eq!(opts.epochs, 10_000);
}

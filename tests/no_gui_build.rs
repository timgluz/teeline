use teeline::tsp::{AppOptions, HeuristicOptions};

#[test]
fn heuristic_options_default_no_gui() {
    let defaults = HeuristicOptions::default();
    assert_eq!(defaults.epochs, 10_000);
    let _ = AppOptions::default();
}

/// Integration tests for the Greedy Edge Construction TSP solver.
use std::path::Path;
use teeline::tsp::{
    AppOptions, HeuristicOptions, Solvers, TspProblem, distance_matrix, greedy_edge, kdtree,
    pipeline, tsplib,
};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn load_tsp(fixture: &str) -> tsplib::TspLibData {
    let path = Path::new("tests/fixtures").join(fixture);
    tsplib::read_from_file(&path).unwrap_or_else(|e| panic!("failed to read {fixture}: {e}"))
}

fn make_problem(cities: Vec<kdtree::KDPoint>) -> TspProblem {
    let dm = distance_matrix::from_cities(&cities);
    TspProblem::new(cities, dm)
}

fn is_valid_tour(route: &[usize], cities: &[kdtree::KDPoint]) -> bool {
    let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
    expected.sort_unstable();
    let mut got = route.to_vec();
    got.sort_unstable();
    got == expected
}

// ─── fast structural tests ────────────────────────────────────────────────────

#[test]
fn greedy_edge_berlin52_returns_valid_tour() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let sol = greedy_edge::solve(&problem, &HeuristicOptions::default(), None, None);
    assert_eq!(sol.route().len(), 52, "tour must visit all 52 cities");
    assert!(
        is_valid_tour(sol.route(), &cities),
        "tour must contain every city exactly once"
    );
}

#[test]
fn greedy_edge_berlin52_reported_distance_matches_tour() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let dm = distance_matrix::from_cities(&cities);
    let sol = greedy_edge::solve(&problem, &HeuristicOptions::default(), None, None);

    let route = sol.route();
    let n = route.len();
    let recomputed: f32 = (0..n)
        .map(|i| {
            dm.distance_between(route[i], route[(i + 1) % n])
                .unwrap_or(f32::MAX)
        })
        .sum();

    assert!(
        (sol.total - recomputed).abs() < 1.0,
        "reported total ({:.1}) must match recomputed tour length ({:.1})",
        sol.total,
        recomputed,
    );
}

/// Empirical quality floor. Known optimum for berlin52 = 7542. Greedy edge
/// construction is deterministic and, measured directly, produces ~9954 on
/// berlin52 (~32% above optimal — worse than the general ~14-25% literature
/// range, since berlin52's layout includes several isolated points that force
/// expensive closing edges). Assert a ceiling ~10% above the measured value to
/// catch regressions without depending on random variation.
#[test]
fn greedy_edge_berlin52_empirical_quality() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities);
    let sol = greedy_edge::solve(&problem, &HeuristicOptions::default(), None, None);
    assert!(
        sol.total <= 11000.0,
        "greedy_edge empirical quality regression: got {:.1}, want ≤11000 (measured ~9954)",
        sol.total,
    );
}

/// Proves the pipeline-seed value proposition from the algorithm's design brief:
/// greedy_edge should be at least as good a seed for two_opt as it is alone.
#[test]
fn greedy_edge_pipeline_as_seed_for_two_opt() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();

    let solo_problem = make_problem(cities.clone());
    let solo = greedy_edge::solve(&solo_problem, &HeuristicOptions::default(), None, None);

    let piped_problem = make_problem(cities);
    let stages = [
        pipeline::PipelineStage::new(
            Solvers::GreedyEdge,
            AppOptions::default(),
            piped_problem.clone(),
            None,
        ),
        pipeline::PipelineStage::new(Solvers::TwoOpt, AppOptions::default(), piped_problem, None),
    ];
    let piped = pipeline::run_pipeline(&stages).unwrap();

    assert!(
        piped.total <= solo.total,
        "greedy_edge -> two_opt ({:.1}) must be no worse than greedy_edge alone ({:.1})",
        piped.total,
        solo.total,
    );
}

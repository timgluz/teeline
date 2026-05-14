/// Integration tests for all TSP solvers.
///
/// Each test loads a real TSPLIB instance and verifies:
///   1. The tour is valid (every city visited exactly once).
///   2. For deterministic solvers (NN, 2-opt): tour length is within a reasonable bound.
///
/// Known optima used as reference:
///   berlin52 – 7542  (EUC_2D)
///
/// Stochastic solvers (SA, hill climbing, tabu, GA) are only checked for tour validity
/// because solution quality depends on runtime epochs.
use std::path::Path;
use teeline::tsp::{bellman_karp, branch_bound, genetic_algorithm, kdtree, nearest_neighbor,
                   simulated_annealing, stochastic_hill, tabu_search, two_opt, tsplib,
                   SolverOptions};

// ─── helpers ──────────────────────────────────────────────────────────────────

fn load_berlin52() -> Vec<kdtree::KDPoint> {
    let path = Path::new("tests/fixtures/berlin52.tsp");
    tsplib::read_from_file(path)
        .expect("failed to read berlin52.tsp")
        .cities()
        .to_vec()
}

fn tsp5_cities() -> Vec<kdtree::KDPoint> {
    kdtree::build_points(&[
        vec![0.0, 0.0],
        vec![0.0, 0.5],
        vec![0.0, 1.0],
        vec![1.0, 1.0],
        vec![1.0, 0.0],
    ])
}

/// Returns a sorted list of city IDs that must appear exactly once in a valid tour.
fn expected_ids(cities: &[kdtree::KDPoint]) -> Vec<usize> {
    let mut ids: Vec<usize> = cities.iter().map(|c| c.id).collect();
    ids.sort();
    ids
}

fn is_valid_tour(tour_route: &[usize], cities: &[kdtree::KDPoint]) -> bool {
    let mut visited = tour_route.to_vec();
    visited.sort();
    visited == expected_ids(cities)
}

fn stochastic_options(epochs: usize) -> SolverOptions {
    let mut opts = SolverOptions::default();
    opts.epochs = epochs;
    opts.platoo_epochs = epochs / 5;
    opts
}

// ─── nearest neighbor ────────────────────────────────────────────────────────

#[test]
fn nearest_neighbor_valid_tour_berlin52() {
    let cities = load_berlin52();
    let tour = nearest_neighbor::solve(&cities, &SolverOptions::default());

    assert!(is_valid_tour(tour.route(), &cities), "NN tour is not valid on berlin52");
}

#[test]
fn nearest_neighbor_tour_is_finite_and_positive_berlin52() {
    let cities = load_berlin52();
    let tour = nearest_neighbor::solve(&cities, &SolverOptions::default());

    assert!(tour.total > 0.0, "NN tour length should be positive");
    assert!(tour.total.is_finite(), "NN tour length should be finite");
}

// ─── 2-opt ───────────────────────────────────────────────────────────────────

#[test]
fn two_opt_valid_tour_berlin52() {
    let cities = load_berlin52();
    let tour = two_opt::solve(&cities, &SolverOptions::default());

    assert!(is_valid_tour(tour.route(), &cities), "2-opt tour is not valid on berlin52");
}

#[test]
fn two_opt_tour_within_50pct_of_optimal_berlin52() {
    let cities = load_berlin52();
    let tour = two_opt::solve(&cities, &SolverOptions::default());

    assert!(
        tour.total < 7542.0 * 1.5,
        "2-opt tour {} is worse than 1.5× optimal on berlin52",
        tour.total
    );
}

// ─── tabu search ─────────────────────────────────────────────────────────────

#[test]
fn tabu_search_valid_tour_berlin52() {
    let cities = load_berlin52();
    let tour = tabu_search::solve(&cities, &stochastic_options(500));

    assert!(is_valid_tour(tour.route(), &cities), "tabu tour is not valid on berlin52");
}

// ─── stochastic hill climbing ─────────────────────────────────────────────────

#[test]
fn stochastic_hill_valid_tour_berlin52() {
    let cities = load_berlin52();
    let tour = stochastic_hill::solve(&cities, &stochastic_options(500));

    assert!(is_valid_tour(tour.route(), &cities), "hill tour is not valid on berlin52");
}

// ─── simulated annealing ──────────────────────────────────────────────────────

#[test]
fn simulated_annealing_valid_tour_berlin52() {
    let cities = load_berlin52();
    let mut opts = SolverOptions::default();
    opts.epochs = 2000;
    let tour = simulated_annealing::solve(&cities, &opts);

    assert!(is_valid_tour(tour.route(), &cities), "SA tour is not valid on berlin52");
}

// ─── genetic algorithm ────────────────────────────────────────────────────────

#[test]
fn genetic_algorithm_valid_tour_berlin52() {
    let cities = load_berlin52();
    let mut opts = SolverOptions::default();
    opts.epochs = 100;
    let tour = genetic_algorithm::solve(&cities, &opts);

    assert!(is_valid_tour(tour.route(), &cities), "GA tour is not valid on berlin52");
}

// ─── exact solvers (small instance only) ─────────────────────────────────────

#[test]
fn bellman_karp_optimal_tour_tsp5() {
    let cities = tsp5_cities();
    let tour = bellman_karp::solve(&cities, &SolverOptions::default());

    assert!(is_valid_tour(tour.route(), &cities), "BHK tour is not valid");
    // known optimal: 0→1→2→3→4→0 = 4.0
    assert!(
        (tour.total - 4.0).abs() < 1e-3,
        "BHK should find optimal tour ~4.0, got {}",
        tour.total
    );
}

#[test]
fn branch_bound_optimal_tour_tsp5() {
    let cities = tsp5_cities();
    let tour = branch_bound::solve(&cities, &SolverOptions::default());

    assert!(is_valid_tour(tour.route(), &cities), "B&B tour is not valid");
    assert!(
        (tour.total - 4.0).abs() < 1e-3,
        "B&B should find optimal tour ~4.0, got {}",
        tour.total
    );
}

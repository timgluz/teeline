/// Integration tests for all TSP solvers.
///
/// Each test loads a real TSPLIB instance and verifies:
///   1. The tour is valid (every city visited exactly once).
///   2. For deterministic solvers (NN, 2-opt): tour length is within a reasonable bound.
///
/// Known optima used as reference:
///   berlin52 – 7542  (EUC_2D)
///   gr17     – 2085  (LOWER_DIAG_ROW explicit matrix)
///
/// Stochastic solvers (SA, hill climbing, tabu, GA) are only checked for tour validity
/// because solution quality depends on runtime epochs.
use std::path::Path;
use teeline::tsp::{
    CSOptions, FPAOptions, GAOptions, HeuristicOptions, SAOptions, TspProblem, bellman_karp,
    branch_bound, cuckoo_search, distance_matrix, flower_pollination, genetic_algorithm,
    gravitational_search, kdtree, nearest_neighbor, particle_swarm, simulated_annealing,
    stochastic_hill, tabu_search, tsplib, two_opt,
};

// ─── helpers ──────────────────────────────────────────────────────────────────

fn load_tsp(fixture: &str) -> tsplib::TspLibData {
    let path = Path::new("tests/fixtures").join(fixture);
    tsplib::read_from_file(&path).unwrap_or_else(|e| panic!("failed to read {fixture}: {e}"))
}

fn load_berlin52() -> Vec<kdtree::KDPoint> {
    load_tsp("berlin52.tsp").cities().to_vec()
}

fn build_dm(cities: &[kdtree::KDPoint]) -> distance_matrix::DistanceMatrix {
    distance_matrix::from_cities(cities)
}

fn make_problem(cities: &[kdtree::KDPoint]) -> TspProblem {
    TspProblem::new(cities.to_vec(), build_dm(cities))
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

fn stochastic_options(epochs: usize) -> HeuristicOptions {
    HeuristicOptions {
        epochs,
        platoo_epochs: epochs / 5,
        ..HeuristicOptions::default()
    }
}

// ─── nearest neighbor ────────────────────────────────────────────────────────

#[test]
fn nearest_neighbor_valid_tour_berlin52() {
    let cities = load_berlin52();
    let tour = nearest_neighbor::solve(
        &make_problem(&cities),
        &HeuristicOptions::default(),
        None,
        None,
    );

    assert!(
        is_valid_tour(tour.route(), &cities),
        "NN tour is not valid on berlin52"
    );
}

#[test]
fn nearest_neighbor_tour_is_finite_and_positive_berlin52() {
    let cities = load_berlin52();
    let tour = nearest_neighbor::solve(
        &make_problem(&cities),
        &HeuristicOptions::default(),
        None,
        None,
    );

    assert!(tour.total > 0.0, "NN tour length should be positive");
    assert!(tour.total.is_finite(), "NN tour length should be finite");
}

// ─── 2-opt ───────────────────────────────────────────────────────────────────

#[test]
fn two_opt_valid_tour_berlin52() {
    let cities = load_berlin52();
    let tour = two_opt::solve(
        &make_problem(&cities),
        &HeuristicOptions::default(),
        None,
        None,
    );

    assert!(
        is_valid_tour(tour.route(), &cities),
        "2-opt tour is not valid on berlin52"
    );
}

#[test]
fn two_opt_tour_within_50pct_of_optimal_berlin52() {
    let cities = load_berlin52();
    let tour = two_opt::solve(
        &make_problem(&cities),
        &HeuristicOptions::default(),
        None,
        None,
    );

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
    let tour = tabu_search::solve(&make_problem(&cities), &stochastic_options(500), None, None);

    assert!(
        is_valid_tour(tour.route(), &cities),
        "tabu tour is not valid on berlin52"
    );
}

// ─── stochastic hill climbing ─────────────────────────────────────────────────

#[test]
fn stochastic_hill_valid_tour_berlin52() {
    let cities = load_berlin52();
    let tour = stochastic_hill::solve(&make_problem(&cities), &stochastic_options(500), None, None);

    assert!(
        is_valid_tour(tour.route(), &cities),
        "hill tour is not valid on berlin52"
    );
}

// ─── simulated annealing ──────────────────────────────────────────────────────

#[test]
fn simulated_annealing_valid_tour_berlin52() {
    let cities = load_berlin52();
    let opts = SAOptions {
        heuristic: HeuristicOptions {
            epochs: 2000,
            ..HeuristicOptions::default()
        },
        ..SAOptions::default()
    };
    let tour = simulated_annealing::solve(&make_problem(&cities), &opts, None, None);

    assert!(
        is_valid_tour(tour.route(), &cities),
        "SA tour is not valid on berlin52"
    );
}

// ─── genetic algorithm ────────────────────────────────────────────────────────

#[test]
fn genetic_algorithm_valid_tour_berlin52() {
    let cities = load_berlin52();
    let opts = GAOptions {
        heuristic: HeuristicOptions {
            epochs: 100,
            ..HeuristicOptions::default()
        },
        ..GAOptions::default()
    };
    let tour = genetic_algorithm::solve(&make_problem(&cities), &opts, None, None);

    assert!(
        is_valid_tour(tour.route(), &cities),
        "GA tour is not valid on berlin52"
    );
}

// ─── particle swarm optimisation ─────────────────────────────────────────────

#[test]
fn particle_swarm_valid_tour_berlin52() {
    let cities = load_berlin52();
    let tour = particle_swarm::solve(&make_problem(&cities), &stochastic_options(200), None, None);

    assert!(
        is_valid_tour(tour.route(), &cities),
        "PSO tour is not valid on berlin52"
    );
}

// ─── gravitational search ─────────────────────────────────────────────────────

#[test]
fn gravitational_search_valid_tour_berlin52() {
    let cities = load_berlin52();
    let tour =
        gravitational_search::solve(&make_problem(&cities), &stochastic_options(200), None, None);
    assert!(
        is_valid_tour(tour.route(), &cities),
        "GSA tour is not valid on berlin52"
    );
}

#[test]
fn gravitational_search_valid_tour_small() {
    let cities = tsp5_cities();
    let tour =
        gravitational_search::solve(&make_problem(&cities), &stochastic_options(50), None, None);
    assert!(
        is_valid_tour(tour.route(), &cities),
        "GSA tour is not valid on 5-city instance"
    );
}

// ─── cuckoo search ────────────────────────────────────────────────────────────

#[test]
fn cuckoo_search_valid_tour_berlin52() {
    let cities = load_berlin52();
    let opts = CSOptions {
        heuristic: HeuristicOptions {
            epochs: 200,
            ..HeuristicOptions::default()
        },
        mutation_probability: 0.25,
        ..CSOptions::default()
    };
    let tour = cuckoo_search::solve(&make_problem(&cities), &opts, None, None);
    assert!(
        is_valid_tour(tour.route(), &cities),
        "Cuckoo Search tour is not valid on berlin52"
    );
}

// ─── exact solvers (small instance only) ─────────────────────────────────────

#[test]
fn bellman_karp_optimal_tour_tsp5() {
    let cities = tsp5_cities();
    let tour = bellman_karp::solve(
        &make_problem(&cities),
        &HeuristicOptions::default(),
        None,
        None,
    );

    assert!(
        is_valid_tour(tour.route(), &cities),
        "BHK tour is not valid"
    );
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
    let tour = branch_bound::solve(
        &make_problem(&cities),
        &HeuristicOptions::default(),
        None,
        None,
    );

    assert!(
        is_valid_tour(tour.route(), &cities),
        "B&B tour is not valid"
    );
    assert!(
        (tour.total - 4.0).abs() < 1e-3,
        "B&B should find optimal tour ~4.0, got {}",
        tour.total
    );
}

// ─── explicit distance matrix (TSPLIB EDGE_WEIGHT_SECTION) ───────────────────

#[test]
fn parser_loads_gr17_lower_diag_row() {
    let data = load_tsp("gr17.tsp");
    assert!(
        data.has_explicit_weights(),
        "gr17 should have explicit weights"
    );
    assert_eq!(data.cities().len(), 17, "gr17 has 17 cities");
}

#[test]
fn parser_loads_bayg29_upper_row() {
    let data = load_tsp("bayg29.tsp");
    assert!(
        data.has_explicit_weights(),
        "bayg29 should have explicit weights"
    );
    assert_eq!(data.cities().len(), 29, "bayg29 has 29 cities");
}

#[test]
fn parser_loads_bays29_full_matrix() {
    let data = load_tsp("bays29.tsp");
    assert!(
        data.has_explicit_weights(),
        "bays29 should have explicit weights"
    );
    assert_eq!(data.cities().len(), 29, "bays29 has 29 cities");
}

#[test]
fn nn_valid_tour_gr17() {
    let data = load_tsp("gr17.tsp");
    let cities = data.cities().to_vec();
    let dm = data.distance_matrix().expect("gr17 distance matrix");
    let problem = TspProblem::new(cities.clone(), dm);
    let tour = nearest_neighbor::solve(&problem, &HeuristicOptions::default(), None, None);

    assert!(
        is_valid_tour(tour.route(), &cities),
        "NN tour is not valid on gr17"
    );
    assert!(tour.total > 0.0);
}

#[test]
fn nn_valid_tour_bays29() {
    let data = load_tsp("bays29.tsp");
    let cities = data.cities().to_vec();
    let dm = data.distance_matrix().expect("bays29 distance matrix");
    let problem = TspProblem::new(cities.clone(), dm);
    let tour = nearest_neighbor::solve(&problem, &HeuristicOptions::default(), None, None);

    assert!(
        is_valid_tour(tour.route(), &cities),
        "NN tour is not valid on bays29"
    );
    assert!(tour.total > 0.0);
}

#[test]
fn two_opt_valid_tour_gr17() {
    let data = load_tsp("gr17.tsp");
    let cities = data.cities().to_vec();
    let dm = data.distance_matrix().expect("gr17 distance matrix");
    let problem = TspProblem::new(cities.clone(), dm);
    let tour = two_opt::solve(&problem, &HeuristicOptions::default(), None, None);

    assert!(
        is_valid_tour(tour.route(), &cities),
        "2-opt tour is not valid on gr17"
    );
    // optimal is 2085; allow 1.5×
    assert!(
        tour.total < 2085.0 * 1.5,
        "2-opt gr17 tour {} is worse than 1.5× optimal",
        tour.total
    );
}

// BHK on a 6-city ring with explicit FULL_MATRIX: ring edges cost 10, cross edges 100.
// Optimal tour visits all cities in ring order: total = 6 × 10 = 60.
#[test]
fn bellman_karp_optimal_ring6_explicit() {
    let data = load_tsp("ring6_explicit.tsp");
    let cities = data.cities().to_vec();
    let dm = data.distance_matrix().expect("ring6 distance matrix");
    let problem = TspProblem::new(cities.clone(), dm);
    let tour = bellman_karp::solve(&problem, &HeuristicOptions::default(), None, None);

    assert!(
        is_valid_tour(tour.route(), &cities),
        "BHK tour is not valid on ring6"
    );
    assert!(
        (tour.total - 60.0).abs() < 1.0,
        "BHK should find optimal 60 on ring6, got {}",
        tour.total
    );
}

#[test]
fn bellman_karp_optimal_gr17() {
    let data = load_tsp("gr17.tsp");
    let cities = data.cities().to_vec();
    let dm = data.distance_matrix().expect("gr17 distance matrix");
    let problem = TspProblem::new(cities.clone(), dm);
    let tour = bellman_karp::solve(&problem, &HeuristicOptions::default(), None, None);

    assert!(
        is_valid_tour(tour.route(), &cities),
        "BHK tour is not valid on gr17"
    );
    assert!(
        (tour.total - 2085.0).abs() < 1.0,
        "BHK should find optimal ~2085 on gr17, got {}",
        tour.total
    );
}

// ─── flower pollination algorithm ────────────────────────────────────────────

#[test]
fn flower_pollination_valid_tour_berlin52() {
    let cities = load_berlin52();
    let opts = FPAOptions {
        heuristic: HeuristicOptions {
            epochs: 200,
            ..HeuristicOptions::default()
        },
        mutation_probability: 0.8,
        ..FPAOptions::default()
    };
    let tour = flower_pollination::solve(&make_problem(&cities), &opts, None, None);
    assert!(
        is_valid_tour(tour.route(), &cities),
        "FPA tour invalid on berlin52"
    );
}

#[test]
fn flower_pollination_tour_is_finite_and_positive() {
    let cities = load_berlin52();
    let opts = FPAOptions {
        heuristic: HeuristicOptions {
            epochs: 200,
            ..HeuristicOptions::default()
        },
        mutation_probability: 0.8,
        ..FPAOptions::default()
    };
    let tour = flower_pollination::solve(&make_problem(&cities), &opts, None, None);
    assert!(tour.total > 0.0);
    assert!(tour.total.is_finite());
}

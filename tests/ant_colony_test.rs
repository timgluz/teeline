/// Integration tests for the Ant Colony Optimization (Ant System) TSP solver.
///
/// Uses an explicit `fast_opts()` helper rather than `AcoOptions::default()` — ACO's
/// per-epoch cost is O(ants * n^2), so even the solver's own (already-reduced) default
/// epoch count would make this test noticeably slower than necessary.
use std::path::Path;
use teeline::tsp::{AcoOptions, HeuristicOptions, TspProblem, distance_matrix, kdtree, tsplib};

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

fn fast_opts() -> AcoOptions {
    AcoOptions {
        heuristic: HeuristicOptions {
            epochs: 30,
            ..HeuristicOptions::default()
        },
        num_ants: 10,
        ..AcoOptions::default()
    }
}

// ─── fast structural tests ────────────────────────────────────────────────────

#[test]
fn ant_colony_valid_tour_berlin52() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let sol = teeline::tsp::ant_colony::solve(&problem, &fast_opts(), None, None);
    assert_eq!(sol.route().len(), 52, "tour must visit all 52 cities");
    assert!(
        is_valid_tour(sol.route(), &cities),
        "tour must contain every city exactly once"
    );
    assert!(sol.total > 0.0, "tour distance must be positive");
    assert!(sol.total.is_finite(), "tour distance must be finite");
}

#[test]
fn ant_colony_valid_tour_with_warm_start() {
    let cities = load_tsp("berlin52.tsp").cities().to_vec();
    let problem = make_problem(cities.clone());
    let nn_seed =
        teeline::tsp::nearest_neighbor::solve(&problem, &HeuristicOptions::default(), None, None);
    let sol = teeline::tsp::ant_colony::solve(&problem, &fast_opts(), None, Some(nn_seed.route()));
    assert!(
        is_valid_tour(sol.route(), &cities),
        "tour must contain every city exactly once"
    );
    assert!(sol.total > 0.0 && sol.total.is_finite());
}

#[test]
fn ant_colony_valid_tour_small() {
    // Cities with non-contiguous IDs to validate the position -> city ID mapping.
    let cities: Vec<kdtree::KDPoint> = vec![5usize, 10, 15, 20, 25]
        .into_iter()
        .enumerate()
        .map(|(i, id)| {
            use std::f32::consts::PI;
            kdtree::KDPoint {
                id,
                coords: [
                    (2.0 * PI * i as f32 / 5.0).cos(),
                    (2.0 * PI * i as f32 / 5.0).sin(),
                ],
            }
        })
        .collect();
    let problem = make_problem(cities.clone());
    let sol = teeline::tsp::ant_colony::solve(&problem, &fast_opts(), None, None);
    let mut got = sol.route().to_vec();
    got.sort_unstable();
    assert_eq!(
        got,
        vec![5, 10, 15, 20, 25],
        "tour must contain original city IDs [5,10,15,20,25], not array positions [0..4]"
    );
}

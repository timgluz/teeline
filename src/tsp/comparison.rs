use crate::tsp::DistanceType;
use crate::tsp::distance_matrix;
use crate::tsp::distance_matrix::DistanceMatrix;
use crate::tsp::kdtree::KDPoint;
use std::collections::{HashMap, HashSet};

pub struct ComparisonStats {
    pub optimal_cost: f32,
    pub solver_cost: f32,
    pub gap_pct: f32,
    pub shared_edges: usize,
    pub solver_only_edges: usize,
    pub optimal_only_edges: usize,
}

/// Sum of EUC_2D distances along the tour, closing the cycle (last→first).
/// City IDs are 1-based and are NOT array indices — uses HashMap lookup.
pub fn tour_cost(route: &[usize], cities: &[KDPoint]) -> f32 {
    tour_cost_with_type(route, cities, DistanceType::Euc2D)
}

/// Sum of distances along the tour using the given distance type.
pub fn tour_cost_with_type(route: &[usize], cities: &[KDPoint], dt: DistanceType) -> f32 {
    let idx: HashMap<usize, &KDPoint> = cities.iter().map(|c| (c.id, c)).collect();
    let n = route.len();
    (0..n)
        .map(|i| {
            let a = idx[&route[i]];
            let b = idx[&route[(i + 1) % n]];
            match dt {
                DistanceType::Euc2D => {
                    let dx = a.x() - b.x();
                    let dy = a.y() - b.y();
                    (dx * dx + dy * dy).sqrt()
                }
                DistanceType::Geo => distance_matrix::geo_distance(a, b),
                DistanceType::Explicit => {
                    panic!("tour_cost_with_type requires DistanceMatrix for EXPLICIT; use tour_cost_from_matrix instead")
                }
            }
        })
        .sum()
}

/// Sum of distances along the tour using a pre-computed distance matrix.
pub fn tour_cost_from_matrix(route: &[usize], dm: &DistanceMatrix) -> f32 {
    dm.tour_length(route)
}

pub fn compare_tours(solver: &[usize], optimal: &[usize], cities: &[KDPoint]) -> ComparisonStats {
    compare_tours_with_type(solver, optimal, cities, DistanceType::Euc2D)
}

pub fn compare_tours_with_type(
    solver: &[usize],
    optimal: &[usize],
    cities: &[KDPoint],
    dt: DistanceType,
) -> ComparisonStats {
    let optimal_cost = tour_cost_with_type(optimal, cities, dt);
    let solver_cost = tour_cost_with_type(solver, cities, dt);
    build_stats(solver_cost, optimal_cost, solver, optimal)
}

pub fn compare_tours_from_matrix(
    solver: &[usize],
    optimal: &[usize],
    dm: &DistanceMatrix,
) -> ComparisonStats {
    let optimal_cost = tour_cost_from_matrix(optimal, dm);
    let solver_cost = tour_cost_from_matrix(solver, dm);
    build_stats(solver_cost, optimal_cost, solver, optimal)
}

fn build_stats(
    solver_cost: f32,
    optimal_cost: f32,
    solver: &[usize],
    optimal: &[usize],
) -> ComparisonStats {
    let gap_pct = if optimal_cost > 0.0 {
        (solver_cost - optimal_cost) / optimal_cost * 100.0
    } else {
        0.0
    };

    let solver_edges = edge_set(solver);
    let optimal_edges = edge_set(optimal);

    let shared_edges = solver_edges.intersection(&optimal_edges).count();
    let solver_only_edges = solver_edges.difference(&optimal_edges).count();
    let optimal_only_edges = optimal_edges.difference(&solver_edges).count();

    ComparisonStats {
        optimal_cost,
        solver_cost,
        gap_pct,
        shared_edges,
        solver_only_edges,
        optimal_only_edges,
    }
}

/// Build a set of undirected edges: each edge stored as (min_id, max_id).
/// Includes the closing edge from last city back to first.
fn edge_set(route: &[usize]) -> HashSet<(usize, usize)> {
    let n = route.len();
    (0..n)
        .map(|i| {
            let a = route[i];
            let b = route[(i + 1) % n];
            (a.min(b), a.max(b))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn square_cities() -> Vec<KDPoint> {
        vec![
            KDPoint {
                id: 1,
                coords: [0.0, 0.0],
            },
            KDPoint {
                id: 2,
                coords: [1.0, 0.0],
            },
            KDPoint {
                id: 3,
                coords: [1.0, 1.0],
            },
            KDPoint {
                id: 4,
                coords: [0.0, 1.0],
            },
        ]
    }

    #[test]
    fn compare_identical_tours() {
        let cities = square_cities();
        let route = vec![1, 2, 3, 4];
        let stats = compare_tours(&route, &route, &cities);
        assert_eq!(stats.gap_pct, 0.0);
        assert_eq!(stats.shared_edges, 4);
        assert_eq!(stats.solver_only_edges, 0);
        assert_eq!(stats.optimal_only_edges, 0);
        assert_eq!(stats.solver_cost, stats.optimal_cost);
    }

    #[test]
    fn compare_single_swap() {
        // Square: 1=(0,0), 2=(1,0), 3=(1,1), 4=(0,1)
        // Optimal: [1,2,3,4] — edges {1-2, 2-3, 3-4, 1-4} — perimeter = 4
        // Solver:  [1,2,4,3] — edges {1-2, 2-4, 3-4, 1-3} — crosses diagonals = 2+2√2
        let cities = square_cities();
        let optimal = vec![1, 2, 3, 4];
        let solver = vec![1, 2, 4, 3];
        let stats = compare_tours(&solver, &optimal, &cities);

        assert!(
            stats.gap_pct > 0.0,
            "solver has a crossing edge, must be worse"
        );
        assert_eq!(stats.shared_edges, 2, "edges (1-2) and (3-4) are shared");
        assert_eq!(
            stats.solver_only_edges, 2,
            "solver has extra (2-4) and (1-3)"
        );
        assert_eq!(
            stats.optimal_only_edges, 2,
            "optimal has (2-3) and (1-4) not in solver"
        );
        // Invariant: shared + solver_only == n
        assert_eq!(stats.shared_edges + stats.solver_only_edges, optimal.len());
    }

    #[test]
    fn compare_berlin52_optimal_against_itself() {
        use crate::tsp::{opt_tour, tsplib};

        let tsp = tsplib::read_from_file(Path::new("tests/fixtures/berlin52.tsp"))
            .expect("berlin52.tsp must exist in tests/fixtures/");
        let opt = opt_tour::read_from_file(Path::new("tests/fixtures/berlin52.opt.tour"))
            .expect("berlin52.opt.tour must exist in tests/fixtures/");
        let cities = tsp.cities();

        // Same route as both solver and optimal: gap must be exactly 0
        let stats = compare_tours(&opt.route, &opt.route, cities);
        assert_eq!(stats.gap_pct, 0.0);
        assert_eq!(stats.shared_edges, opt.route.len());
        assert_eq!(stats.solver_only_edges, 0);

        // Sorted-by-id route is a different (worse) tour for Berlin52
        let mut sorted_route: Vec<usize> = opt.route.clone();
        sorted_route.sort_unstable();
        let stats2 = compare_tours(&sorted_route, &opt.route, cities);
        assert!(
            stats2.gap_pct > 0.0,
            "ID-sorted route must be worse than the known optimal"
        );
    }

    #[test]
    fn tour_cost_from_matrix_ring6() {
        let input = "NAME: ring6\nDIMENSION: 6\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: FULL_MATRIX\nEDGE_WEIGHT_SECTION\n0 10 100 100 100 10\n10 0 10 100 100 100\n100 10 0 10 100 100\n100 100 10 0 10 100\n100 100 100 10 0 10\n10 100 100 100 10 0\nEOF\n";
        let data = crate::tsp::tsplib::read_from_str(input).unwrap();
        let dm = data.distance_matrix().unwrap();
        // Optimal ring tour: 0-1-2-3-4-5, edges are all 10 = 60
        let route = vec![1, 2, 3, 4, 5, 6];
        let cost = tour_cost_from_matrix(&route, &dm);
        assert!((cost - 60.0).abs() < 0.01, "expected 60, got {cost}");
    }

    #[test]
    fn compare_tours_from_matrix_ring6_self_is_zero_gap() {
        let input = "NAME: ring6\nDIMENSION: 6\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: FULL_MATRIX\nEDGE_WEIGHT_SECTION\n0 10 100 100 100 10\n10 0 10 100 100 100\n100 10 0 10 100 100\n100 100 10 0 10 100\n100 100 100 10 0 10\n10 100 100 100 10 0\nEOF\n";
        let data = crate::tsp::tsplib::read_from_str(input).unwrap();
        let dm = data.distance_matrix().unwrap();
        let route = vec![1, 2, 3, 4, 5, 6];
        let stats = compare_tours_from_matrix(&route, &route, &dm);
        assert_eq!(stats.gap_pct, 0.0);
        assert_eq!(stats.shared_edges, 6);
        assert_eq!(stats.solver_only_edges, 0);
    }

    #[test]
    fn tour_cost_with_type_geo_burma14_pair() {
        // Two cities from burma14 at positions that should produce the known
        // GEO distance from the distance_matrix tests: 837.
        let a = KDPoint::new_with_id(1, &[16.47, 96.10]);
        let b = KDPoint::new_with_id(2, &[23.70, 96.99]);
        let cities = vec![a, b];
        let route = vec![1, 2];
        let cost = tour_cost_with_type(&route, &cities, DistanceType::Geo);
        assert!(
            (cost - 837.0 * 2.0).abs() < 1.0,
            "GEO cost should be ~1674 (837 per edge), got {cost}"
        );
    }

    #[test]
    fn compare_tours_with_type_geo_is_different_from_euc2d() {
        let a = KDPoint::new_with_id(1, &[16.47, 96.10]);
        let b = KDPoint::new_with_id(2, &[23.70, 96.99]);
        let c = KDPoint::new_with_id(3, &[22.39, 93.37]);
        let cities = vec![a, b, c];
        let solver = vec![1, 2, 3];
        let optimal = vec![1, 3, 2];
        let geo_stats = compare_tours_with_type(&solver, &optimal, &cities, DistanceType::Geo);
        let euc_stats = compare_tours_with_type(&solver, &optimal, &cities, DistanceType::Euc2D);
        assert!(
            (geo_stats.optimal_cost - euc_stats.optimal_cost).abs() > 1.0,
            "GEO and EUC_2D costs should differ for these cities, got geo={}, euc={}",
            geo_stats.optimal_cost,
            euc_stats.optimal_cost
        );
    }
}

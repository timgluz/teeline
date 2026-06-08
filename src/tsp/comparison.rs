use std::collections::{HashMap, HashSet};
use crate::tsp::kdtree::KDPoint;

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
    let idx: HashMap<usize, &KDPoint> = cities.iter().map(|c| (c.id, c)).collect();
    let n = route.len();
    (0..n)
        .map(|i| {
            let a = idx[&route[i]];
            let b = idx[&route[(i + 1) % n]];
            let dx = a.x() - b.x();
            let dy = a.y() - b.y();
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}

pub fn compare_tours(solver: &[usize], optimal: &[usize], cities: &[KDPoint]) -> ComparisonStats {
    let optimal_cost = tour_cost(optimal, cities);
    let solver_cost = tour_cost(solver, cities);
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
            KDPoint { id: 1, coords: [0.0, 0.0] },
            KDPoint { id: 2, coords: [1.0, 0.0] },
            KDPoint { id: 3, coords: [1.0, 1.0] },
            KDPoint { id: 4, coords: [0.0, 1.0] },
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
        let solver  = vec![1, 2, 4, 3];
        let stats = compare_tours(&solver, &optimal, &cities);

        assert!(stats.gap_pct > 0.0, "solver has a crossing edge, must be worse");
        assert_eq!(stats.shared_edges, 2,        "edges (1-2) and (3-4) are shared");
        assert_eq!(stats.solver_only_edges, 2,   "solver has extra (2-4) and (1-3)");
        assert_eq!(stats.optimal_only_edges, 2,  "optimal has (2-3) and (1-4) not in solver");
        // Invariant: shared + solver_only == n
        assert_eq!(stats.shared_edges + stats.solver_only_edges, optimal.len());
    }

    #[test]
    fn compare_berlin52_optimal_against_itself() {
        use crate::tsp::{opt_tour, tsplib};

        let tsp = tsplib::read_from_file(Path::new("data/tsplib/berlin52.tsp"))
            .expect("berlin52.tsp must exist in data/tsplib/");
        let opt = opt_tour::read_from_file(Path::new("data/tsplib/berlin52.opt.tour"))
            .expect("berlin52.opt.tour must exist in data/tsplib/");
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
        assert!(stats2.gap_pct > 0.0, "ID-sorted route must be worse than the known optimal");
    }
}

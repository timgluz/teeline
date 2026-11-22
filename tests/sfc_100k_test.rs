//! 100k-city Space-Filling Curve through the normal solver dispatch.
//!
//! Ignored by default: it builds 100k cities and runs the full constructor. The end-to-end
//! memory measurement lives in `scripts/bench-distance-scaling.sh`; this test pins the
//! correctness contract — a valid tour and a reported cost equal to an independent
//! recomputation.

use teeline::tsp::{
    AppOptions, DistanceBackend, DistanceSource, DistanceType, Solvers, TspProblem,
    kdtree::KDPoint, solve_problem,
};

/// A distinct point per cell keeps the SFC grid mapping unambiguous.
fn grid_cities(n: usize) -> Vec<KDPoint> {
    let side = (n as f64).sqrt().ceil() as usize;
    (0..n)
        .map(|i| KDPoint::new_with_id(i, &[((i % side) * 10) as f32, ((i / side) * 10) as f32]))
        .collect()
}

#[test]
#[ignore = "slow: 100k-city SFC"]
fn sfc_100k_via_dispatch_is_valid_and_self_consistent() {
    let n = 100_000;
    let problem =
        TspProblem::from_points(grid_cities(n), DistanceType::Euc2D, DistanceBackend::Auto)
            .expect("100k coordinate problem builds");

    let solution =
        solve_problem(Solvers::SpaceFillingCurve, &problem, &AppOptions::default()).unwrap();

    assert_eq!(solution.route().len(), n);
    let mut ids = solution.route().to_vec();
    ids.sort_unstable();
    assert_eq!(
        ids,
        (0..n).collect::<Vec<_>>(),
        "tour must visit every city"
    );

    let route = solution.route();
    let mut reference = 0.0f32;
    for pair in route.windows(2) {
        reference += problem
            .distances
            .distance_between(pair[0], pair[1])
            .expect("consecutive route cities exist");
    }
    reference += problem
        .distances
        .distance_between(route[n - 1], route[0])
        .expect("closing edge exists");

    assert_eq!(
        solution.total, reference,
        "reported cost must equal the independent tour sum"
    );
    assert!(solution.total > 0.0);
}

use std::collections::HashSet;
use teeline::tsp::distance_matrix::DistanceMatrix;
use teeline::tsp::kdtree::KDPoint;
use teeline::tsp::{distance_matrix, kdtree};

/// Local float-comparison helper (mirrors `src/test/helpers.rs` semantics).
/// The crate's `assert_approx` is behind `#[cfg(test)]` and not reachable from
/// integration tests.
fn assert_approx(expected: f32, actual: f32) {
    let abs = (expected - actual).abs();
    let magnitude = expected.abs().max(actual.abs());
    let tol = 1e-5_f32.max(magnitude * 1e-5);
    assert!(
        abs <= tol,
        "assert_approx: expected {expected}, got {actual} (diff {abs} > tol {tol})"
    );
}

/// Regression: nearest(n > 1) must return the correct k nearest, not just the closest.
///
/// Bug: KDNode::nearest() propagates only the single closest point back up per
/// recursive level, and prunes the far branch using closest_distance() instead of
/// the k-th farthest distance. For n > 1 this causes the far subtree to be pruned
/// too aggressively, yielding fewer than n results and wrong IDs.
///
/// Five cities on the x-axis; target = cities[0]. The KD tree splits at x=2
/// (pivot id=2), putting cities 3 and 4 in the far branch. With n=4 the buggy
/// code returns only [{1,2}] (2 results) because after visiting the close branch
/// closest_distance=1.0 < split_dist=2.0, so the far branch is pruned even
/// though the buffer is not yet full.
#[test]
fn test_knn_n_gt_1_matches_oracle() {
    let cities = kdtree::build_points(&[
        vec![0.0, 0.0], // id=0  <- target (excluded from own results)
        vec![1.0, 0.0], // id=1  distance 1 from target
        vec![2.0, 0.0], // id=2  distance 2
        vec![3.0, 0.0], // id=3  distance 3  (in far subtree after split at x=2)
        vec![4.0, 0.0], // id=4  distance 4  (in far subtree)
    ]);

    let kd = kdtree::from_cities(&cities);
    let dm = distance_matrix::from_cities(&cities);

    for n in 1usize..=4 {
        let kd_res = kd.nearest(&cities[0], n);
        let dm_res = dm.nearest(&cities[0], n);

        let kd_ids: HashSet<usize> = kd_res.nearest().iter().map(|r| r.point.id).collect();
        let dm_ids: HashSet<usize> = dm_res.nearest().iter().map(|r| r.point.id).collect();

        assert_eq!(
            kd_res.nearest().len(),
            n,
            "n={n}: kdtree returned {} results, expected {n}",
            kd_res.nearest().len()
        );
        assert_eq!(
            kd_ids, dm_ids,
            "n={n}: kdtree={kd_ids:?}  oracle={dm_ids:?}"
        );
    }
}

// ── Issue #97 regression tests ───────────────────────────────────────────────

/// nearest() must not include the query city itself (self-distance 0.0).
/// Guaranteed by NearestResult::add() since #95 — test documents the contract.
#[test]
fn test_nearest_excludes_self() {
    let cities = kdtree::build_points(&[
        vec![0.0, 0.0], // id=0 <- target
        vec![1.0, 0.0], // id=1
        vec![2.0, 0.0], // id=2
    ]);
    let dm = distance_matrix::from_cities(&cities);
    let result = dm.nearest(&cities[0], 2);
    let ids: Vec<usize> = result.nearest().iter().map(|r| r.point.id).collect();
    assert!(
        !ids.contains(&0),
        "self must not appear in nearest results: {ids:?}"
    );
}

/// distance_by_pos must return Err (not panic) for an out-of-range position.
/// Bug: old guard `n_items_before > size` passes for pos == n, then panics on
/// direct index access.
#[test]
fn test_distance_by_pos_out_of_range_returns_err() {
    let cities = kdtree::build_points(&[
        vec![0.0, 0.0],
        vec![1.0, 0.0],
        vec![2.0, 0.0], // n=3
    ]);
    let dm = DistanceMatrix::from_cities(&cities).unwrap();
    assert!(dm.distance_by_pos(3, 0).is_err(), "pos >= n must be Err");
    assert!(dm.distance_by_pos(0, 3).is_err(), "pos >= n must be Err");
}

/// distance_between must return Err for an unknown city ID, not panic.
#[test]
fn test_distance_between_unknown_city_returns_err() {
    let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0]]);
    let dm = DistanceMatrix::from_cities(&cities).unwrap();
    assert!(dm.distance_between(99, 0).is_err());
    assert!(dm.distance_between(0, 99).is_err());
}

/// nearest() must return an empty result (not panic) for an unknown target city ID.
#[test]
fn test_nearest_unknown_target_returns_empty() {
    let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![2.0, 0.0]]);
    let dm = distance_matrix::from_cities(&cities);
    let unknown = KDPoint::new_with_id(99, &[0.0, 0.0]);
    let result = dm.nearest(&unknown, 2);
    assert_eq!(0, result.nearest().len(), "unknown target → empty result");
}

/// tour_length must not panic for a path that contains an unknown city ID.
#[test]
fn test_tour_length_with_unknown_city_does_not_panic() {
    let cities = kdtree::build_points(&[
        vec![0.0, 0.0],
        vec![0.0, 0.5],
        vec![0.0, 1.0],
        vec![1.0, 1.0],
        vec![1.0, 0.0],
    ]);
    let dm = DistanceMatrix::from_cities(&cities).unwrap();
    let _ = dm.tour_length(&[0, 99, 1]); // must not panic
}

/// burma14.tsp declares EDGE_WEIGHT_TYPE: GEO; verify distance_type is parsed and
/// that GEO distances are in a realistic range (hundreds of km, not Euclidean ~1.66).
#[test]
fn test_burma14_geo_distance_matrix() {
    let path = std::path::Path::new("tests/fixtures/burma14.tsp");
    let tsp_data = teeline::tsp::tsplib::read_from_file(path).expect("burma14 not found");

    assert_eq!(tsp_data.distance_type, teeline::DistanceType::Geo);

    let dm = tsp_data.distance_matrix().expect("distance matrix");

    // Cities 1=(16.47, 96.10) and 2=(16.47, 94.44) share the same latitude.
    // Euclidean distance would be ~1.66; GEO distance should be much larger (hundreds of km).
    let geo_d = dm
        .distance_between(1, 2)
        .expect("distance between city 1 and 2");
    assert!(
        geo_d > 100.0,
        "GEO distance should be > 100 km, got {geo_d}"
    );
    assert!(
        geo_d < 300.0,
        "GEO distance should be < 300 km, got {geo_d}"
    );
}

/// tour_length_by_pos must return the same result as tour_length for a valid path.
#[test]
fn test_tour_length_by_pos_matches_tour_length() {
    let cities = kdtree::build_points(&[
        vec![0.0, 0.0],
        vec![0.0, 0.5],
        vec![0.0, 1.0],
        vec![1.0, 1.0],
        vec![1.0, 0.0],
    ]);
    let dm = DistanceMatrix::from_cities(&cities).unwrap();
    let city_id_path = vec![0usize, 1, 2, 3, 4];
    let pos_path = vec![0usize, 1, 2, 3, 4]; // positions == ids for 0-indexed cities
    assert_eq!(
        dm.tour_length(&city_id_path),
        dm.tour_length_by_pos(&pos_path)
    );
}

#[test]
fn test_kdtree_vs_distance_matrix() {
    let cities = kdtree::build_points(&[
        vec![0.0, 0.0],
        vec![0.0, 0.5],
        vec![0.0, 1.0],
        vec![1.0, 1.0],
        vec![1.0, 0.0],
    ]);

    let kd = kdtree::from_cities(&cities);
    let dm = distance_matrix::from_cities(&cities);

    let pt1 = KDPoint::new(&[0.0, 0.0]);
    let kd_res1 = kd.nearest(&pt1, 1);
    let dm_res1 = dm.nearest(&pt1, 1);

    assert_eq!(kd_res1.closest_distance(), dm_res1.closest_distance());
}

// ── KD-tree k-NN buffer coverage (Issue: results[1..n] were never asserted) ──

/// The KD-tree's k-NN buffer must return the same *ordered* sequence as the
/// DistanceMatrix oracle — not just the same set. Catches sort/insertion bugs
/// in the buffer that set-membership checks miss.
#[test]
fn test_kdtree_knn_full_buffer_matches_oracle_ordered() {
    let cities = kdtree::build_points(&[
        vec![0.0, 0.0],  // id=0  <- target
        vec![1.0, 0.0],  // id=1  d=1
        vec![2.0, 0.0],  // id=2  d=2
        vec![3.0, 0.0],  // id=3  d=3
        vec![10.0, 0.0], // id=4  d=10
        vec![0.0, 5.0],  // id=5  d=5
    ]);

    let kd = kdtree::from_cities(&cities);
    let dm = distance_matrix::from_cities(&cities);

    for n in 1usize..=5 {
        let kd_res = kd.nearest(&cities[0], n);
        let dm_res = dm.nearest(&cities[0], n);

        let kd_ids: Vec<usize> = kd_res.nearest().iter().map(|r| r.point.id).collect();
        let dm_ids: Vec<usize> = dm_res.nearest().iter().map(|r| r.point.id).collect();

        assert_eq!(
            kd_res.nearest().len(),
            n,
            "n={n}: buffer returned {} items",
            kd_res.nearest().len()
        );
        assert_eq!(
            kd_ids, dm_ids,
            "n={n}: ordered IDs differ — kd={kd_ids:?} oracle={dm_ids:?}"
        );

        // Distances must be monotonically non-decreasing (buffer invariant).
        let dists: Vec<f32> = kd_res.nearest().iter().map(|r| r.distance).collect();
        for w in dists.windows(2) {
            assert!(
                w[0] <= w[1],
                "n={n}: buffer not sorted ascending: {dists:?}"
            );
        }
    }
}

/// n=1 path: buffer holds exactly one item, and it must be the true closest.
#[test]
fn test_kdtree_nearest_n_equals_1() {
    let cities = kdtree::build_points(&[
        vec![0.0, 0.0], // id=0  <- target
        vec![0.5, 0.0], // id=1  d=0.5  (closest)
        vec![1.0, 0.0], // id=2  d=1.0
    ]);

    let kd = kdtree::from_cities(&cities);
    let res = kd.nearest(&cities[0], 1);

    assert_eq!(
        1,
        res.nearest().len(),
        "n=1: buffer must hold exactly 1 item"
    );
    assert_eq!(1, res.nearest()[0].point.id, "n=1: closest must be id=1");
    assert_approx(0.5, res.closest_distance());
}

/// n=0 path: `NearestResult::add` early-returns when n==0. The result must be
/// empty and must not panic, regardless of how many nodes the tree visits.
#[test]
fn test_kdtree_nearest_with_n_zero() {
    let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![2.0, 0.0]]);
    let kd = kdtree::from_cities(&cities);

    let res = kd.nearest(&cities[0], 0);
    assert_eq!(0, res.nearest().len(), "n=0: result must be empty");
}

/// Duplicate-coordinate trees: all points at (0,0) with distinct ids. The
/// query must exclude self (by id) and return the rest, all at distance 0.
#[test]
fn test_kdtree_duplicate_coordinates() {
    let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![0.0, 0.0], vec![0.0, 0.0]]);
    // ids: 0, 1, 2 — all at (0,0)
    let kd = kdtree::from_cities(&cities);

    let res = kd.nearest(&cities[0], 2);
    assert_eq!(
        2,
        res.nearest().len(),
        "duplicate coords: should return 2 of 3"
    );
    assert_approx(0.0, res.closest_distance());

    let ids: HashSet<usize> = res.nearest().iter().map(|r| r.point.id).collect();
    assert!(
        !ids.contains(&0),
        "self must be excluded even with duplicate coords"
    );
    assert!(
        ids.contains(&1) && ids.contains(&2),
        "both other ids must be present: {ids:?}"
    );
}

/// Pruning boundary: target lies exactly on a splitting plane. The far branch
/// must still be visited if it contains candidates closer than the k-th best.
/// Uses collinear points so the split plane lands on an integer x-coordinate.
#[test]
fn test_kdtree_pruning_boundary_on_splitting_plane() {
    // Collinear on x-axis; median split lands at x=2 or x=3.
    let cities = kdtree::build_points(&[
        vec![0.0, 0.0], // id=0  <- target
        vec![1.0, 0.0], // id=1  d=1
        vec![2.0, 0.0], // id=2  d=2  (likely splitting plane)
        vec![3.0, 0.0], // id=3  d=3
        vec![4.0, 0.0], // id=4  d=4
        vec![5.0, 0.0], // id=5  d=5  (in the far subtree)
    ]);

    let kd = kdtree::from_cities(&cities);
    let dm = distance_matrix::from_cities(&cities);

    // k=4: far subtree point (id=3 or id=4) must be reached despite the
    // splitting-plane prune check. Cross-check against the oracle.
    let kd_res = kd.nearest(&cities[0], 4);
    let dm_res = dm.nearest(&cities[0], 4);

    let kd_ids: HashSet<usize> = kd_res.nearest().iter().map(|r| r.point.id).collect();
    let dm_ids: HashSet<usize> = dm_res.nearest().iter().map(|r| r.point.id).collect();
    assert_eq!(
        kd_ids, dm_ids,
        "pruning boundary: kd={kd_ids:?} oracle={dm_ids:?}"
    );
    assert_eq!(4, kd_ids.len(), "all 4 slots must be filled");
}

/// Documents the id-collision footgun (C2): querying with `KDPoint::new`
/// (which defaults `id = 0`) against a tree containing an `id = 0` point
/// silently excludes that point from the results. Callers in a different id
/// space must use `KDPoint::new_with_id(usize::MAX, ...)` — see
/// `src/tsp/fourier.rs:86-95` for the workaround.
#[test]
fn test_kdtree_id_collision_excludes_zero_id() {
    let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![2.0, 0.0]]);
    // ids: 0, 1, 2
    let kd = kdtree::from_cities(&cities);

    // Query with default id=0 — collides with cities[0].id
    let target = KDPoint::new(&[0.0, 0.0]); // id defaults to 0
    let res = kd.nearest(&target, 2);

    let ids: Vec<usize> = res.nearest().iter().map(|r| r.point.id).collect();
    assert!(
        !ids.contains(&0),
        "id-collision footgun: id=0 must be excluded from results (target.id == 0), got {ids:?}"
    );
    // The actual nearest (id=1 at distance 1.0) must still be returned.
    assert!(ids.contains(&1), "id=1 must be present: {ids:?}");
}

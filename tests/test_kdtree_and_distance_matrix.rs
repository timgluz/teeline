use std::collections::HashSet;
use teeline::tsp::distance_matrix::DistanceMatrix;
use teeline::tsp::kdtree::KDPoint;
use teeline::tsp::{distance_matrix, kdtree};

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
    assert!(!ids.contains(&0), "self must not appear in nearest results: {ids:?}");
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
    let cities = kdtree::build_points(&[
        vec![0.0, 0.0],
        vec![1.0, 0.0],
        vec![2.0, 0.0],
    ]);
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
    let geo_d = dm.distance_between(1, 2).expect("distance between city 1 and 2");
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

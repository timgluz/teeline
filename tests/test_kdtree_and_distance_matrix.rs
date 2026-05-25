use std::collections::HashSet;
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

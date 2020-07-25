extern crate teeline;

use teeline::tsp::kdtree::KDPoint;
use teeline::tsp::{distance_matrix, kdtree};

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

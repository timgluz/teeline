use std::cmp::Ordering;

use super::NearestResult;

pub type PointMatrix = Vec<Vec<f32>>;
pub(crate) type KDSubTree = Option<Box<KDNode>>;

/// builds a collection of KDPoints from PointMatrix,
/// where id would be the row_id of PointMatri
pub fn build_points(rows: &[Vec<f32>]) -> Vec<KDPoint> {
    let mut points = vec![];

    for (i, coords) in rows.iter().enumerate() {
        points.push(KDPoint::new_with_id(i, coords));
    }
    points
}

pub fn from_cities(points: &[KDPoint]) -> KDTree {
    let mut tree = KDTree::empty();

    if points.is_empty() {
        return tree;
    };

    let n_points = points.len();
    let tree_points = points.to_vec();
    if let Some(root) = build_subtree(tree_points, 0) {
        tree.size = n_points;
        tree.root = Some(root);
    }

    tree
}

fn build_subtree(points: Vec<KDPoint>, depth: usize) -> KDSubTree {
    if points.is_empty() {
        return None;
    }

    if points.len() == 1 {
        return Some(Box::new(KDNode::leaf(points[0], depth)));
    }

    let (pivot_pt, left_points, right_points) = partition_points(points, depth);
    let root = KDNode::from_subtrees(
        pivot_pt,
        depth,
        build_subtree(left_points, depth + 1),
        build_subtree(right_points, depth + 1),
    );

    Some(Box::new(root))
}

fn partition_points(
    mut points: Vec<KDPoint>,
    depth: usize,
) -> (KDPoint, Vec<KDPoint>, Vec<KDPoint>) {
    let coord = depth % 2;
    let pivot_idx = points.len() / 2;

    points.select_nth_unstable_by(pivot_idx, |a, b| {
        a.cmp_by_coord(b, coord).unwrap_or(Ordering::Equal)
    });

    let pivot_pt = points[pivot_idx];
    let right = points.split_off(pivot_idx + 1);
    points.pop(); // remove pivot
    let left = points;

    (pivot_pt, left, right)
}

#[derive(Debug)]
pub struct KDTree {
    size: usize,
    root: KDSubTree,
}

impl KDTree {
    #[cfg(test)]
    pub(crate) fn new(root: KDNode) -> Self {
        KDTree {
            root: Some(Box::new(root)),
            size: 1,
        }
    }

    pub fn empty() -> Self {
        KDTree {
            root: None,
            size: 0,
        }
    }

    pub fn walk(&self, mut callback: impl FnMut(&KDPoint)) {
        Self::walk_in_order(&self.root, &mut callback);
    }

    fn walk_in_order(subtree: &KDSubTree, callback: &mut impl FnMut(&KDPoint)) {
        if let Some(node) = subtree {
            Self::walk_in_order(&node.left, callback);
            callback(&node.point);
            Self::walk_in_order(&node.right, callback);
        }
    }

    /// Finds the `n` nearest neighbours of `target` in the tree.
    ///
    /// **Id-collision footgun**: `NearestResult::add` skips any tree point whose
    /// `id` matches `target.id`. This is correct when query and tree share the
    /// same id space (e.g. city-to-city queries), but silently excludes points
    /// if the query comes from a different id space with a colliding id.
    /// See `src/tsp/fourier.rs:nearest_sample` for the `usize::MAX` workaround.
    pub fn nearest(&self, target: &KDPoint, n: usize) -> NearestResult {
        let mut acc = NearestResult::new(*target, n);
        if let Some(root) = &self.root {
            root.nearest(target, &mut acc);
        }
        acc
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn to_vec(&self) -> PointMatrix {
        let mut pts = vec![];
        self.walk(|p| pts.push(p.coords().to_vec()));
        pts
    }
}

#[derive(Debug)]
pub(crate) struct KDNode {
    point: KDPoint,
    depth: usize,
    left: KDSubTree,
    right: KDSubTree,
}

impl KDNode {
    #[cfg(test)]
    pub(crate) fn new(
        point: KDPoint,
        depth: usize,
        left: Option<KDNode>,
        right: Option<KDNode>,
    ) -> Self {
        KDNode {
            point,
            depth,
            left: left.map(Box::new),
            right: right.map(Box::new),
        }
    }

    pub(crate) fn from_subtrees(
        point: KDPoint,
        depth: usize,
        left: KDSubTree,
        right: KDSubTree,
    ) -> Self {
        KDNode {
            point,
            depth,
            left,
            right,
        }
    }

    pub(crate) fn leaf(point: KDPoint, depth: usize) -> Self {
        KDNode {
            point,
            depth,
            left: None,
            right: None,
        }
    }

    // The pruning invariant: we skip the far branch only when every point in it
    // is guaranteed farther than our k-th best candidate so far.
    // Guard: acc.search_radius() > split_dist
    //   - search_radius() == INFINITY while the buffer has fewer than n items,
    //     ensuring we never prune before the buffer is full.
    //   - Once full, search_radius() == farthest_distance(), the standard
    //     k-d tree k-NN pruning condition.
    fn nearest(&self, target_point: &KDPoint, acc: &mut NearestResult) {
        acc.add(self.point, self.point.distance(target_point));

        let (closest_branch, further_branch) = match self.cmp_by_point(target_point) {
            None => panic!("Dimension conflict in nearest function"),
            Some(Ordering::Greater) => (self.left(), self.right()),
            Some(_) => (self.right(), self.left()),
        };

        if let Some(branch) = closest_branch {
            branch.nearest(target_point, acc);
        }

        let split_dist = self.point.split_distance(target_point, self.level_coord());
        if acc.search_radius() > split_dist
            && let Some(branch) = further_branch
        {
            branch.nearest(target_point, acc);
        }
    }

    fn cmp_by_point(&self, other: &KDPoint) -> Option<Ordering> {
        self.point.cmp_by_coord(other, self.level_coord())
    }

    fn level_coord(&self) -> usize {
        self.depth % 2
    }

    pub(crate) fn left(&self) -> Option<&KDNode> {
        self.left.as_deref()
    }

    pub(crate) fn right(&self) -> Option<&KDNode> {
        self.right.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn is_leaf(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }

    #[cfg(test)]
    pub(crate) fn height(&self) -> usize {
        if self.is_leaf() {
            1
        } else {
            let left_height = self.left().map_or(0, |n| n.height());
            let right_height = self.right().map_or(0, |n| n.height());

            1 + std::cmp::max(left_height, right_height)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KDPoint {
    pub id: usize,
    pub coords: [f32; 2],
}

impl KDPoint {
    pub fn new(coords: &[f32]) -> Self {
        assert!(
            coords.len() >= 2,
            "KDPoint requires at least 2 coordinates, got {}",
            coords.len()
        );
        KDPoint {
            id: 0,
            coords: [coords[0], coords[1]],
        }
    }

    pub fn new_with_id(id: usize, coords: &[f32]) -> Self {
        assert!(
            coords.len() >= 2,
            "KDPoint requires at least 2 coordinates, got {}",
            coords.len()
        );
        KDPoint {
            id,
            coords: [coords[0], coords[1]],
        }
    }

    pub fn dim(&self) -> usize {
        2
    }

    pub fn coords(&self) -> &[f32] {
        &self.coords
    }

    pub fn get(&self, dimension: usize) -> Option<f32> {
        self.coords.get(dimension).copied()
    }

    pub fn distance(&self, other: &KDPoint) -> f32 {
        let dx = self.coords[0] - other.coords[0];
        let dy = self.coords[1] - other.coords[1];
        (dx * dx + dy * dy).sqrt()
    }

    fn split_distance(&self, other: &KDPoint, coord: usize) -> f32 {
        (self.coords[coord] - other.coords[coord]).abs()
    }

    pub fn cmp_by_coord(&self, other: &KDPoint, coord: usize) -> Option<Ordering> {
        let a = self.coords.get(coord)?;
        let b = other.coords.get(coord)?;
        // Relative tolerance: scales with the magnitude of the coordinates so
        // it works across the full TSPLIB range (fractional to ~thousands).
        // The bare f32::EPSILON used previously is the ULP at 1.0, making the
        // Equal branch unreachable for coords > ~16 and overly loose near 0.
        let tol = a.abs().max(b.abs()) * f32::EPSILON;
        let res = if (a - b).abs() <= tol {
            Ordering::Equal
        } else if a < b {
            Ordering::Less
        } else {
            Ordering::Greater
        };
        Some(res)
    }

    pub fn x(&self) -> f32 {
        self.coords[0]
    }

    pub fn y(&self) -> f32 {
        self.coords[1]
    }
}

impl PartialEq for KDPoint {
    fn eq(&self, other: &KDPoint) -> bool {
        self.distance(other) < f32::EPSILON
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::assert_approx;

    #[test]
    fn kdpoint_cmp_by_coord_out_of_bounds_returns_none() {
        let pt = KDPoint::new(&[1.0, 0.0]);
        assert_eq!(None, pt.cmp_by_coord(&pt, 2));
    }

    #[test]
    fn kdpoint_cmp_by_coord_with_pt_less_than() {
        let pt = KDPoint::new(&[-1.0, 0.0]);
        let other_pt = KDPoint::new(&[0.0, -1.0]);

        assert_eq!(Some(Ordering::Less), pt.cmp_by_coord(&other_pt, 0));
        assert_eq!(Some(Ordering::Less), other_pt.cmp_by_coord(&pt, 1));
    }

    #[test]
    fn kdpoint_cmp_by_coord_with_pt_equal() {
        let pt = KDPoint::new(&[-1.0, 0.0]);

        assert_eq!(Some(Ordering::Equal), pt.cmp_by_coord(&pt, 0));
    }

    #[test]
    fn kdpoint_cmp_by_coord_with_pt_greater_than() {
        let pt = KDPoint::new(&[1.0, 0.0]);
        let other_pt = KDPoint::new(&[0.0, 1.0]);

        assert_eq!(Some(Ordering::Greater), pt.cmp_by_coord(&other_pt, 0));
        assert_eq!(Some(Ordering::Greater), other_pt.cmp_by_coord(&pt, 1));
    }

    #[test]
    fn kdpoint_cmp_by_coord_large_magnitude_ulp_equal() {
        // At magnitude ~5000, a 1-ULP difference falls within the relative
        // tolerance (5000 * EPSILON ~= 5.96e-4 > ULP ~= 4.88e-4), so it
        // returns Equal. The old bare-EPSILON code returned Greater here.
        let base = 5000.0_f32;
        let next = f32::from_bits(base.to_bits() + 1);
        let pt = KDPoint::new(&[base, 0.0]);
        assert_eq!(
            Some(Ordering::Equal),
            pt.cmp_by_coord(&KDPoint::new(&[next, 0.0]), 0),
            "1-ULP diff at magnitude 5000 should be Equal with relative tolerance"
        );
    }

    #[test]
    #[should_panic(expected = "at least 2 coordinates")]
    fn kdpoint_new_with_short_slice_panics() {
        let _ = KDPoint::new(&[1.0]);
    }

    #[test]
    #[should_panic(expected = "at least 2 coordinates")]
    fn kdpoint_new_with_id_with_short_slice_panics() {
        let _ = KDPoint::new_with_id(5, &[1.0]);
    }

    #[test]
    #[should_panic(expected = "at least 2 coordinates")]
    fn kdpoint_new_with_empty_slice_panics() {
        let _ = KDPoint::new(&[]);
    }

    #[test]
    fn kdpoint_eq_with_same_values() {
        let pt = KDPoint::new(&[1.0, 0.0]);

        assert!(pt.eq(&pt));
    }

    #[test]
    fn kdpoint_distance_from_origin_to_origin() {
        let pt = KDPoint::new(&[0.0, 0.0]);

        assert_approx(0.0, pt.distance(&pt));
    }

    #[test]
    fn kdpoint_distance_from_origin_to_x_axis() {
        let origin = KDPoint::new(&[0.0, 0.0]);
        let other = KDPoint::new(&[1.0, 0.0]);

        assert_approx(1.0, origin.distance(&other));
    }

    #[test]
    fn kdpoint_distance_from_origin_to_y_axis() {
        let origin = KDPoint::new(&[0.0, 0.0]);
        let other = KDPoint::new(&[0.0, 1.0]);

        assert_approx(1.0, origin.distance(&other));
    }

    #[test]
    fn kdpoint_distance_on_diagonal() {
        let pt = KDPoint::new(&[-1.0, -1.0]);
        let other = KDPoint::new(&[1.0, 1.0]);

        assert_approx(2.828427, pt.distance(&other))
    }

    #[test]
    fn kdtree_walk_with_empty_tree() {
        let tree = KDTree::empty();

        let mut pts: Vec<KDPoint> = vec![];
        tree.walk(|pt| pts.push(*pt));

        assert!(pts.is_empty());
    }

    #[test]
    fn kdtree_walk_with_only_root_node() {
        let root = KDNode::new(KDPoint::new(&[0.0, 0.0]), 0, None, None);
        let tree = KDTree::new(root);

        let mut pts: Vec<KDPoint> = vec![];
        tree.walk(|pt| pts.push(*pt));

        assert!(!pts.is_empty());
        assert_eq!(1, pts.len());
        assert_eq!(&[0.0, 0.0], pts[0].coords());
    }

    #[test]
    fn partition_points_single_elem() {
        let points = build_points(&[vec![0.0, 0.0]]);

        let res = partition_points(points, 0);
        assert_eq!(&[0.0, 0.0], res.0.coords());
        assert!(res.1.is_empty());
        assert!(res.2.is_empty());
    }

    #[test]
    fn partition_points_with_2points_with_left_subtree() {
        let points = build_points(&[vec![-1.0, 0.0], vec![0.0, 0.0]]);

        let res = partition_points(points, 0);
        assert_eq!(&[0.0, 0.0], res.0.coords());
        assert_eq!(&[-1.0, 0.0], res.1[0].coords());
        assert!(res.2.is_empty());
    }

    #[test]
    fn partition_points_with_2points_with_right_subtree() {
        let points = build_points(&vec![vec![0.0, 0.0], vec![2.0, 0.0]]);

        let res = partition_points(points, 0);
        assert_eq!(&[2.0, 0.0], res.0.coords());
        assert_eq!(&[0.0, 0.0], res.1[0].coords());
        assert!(res.2.is_empty());
    }

    #[test]
    fn partition_points_with_2points_with_full_tree() {
        let points = build_points(&vec![vec![-1.0, 0.0], vec![2.0, 0.0], vec![0.0, 0.0]]);

        let res = partition_points(points, 0);
        assert_eq!(&[0.0, 0.0], res.0.coords());
        assert_eq!(&[-1.0, 0.0], res.1[0].coords());
        assert_eq!(&[2.0, 0.0], res.2[0].coords());
    }

    #[test]
    fn partition_points_with_3points_by_second_dimension() {
        let points = build_points(&vec![vec![0.0, 0.0], vec![2.0, -1.0], vec![1.0, 2.0]]);

        let res = partition_points(points, 1);
        assert_eq!(&[0.0, 0.0], res.0.coords());
        assert_eq!(&[2.0, -1.0], res.1[0].coords());
        assert_eq!(&[1.0, 2.0], res.2[0].coords());
    }

    #[test]
    fn from_cities_example() {
        let points = build_points(&vec![
            vec![0.0, 0.0],
            vec![-1.0, 0.0],
            vec![1.0, 0.0],
            vec![-1.0, -1.0],
            vec![-1.0, 1.0],
            vec![1.0, -1.0],
            vec![1.0, 1.0],
        ]);
        let tree = from_cities(&points);

        assert_eq!(7, tree.len());

        let mut coords: PointMatrix = vec![];
        tree.walk(|n| coords.push(n.coords().to_vec()));

        assert_eq!(vec![-1.0, -1.0], coords[0]);
        assert_eq!(vec![-1.0, 0.0], coords[1]);
        assert_eq!(vec![-1.0, 1.0], coords[2]);
        assert_eq!(vec![0.0, 0.0], coords[3]);
        assert_eq!(vec![1.0, -1.0], coords[4]);
        assert_eq!(vec![1.0, 0.0], coords[5]);
        assert_eq!(vec![1.0, 1.0], coords[6]);
    }

    #[test]
    fn kdtree_nearest_for_tsp_5_1() {
        let cities = build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);

        let kd = from_cities(&cities);

        let res = kd.nearest(&cities[0], 2);
        assert_eq!(cities[1].id, res.closest_point().unwrap().id);

        let res2 = kd.nearest(&cities[1], 2);
        assert_eq!(cities[2].id, res2.closest_point().unwrap().id);

        let res3 = kd.nearest(&cities[2], 2);
        assert_eq!(cities[1].id, res3.closest_point().unwrap().id);

        let res4 = kd.nearest(&cities[3], 2);
        assert_eq!(cities[2].id, res4.closest_point().unwrap().id);

        let res5 = kd.nearest(&cities[4], 2);
        assert_eq!(cities[3].id, res5.closest_point().unwrap().id);
    }

    #[test]
    fn kdtree_nearest_with_points_around_node4() {
        let points = build_points(&[
            vec![100.0, 100.0],
            vec![-100.0, 100.0],
            vec![100.0, -100.0],
            vec![-100.0, -100.0], // it is node 4
        ]);

        let expected_coords = [-100.0, -100.0];
        let tree = from_cities(&points);
        assert_eq!(4, tree.len());

        let pt1 = KDPoint::new(&[-110.0, -100.0]);
        let res = tree.nearest(&pt1, 1);

        assert_approx(10.0, res.closest_distance());
        assert_eq!(expected_coords, res.closest_point().unwrap().coords);

        let pt2 = KDPoint::new(&[-90.0, -100.0]);
        let res = tree.nearest(&pt2, 1);

        assert_approx(10.0, res.closest_distance());
        assert_eq!(expected_coords, res.closest_point().unwrap().coords);

        let pt3 = KDPoint::new(&[-100.0, -90.0]);
        let res = tree.nearest(&pt3, 1);

        assert_approx(10.0, res.closest_distance());
        assert_eq!(expected_coords, res.closest_point().unwrap().coords);

        let pt4 = KDPoint::new(&[-100.0, -110.0]);
        let res = tree.nearest(&pt4, 1);

        assert_approx(10.0, res.closest_distance());
        assert_eq!(expected_coords, res.closest_point().unwrap().coords);
    }
}

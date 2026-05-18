use std::cell::RefCell;
use std::cmp::Ordering;

use super::NearestResult;

pub type PointMatrix = Vec<Vec<f32>>;
pub type KDSubTree = Option<Box<KDNode>>;

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
        let leaf_node = KDNode::leaf(points[0].clone(), depth);
        return Some(Box::new(leaf_node));
    }

    let k = points[0].dim();
    let (pivot_pt, left_points, right_points) = partition_points(points, depth, k);
    let root = KDNode::from_subtrees(
        pivot_pt,
        depth,
        build_subtree(left_points, depth + 1),
        build_subtree(right_points, depth + 1),
    );

    Some(Box::new(root))
}

fn partition_points(
    points: Vec<KDPoint>,
    depth: usize,
    k: usize,
) -> (KDPoint, Vec<KDPoint>, Vec<KDPoint>) {
    let mut sorted_points = points.clone();

    if sorted_points.len() == 1 {
        let pivot_pt = sorted_points[0].clone();
        return (pivot_pt, vec![], vec![]);
    }

    let coord = depth % k;
    sorted_points.sort_by(|a, b| a.cmp_by_coord(b, coord).unwrap());

    let pivot_idx = sorted_points.len() / 2;
    let pivot_pt = sorted_points[pivot_idx].clone();

    (
        pivot_pt,
        sorted_points[0..pivot_idx].to_vec(),
        sorted_points[(pivot_idx + 1)..].to_vec(),
    )
}

#[derive(Debug)]
pub struct KDTree {
    size: usize,
    root: KDSubTree,
}

impl KDTree {
    pub fn new(root: KDNode) -> Self {
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

    pub fn walk(&self, callback: impl Fn(&KDPoint)) {
        self.walk_in_order(&self.root, &callback);
    }

    pub fn walk_in_order(&self, subtree: &KDSubTree, callback: &impl Fn(&KDPoint)) {
        if let Some(node) = subtree {
            self.walk_in_order(&node.left, callback);
            callback(&node.point);
            self.walk_in_order(&node.right, callback);
        }
    }

    pub fn nearest(&self, target: &KDPoint, n: usize) -> NearestResult {
        let best_result = NearestResult::new(target.clone(), f32::INFINITY, n);

        match &self.root {
            None => best_result,
            Some(n) => n.nearest(target, best_result),
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn to_vec(&self) -> PointMatrix {
        let pts: RefCell<PointMatrix> = RefCell::new(vec![]);

        self.walk(|n| pts.borrow_mut().push(n.coords().to_vec()));

        pts.borrow().to_vec()
    }
}

#[derive(Debug)]
pub struct KDNode {
    point: KDPoint,
    depth: usize,
    size: usize, // todo: remove seems redundant
    left: KDSubTree,
    right: KDSubTree,
}

impl KDNode {
    pub fn new(point: KDPoint, depth: usize, left: Option<KDNode>, right: Option<KDNode>) -> Self {
        let left_node = left.map(Box::new);
        let right_node = right.map(Box::new);

        let left_size = left_node.as_ref().map_or(0, |n| n.len());
        let right_size = right_node.as_ref().map_or(0, |n| n.len());

        KDNode {
            point,
            depth,
            size: 1 + left_size + right_size,
            left: left_node,
            right: right_node,
        }
    }

    pub fn from_subtrees(point: KDPoint, depth: usize, left: KDSubTree, right: KDSubTree) -> Self {
        let left_size = left.as_ref().map_or(0, |n| n.len());
        let right_size = right.as_ref().map_or(0, |n| n.len());

        KDNode {
            point,
            depth,
            left,
            right,
            size: 1 + left_size + right_size,
        }
    }

    pub fn leaf(point: KDPoint, depth: usize) -> Self {
        KDNode {
            point,
            depth,
            size: 1,
            left: None,
            right: None,
        }
    }

    pub fn nearest(&self, target_point: &KDPoint, best_result: NearestResult) -> NearestResult {
        if self.is_empty() {
            return best_result;
        }

        let distance_from_target = self.point.distance(target_point);

        let best_distance = best_result.distance;
        let mut nearest_result = best_result;

        if distance_from_target <= best_distance {
            let pt = self.point.clone();
            nearest_result.add(pt, distance_from_target);
        };

        let (closest_branch, futher_branch) = match self.cmp_by_point(target_point) {
            None => panic!("Dimension conflict in nearest function"),
            Some(Ordering::Greater) => (self.left(), self.right()),
            Some(_) => (self.right(), self.left()),
        };

        if let Some(branch) = closest_branch {
            let closest_result = branch.nearest(target_point, nearest_result.clone());
            let pt_distance = closest_result.closest_distance();
            nearest_result.add(closest_result.point, pt_distance);
        }

        // check distance from split line
        let split_dist = self.point.split_distance(target_point, self.level_coord());
        if nearest_result.closest_distance() > split_dist
            && let Some(branch) = futher_branch
        {
            let further_result = branch.nearest(target_point, nearest_result.clone());
            let pt_distance = further_result.closest_distance();
            nearest_result.add(further_result.point, pt_distance);
        }

        nearest_result
    }

    fn cmp_by_point(&self, other: &KDPoint) -> Option<Ordering> {
        self.point.cmp_by_coord(other, self.level_coord())
    }

    /// returns a dimension for comparision
    fn level_coord(&self) -> usize {
        self.depth % self.point.dimensionality
    }

    pub fn left(&self) -> Option<&KDNode> {
        self.left.as_deref()
    }

    pub fn right(&self) -> Option<&KDNode> {
        self.right.as_deref()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_leaf(&self) -> bool {
        self.len() == 1
    }

    pub fn len(&self) -> usize {
        self.size
    }

    /// returns the number of levels in the subtree rooted at this node;
    /// leaves have height 1
    pub fn height(&self) -> usize {
        if self.is_empty() {
            0
        } else if self.is_leaf() {
            1
        } else {
            let left_height = self.left().map_or(0, |n| n.height());
            let right_height = self.right().map_or(0, |n| n.height());

            1 + std::cmp::max(left_height, right_height)
        }
    }
}

#[derive(Debug, Clone)]
pub struct KDPoint {
    pub id: usize,
    dimensionality: usize,
    coords: Vec<f32>,
}

impl KDPoint {
    pub fn new(coords: &[f32]) -> Self {
        KDPoint {
            id: 0,
            dimensionality: coords.len(),
            coords: coords.to_vec(),
        }
    }

    pub fn new_with_id(id: usize, coords: &[f32]) -> Self {
        KDPoint {
            id,
            dimensionality: coords.len(),
            coords: coords.to_vec(),
        }
    }

    pub fn dim(&self) -> usize {
        self.dimensionality
    }

    pub fn coords(&self) -> &[f32] {
        &self.coords[..]
    }

    pub fn get(&self, dimension: usize) -> Option<f32> {
        self.coords.get(dimension).copied()
    }


    pub fn distance(&self, other: &KDPoint) -> f32 {
        let distance: f32 = self
            .coords
            .iter()
            .zip(other.coords())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt();

        distance
    }

    /// returns distance from split level
    fn split_distance(&self, other: &KDPoint, coord: usize) -> f32 {
        (self.coords[coord] - other.get(coord).unwrap()).abs()
    }

    pub fn cmp_by_coord(&self, other: &KDPoint, coord: usize) -> Option<Ordering> {
        if self.get(coord).is_none() || other.get(coord).is_none() {
            return None;
        }

        let self_coord = self.get(coord).unwrap();
        let other_coord = other.get(coord).unwrap();

        let res = if self_coord < other_coord {
            Ordering::Less
        } else if (self_coord - other_coord).abs() < f32::EPSILON {
            Ordering::Equal
        } else {
            Ordering::Greater
        };

        Some(res)
    }

    pub fn x(&self) -> f32 {
        if self.dimensionality < 1 {
            panic!("for accessing y, dimensionality must be > 0");
        }

        self.coords[0]
    }

    pub fn y(&self) -> f32 {
        if self.dimensionality < 2 {
            panic!("for accessing y, dimensionality must be > 1");
        }

        self.coords[1]
    }
}

impl PartialEq for KDPoint {
    fn eq(&self, other: &KDPoint) -> bool {
        if self.dimensionality != other.dimensionality {
            return false;
        }
        self.distance(other) < f32::EPSILON
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::assert_approx;
    use std::cell::RefCell;

    #[test]
    fn kdpoint_cmp_by_coord_with_empty_points() {
        let first_pt = KDPoint::new(&[]);
        let other_pt = KDPoint::new(&[]);

        assert_eq!(None, first_pt.cmp_by_coord(&other_pt, 0));
    }

    #[test]
    fn kdpoint_cmp_by_coord_when_other_node_has_different_dim() {
        let first_pt = KDPoint::new(&[1.0, 0.0]);
        let other_pt = KDPoint::new(&[0.0]);

        assert_eq!(None, first_pt.cmp_by_coord(&other_pt, 1));
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

        let pts: RefCell<Vec<KDPoint>> = RefCell::new(vec![]);
        tree.walk(|pt| pts.borrow_mut().push(pt.clone()));

        assert!(pts.borrow().is_empty());
    }

    #[test]
    fn kdtree_walk_with_only_root_node() {
        let root = KDNode::new(KDPoint::new(&[0.0, 0.0]), 0, None, None);
        let tree = KDTree::new(root);

        let pts: RefCell<Vec<KDPoint>> = RefCell::new(vec![]);
        tree.walk(|pt| pts.borrow_mut().push(pt.clone()));

        assert!(!pts.borrow().is_empty());
        assert_eq!(1, pts.borrow().len());
        assert_eq!(&[0.0, 0.0], pts.borrow().get(0).unwrap().coords());
    }

    #[test]
    fn partition_points_single_elem() {
        let points = build_points(&[vec![0.0, 0.0]]);

        let res = partition_points(points, 0, 2);
        assert_eq!(&[0.0, 0.0], res.0.coords());
        assert!(res.1.is_empty());
        assert!(res.2.is_empty());
    }

    #[test]
    fn partition_points_with_2points_with_left_subtree() {
        let points = build_points(&[vec![-1.0, 0.0], vec![0.0, 0.0]]);

        let res = partition_points(points, 0, 2);
        assert_eq!(&[0.0, 0.0], res.0.coords());
        assert_eq!(&[-1.0, 0.0], res.1[0].coords());
        assert!(res.2.is_empty());
    }

    #[test]
    fn partition_points_with_2points_with_right_subtree() {
        let points = build_points(&vec![vec![0.0, 0.0], vec![2.0, 0.0]]);

        let res = partition_points(points, 0, 2);
        assert_eq!(&[2.0, 0.0], res.0.coords());
        assert_eq!(&[0.0, 0.0], res.1[0].coords());
        assert!(res.2.is_empty());
    }

    #[test]
    fn partition_points_with_2points_with_full_tree() {
        let points = build_points(&vec![vec![-1.0, 0.0], vec![2.0, 0.0], vec![0.0, 0.0]]);

        let res = partition_points(points, 0, 2);
        assert_eq!(&[0.0, 0.0], res.0.coords());
        assert_eq!(&[-1.0, 0.0], res.1[0].coords());
        assert_eq!(&[2.0, 0.0], res.2[0].coords());
    }

    #[test]
    fn partition_points_with_3points_by_second_dimension() {
        let points = build_points(&vec![vec![0.0, 0.0], vec![2.0, -1.0], vec![1.0, 2.0]]);

        let res = partition_points(points, 1, 2);
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

        let points: RefCell<PointMatrix> = RefCell::new(vec![]);
        tree.walk(|n| points.borrow_mut().push(n.coords().to_vec()));

        assert_eq!(vec![-1.0, -1.0], points.borrow()[0]);
        assert_eq!(vec![-1.0, 0.0], points.borrow()[1]);
        assert_eq!(vec![-1.0, 1.0], points.borrow()[2]);
        assert_eq!(vec![0.0, 0.0], points.borrow()[3]);
        assert_eq!(vec![1.0, -1.0], points.borrow()[4]);
        assert_eq!(vec![1.0, 0.0], points.borrow()[5]);
        assert_eq!(vec![1.0, 1.0], points.borrow()[6]);
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
        assert_eq!(cities[1].id, res.point.id);

        let res2 = kd.nearest(&cities[1], 2);
        assert_eq!(cities[2].id, res2.point.id);

        let res3 = kd.nearest(&cities[2], 2);
        assert_eq!(cities[1].id, res3.point.id);

        let res4 = kd.nearest(&cities[3], 2);
        assert_eq!(cities[2].id, res4.point.id);

        let res5 = kd.nearest(&cities[4], 2);
        assert_eq!(cities[3].id, res5.point.id);
    }

    #[test]
    fn kdtree_nearest_with_points_around_node4() {
        let points = build_points(&vec![
            vec![100.0, 100.0],
            vec![-100.0, 100.0],
            vec![100.0, -100.0],
            vec![-100.0, -100.0], // it is node 4
        ]);

        let expected_coords = vec![-100.0, -100.0];
        let tree = from_cities(&points);
        assert_eq!(4, tree.len());

        let pt1 = KDPoint::new(&[-110.0, -100.0]);
        let res = tree.nearest(&pt1, 1);

        assert_approx(10.0, res.distance);
        assert_eq!(expected_coords, res.point.coords);

        let pt2 = KDPoint::new(&[-90.0, -100.0]);
        let res = tree.nearest(&pt2, 1);

        assert_approx(10.0, res.distance);
        assert_eq!(expected_coords, res.point.coords);

        let pt3 = KDPoint::new(&[-100.0, -90.0]);
        let res = tree.nearest(&pt3, 1);

        assert_approx(10.0, res.distance);
        assert_eq!(expected_coords, res.point.coords);

        let pt4 = KDPoint::new(&[-100.0, -110.0]);
        let res = tree.nearest(&pt4, 1);

        assert_approx(10.0, res.distance);
        assert_eq!(expected_coords, res.point.coords);
    }
}

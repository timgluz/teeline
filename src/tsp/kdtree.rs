use std::cell::RefCell;
use std::cmp::Ordering;

pub type PointMatrix = Vec<Vec<f32>>;
pub type KDSubTree = Option<Box<KDNode>>;

pub fn build_tree(points: PointMatrix) -> KDTree {
    let mut tree = KDTree::empty();

    if points.is_empty() {
        return tree;
    };

    let k = points[0].len();
    let n_points = points.len();
    if let Some(root) = build_subtree(points, 0, k) {
        tree.size = n_points;
        tree.root = Some(root);
    }

    tree
}

fn build_subtree(points: PointMatrix, depth: usize, k: usize) -> KDSubTree {
    if points.is_empty() {
        return None;
    }

    if points.len() == 1 {
        let leaf_node = KDNode::leaf(KDPoint::new(points[0].as_ref()), depth);
        return Some(Box::new(leaf_node));
    }

    let (pivot_pt, left_points, right_points) = partition_points(points, depth, k);
    let root = KDNode::from_subtrees(
        pivot_pt,
        depth,
        build_subtree(left_points, depth + 1, k),
        build_subtree(right_points, depth + 1, k),
    );

    Some(Box::new(root))
}

fn partition_points(
    points: PointMatrix,
    depth: usize,
    k: usize,
) -> (KDPoint, PointMatrix, PointMatrix) {
    let mut sorted_points = points.clone();

    if sorted_points.len() == 1 {
        let pivot_pt = KDPoint::new(sorted_points[0].as_ref());
        return (pivot_pt, vec![], vec![]);
    }

    let dim = depth % k;
    sorted_points.sort_by(|a, b| a[dim].partial_cmp(&b[dim]).unwrap());

    let pivot_idx = sorted_points.len() / 2;
    let pivot_pt = KDPoint::new(sorted_points[pivot_idx].as_ref());

    (
        pivot_pt,
        sorted_points[0..pivot_idx].to_vec(),
        sorted_points[(pivot_idx + 1)..].to_vec(),
    )
}

#[derive(Debug)]
pub struct KDTree {
    size: usize,
    dimensionality: usize,
    root: KDSubTree,
}

impl KDTree {
    pub fn new(root: KDNode) -> Self {
        let pt_dimension = root.point.dim();

        KDTree {
            root: Some(Box::new(root)),
            dimensionality: pt_dimension,
            size: 1,
        }
    }

    pub fn empty() -> Self {
        KDTree {
            root: None,
            dimensionality: 0,
            size: 0,
        }
    }
    // usage: find parent, then add point
    fn add(&mut self, new_point: KDPoint) {
        self.size += 1;
        if self.dimensionality == 0 {
            self.dimensionality = new_point.dim();
        }

        let parent = std::mem::replace(&mut self.root, None);
        self.root = self.add_rec(parent, new_point, 0);
    }

    fn add_rec(&mut self, parent: KDSubTree, new_point: KDPoint, depth: usize) -> KDSubTree {
        if parent.is_none() {
            return Some(Box::new(KDNode::leaf(new_point, depth + 1)));
        }

        let mut node = parent.unwrap();
        match node.cmp_by_point(&new_point) {
            None => panic!("Point dimensionality is not matching with tree"), // fails with broken data
            Some(Ordering::Greater) => {
                // if parent is greater than new point then the newpoint should go left
                node.left = self.add_rec(node.left, new_point, depth + 1);
                Some(node)
            }
            _ => {
                node.right = self.add_rec(node.right, new_point, depth + 1);
                Some(node)
            }
        }
    }

    pub fn walk(&self, callback: impl Fn(&KDPoint) -> ()) {
        self.walk_in_order(&self.root, &callback);
    }

    pub fn walk_in_order(&self, subtree: &KDSubTree, callback: &impl Fn(&KDPoint) -> ()) {
        if let Some(node) = subtree {
            self.walk_in_order(&node.left, callback);
            callback(&node.point);
            self.walk_in_order(&node.right, callback);
        }
    }

    pub fn nearest(&self, target: &KDPoint) -> NearestResult {
        let best_result = NearestResult::new(target.clone(), f32::INFINITY);

        match &self.root {
            None => best_result,
            Some(n) => n.nearest(target, best_result),
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn to_vec(&self) -> PointMatrix {
        let pts: RefCell<PointMatrix> = RefCell::new(vec![]);

        self.walk(|n| pts.borrow_mut().push(n.coords().to_vec()));

        let res = pts.borrow().to_vec();
        res
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
        let left_node = match left {
            Some(node) => Some(Box::new(node)),
            None => None,
        };

        let right_node = match right {
            Some(node) => Some(Box::new(node)),
            None => None,
        };

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

        // TODO: remove
        /*
                println!(
                    "Nearest from #{:?} distance: #{:?}, best: {:?}",
                    self.point(),
                    distance_from_target,
                    best_result
                );
        */
        let mut nearest_result = if distance_from_target <= best_result.distance {
            NearestResult::new(self.point.clone(), distance_from_target)
        } else {
            best_result
        };

        let (closest_branch, futher_branch) = match self.cmp_by_point(&target_point) {
            None => panic!("Dimension conflict in nearest function"),
            Some(Ordering::Greater) => (self.left(), self.right()),
            Some(_) => (self.right(), self.left()),
        };

        if closest_branch.is_some() {
            nearest_result = closest_branch
                .unwrap()
                .nearest(target_point, nearest_result);
        }

        // check distance from split line
        let split_dist = self.point.split_distance(&target_point, self.level_coord());
        if nearest_result.distance > split_dist && futher_branch.is_some() {
            nearest_result = futher_branch.unwrap().nearest(target_point, nearest_result);
        }

        nearest_result
    }

    fn point(&self) -> &KDPoint {
        &self.point
    }

    fn cmp(&self, other: &KDNode) -> Option<Ordering> {
        self.cmp_by_point(&other.point)
    }

    fn cmp_by_point(&self, other: &KDPoint) -> Option<Ordering> {
        self.point.cmp_by_coord(other, self.level_coord())
    }

    /// returns a dimension for comparision
    fn level_coord(&self) -> usize {
        self.depth % self.point.dimensionality
    }

    pub fn left(&self) -> Option<&Box<KDNode>> {
        self.left.as_ref().map(|n| n.clone())
    }

    pub fn right(&self) -> Option<&Box<KDNode>> {
        self.right.as_ref().map(|n| n.clone())
    }

    pub fn is_empty(&self) -> bool {
        if self.len() == 0 {
            true
        } else {
            false
        }
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
pub struct NearestResult {
    pub point: KDPoint,
    pub distance: f32,
}

impl NearestResult {
    pub fn new(point: KDPoint, distance: f32) -> Self {
        NearestResult { point, distance }
    }
}

#[derive(Debug, Clone)]
pub struct KDPoint {
    dimensionality: usize,
    coords: Vec<f32>,
}

impl KDPoint {
    pub fn new(coords: &[f32]) -> Self {
        KDPoint {
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
        self.coords.get(dimension).map(|x| x.clone())
    }

    // TODO: finish and add tests
    pub fn eq(&self, other: &KDPoint) -> bool {
        if self.dimensionality != other.dimensionality {
            return false;
        }

        let diff = self.distance(other);
        diff < f32::EPSILON
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn assert_approx(expected_val: f32, actual_val: f32) {
        assert!(
            (expected_val - actual_val).abs() < f32::EPSILON,
            "res was: {}",
            actual_val
        );
    }

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
    fn kdtree_add_new_node_to_empty_tree() {
        let mut tree = KDTree::empty();

        assert_eq!(0, tree.len());

        tree.add(KDPoint::new(&[0.0, 0.0]));

        assert_eq!(1, tree.len());
        assert_eq!(2, tree.dimensionality);
    }

    #[test]
    fn kdtree_add_new_node_to_tree_with_root() {
        let root = KDNode::new(KDPoint::new(&[0.0, 0.0]), 0, None, None);

        let mut tree = KDTree::new(root);

        assert_eq!(1, tree.len());
        assert_eq!(2, tree.dimensionality);

        tree.add(KDPoint::new(&[-1.0, 0.0]));

        assert_eq!(2, tree.len());
        assert_eq!(2, tree.dimensionality);
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
    fn kdtree_walk_balances_1level_tree() {
        let mut tree = KDTree::empty();

        //add some nodes
        tree.add(KDPoint::new(&[0.0, 0.0]));
        tree.add(KDPoint::new(&[-1.0, 0.0]));
        tree.add(KDPoint::new(&[1.0, 0.0]));

        // double-check insertion
        assert_eq!(3, tree.len());
        assert_eq!(2, tree.dimensionality);

        // check insertion order
        let pts: RefCell<Vec<KDPoint>> = RefCell::new(vec![]);
        tree.walk(|pt| pts.borrow_mut().push(pt.clone()));

        assert_eq!(3, pts.borrow().len());
        assert_eq!(&[0.0, 0.0], pts.borrow().get(0).unwrap().coords());
        assert_eq!(&[-1.0, 0.0], pts.borrow().get(1).unwrap().coords());
        assert_eq!(&[1.0, 0.0], pts.borrow().get(2).unwrap().coords());
    }

    #[test]
    fn partition_points_single_elem() {
        let points = vec![vec![0.0, 0.0]];

        let res = partition_points(points, 0, 2);
        assert_eq!(&[0.0, 0.0], res.0.coords());
        assert!(res.1.is_empty());
        assert!(res.2.is_empty());
    }

    #[test]
    fn partition_points_with_2points_with_left_subtree() {
        let points = vec![vec![-1.0, 0.0], vec![0.0, 0.0]];

        let res = partition_points(points, 0, 2);
        assert_eq!(&[0.0, 0.0], res.0.coords());
        assert_eq!(vec![vec![-1.0, 0.0]], res.1);
        assert!(res.2.is_empty());
    }

    #[test]
    fn partition_points_with_2points_with_right_subtree() {
        let points = vec![vec![0.0, 0.0], vec![2.0, 0.0]];

        let res = partition_points(points, 0, 2);
        assert_eq!(&[2.0, 0.0], res.0.coords());
        assert_eq!(vec![vec![0.0, 0.0]], res.1);
        assert!(res.2.is_empty());
    }

    #[test]
    fn partition_points_with_2points_with_full_tree() {
        let points = vec![vec![-1.0, 0.0], vec![2.0, 0.0], vec![0.0, 0.0]];

        let res = partition_points(points, 0, 2);
        assert_eq!(&[0.0, 0.0], res.0.coords());
        assert_eq!(vec![vec![-1.0, 0.0]], res.1);
        assert_eq!(vec![vec![2.0, 0.0]], res.2);
    }

    #[test]
    fn partition_points_with_3points_by_second_dimension() {
        let points = vec![vec![0.0, 0.0], vec![2.0, -1.0], vec![1.0, 2.0]];

        let res = partition_points(points, 1, 2);
        assert_eq!(&[0.0, 0.0], res.0.coords());
        assert_eq!(vec![vec![2.0, -1.0]], res.1);
        assert_eq!(vec![vec![1.0, 2.0]], res.2);
    }

    #[test]
    fn build_tree_example() {
        let points = vec![
            vec![0.0, 0.0],
            vec![-1.0, 0.0],
            vec![1.0, 0.0],
            vec![-1.0, -1.0],
            vec![-1.0, 1.0],
            vec![1.0, -1.0],
            vec![1.0, 1.0],
        ];
        let tree = build_tree(points);

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
    fn kdtree_nearest_with_points_on_trees() {
        let points = vec![
            vec![100.0, 100.0],
            vec![-100.0, 100.0],
            vec![100.0, -100.0],
            vec![-100.0, -100.0],
        ];

        let tree = build_tree(points);
        assert_eq!(4, tree.len());

        // pt1 should return existing node and distance should be close to 0
        let pt1 = KDPoint::new(&[100.0, 100.0]);
        let res = tree.nearest(&pt1);

        assert_approx(0.0, res.distance);
        assert_eq!(pt1.coords, res.point.coords);
        // check pt2
        let pt2 = KDPoint::new(&[-100.0, 100.0]);
        let res = tree.nearest(&pt2);

        assert_approx(0.0, res.distance);
        assert_eq!(pt2.coords, res.point.coords);

        // check pt3
        let pt3 = KDPoint::new(&[100.0, -100.0]);
        let res = tree.nearest(&pt3);

        assert_approx(0.0, res.distance);
        assert_eq!(pt3.coords, res.point.coords);

        // check pt4
        let pt4 = KDPoint::new(&[-100.0, -100.0]);
        let res = tree.nearest(&pt4);

        assert_approx(0.0, res.distance);
        assert_eq!(pt4.coords, res.point.coords);
    }

    #[test]
    fn kdtree_nearest_with_points_around_node4() {
        let points = vec![
            vec![100.0, 100.0],
            vec![-100.0, 100.0],
            vec![100.0, -100.0],
            vec![-100.0, -100.0], // it is node 4
        ];

        let expected_coords = vec![-100.0, -100.0];
        let tree = build_tree(points);
        assert_eq!(4, tree.len());

        let pt1 = KDPoint::new(&[-110.0, -100.0]);
        let res = tree.nearest(&pt1);

        assert_approx(10.0, res.distance);
        assert_eq!(expected_coords, res.point.coords);

        let pt2 = KDPoint::new(&[-90.0, -100.0]);
        let res = tree.nearest(&pt2);

        assert_approx(10.0, res.distance);
        assert_eq!(expected_coords, res.point.coords);

        let pt3 = KDPoint::new(&[-100.0, -90.0]);
        let res = tree.nearest(&pt3);

        assert_approx(10.0, res.distance);
        assert_eq!(expected_coords, res.point.coords);

        let pt4 = KDPoint::new(&[-100.0, -110.0]);
        let res = tree.nearest(&pt4);

        assert_approx(10.0, res.distance);
        assert_eq!(expected_coords, res.point.coords);
    }
}

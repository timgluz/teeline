use std::cmp::Ordering;

use super::{NearestResult, NearestResultItem};

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

    /// Nearest point satisfying `predicate`, or `None` when the tree holds no
    /// eligible point.
    ///
    /// Unlike `nearest` followed by filtering the returned `n` items, an ineligible
    /// point is *never added to the accumulator*. That distinction is the point of
    /// this method: a filtered-out point must not occupy a result slot nor raise
    /// `search_radius()`, because `search_radius()` is what prunes subtrees. If a
    /// visited point is allowed to raise the radius, the search prunes branches that
    /// may hold the true nearest unvisited point and silently returns a farther one.
    /// That is precisely how `nearest_neighbor`'s "nearest unvisited among the k
    /// nearest overall" approximation ended up picking non-nearest cities.
    ///
    /// Determinism: the *distance* returned is independent of traversal order
    /// (pruning is exact for `n = 1`), but which member of an exact-distance tie is
    /// returned is not specified — see `nearest_unvisited` for the tie-broken form.
    pub fn nearest_where(
        &self,
        target: &KDPoint,
        predicate: &mut dyn FnMut(&KDPoint) -> bool,
    ) -> Option<NearestResultItem> {
        let mut acc = NearestResult::new(*target, 1);
        if let Some(root) = &self.root {
            root.nearest_where(target, &mut acc, predicate);
        }
        acc.nearest().first().copied()
    }

    /// Nearest point satisfying `is_candidate`, ties broken by **lowest id**.
    ///
    /// The pruned counterpart of `DistanceMatrix::nearest_unvisited`, and the same
    /// contract: a predicate rather than a visited/unvisited set, so the caller states
    /// the membership test explicitly instead of relying on a parameter name to convey
    /// polarity.
    ///
    /// Deterministic in distance and identity. `nearest_where` fixes the minimum
    /// distance without letting skipped points distort pruning, but which member of an
    /// exact-distance tie it returns is unspecified, so ties are resolved here by a
    /// scan for the lowest eligible id — and only when a tie actually exists, which
    /// keeps the common path sublinear.
    pub fn nearest_unvisited(
        &self,
        target: &KDPoint,
        cities: &[KDPoint],
        is_candidate: impl Fn(usize) -> bool,
    ) -> Option<NearestResultItem> {
        let best = {
            let mut predicate = |pt: &KDPoint| pt.id != target.id && is_candidate(pt.id);
            self.nearest_where(target, &mut predicate)?
        };

        // Exact float equality is deliberate: this asks whether another candidate is
        // the same distance away, which is exactly when traversal order decides.
        let tied_lower = cities
            .iter()
            .filter(|pt| {
                pt.id < best.point.id
                    && pt.id != target.id
                    && is_candidate(pt.id)
                    && pt.distance(target) == best.distance
            })
            .map(|pt| pt.id)
            .min();

        match tied_lower {
            Some(id) => {
                let pt = cities.iter().find(|c| c.id == id)?;
                Some(NearestResultItem::new(*pt, best.distance))
            }
            None => Some(best),
        }
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

    /// `nearest` with a candidate predicate: ineligible points are skipped entirely,
    /// so they neither fill a result slot nor raise `search_radius()`.
    ///
    /// The pruning guard is unchanged, and deliberately uses a strict `>`: a branch
    /// exactly `search_radius()` away is still visited, so points tied at the current
    /// best distance are always reachable and cannot be missed because of where the
    /// tree happened to split. Skipping ineligible points here — rather than
    /// filtering `nearest`'s output — is what keeps the pruning radius honest.
    fn nearest_where(
        &self,
        target_point: &KDPoint,
        acc: &mut NearestResult,
        predicate: &mut dyn FnMut(&KDPoint) -> bool,
    ) {
        if predicate(&self.point) {
            acc.add(self.point, self.point.distance(target_point));
        }

        let (closest_branch, further_branch) = match self.cmp_by_point(target_point) {
            None => panic!("Dimension conflict in nearest function"),
            Some(Ordering::Greater) => (self.left(), self.right()),
            Some(_) => (self.right(), self.left()),
        };

        if let Some(branch) = closest_branch {
            branch.nearest_where(target_point, acc, predicate);
        }

        let split_dist = self.point.split_distance(target_point, self.level_coord());
        if acc.search_radius() > split_dist
            && let Some(branch) = further_branch
        {
            branch.nearest_where(target_point, acc, predicate);
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
    fn nearest_where_skips_ineligible_points_without_distorting_pruning() {
        // Four collinear points at x = 0, 10, 20, 30. Querying from x=1 for the
        // nearest *eligible* point must return 10 even when 0 is excluded, and must
        // not be fooled into returning 20 because 0 filled the result buffer.
        let cities = build_points(&[
            vec![0.0, 0.0],
            vec![10.0, 0.0],
            vec![20.0, 0.0],
            vec![30.0, 0.0],
        ]);
        let tree = from_cities(&cities);
        let target = KDPoint::new(&[1.0, 0.0]);

        // Exclude the two nearest (x=0 at 1.0 and x=10 at 9.0): expect x=20.
        let excluded = [cities[0].id, cities[1].id];
        let pick = tree
            .nearest_where(&target, &mut |pt| !excluded.contains(&pt.id))
            .expect("two points remain eligible");
        assert_eq!(pick.point.id, cities[2].id);
        assert_approx(19.0, pick.distance);

        // Excluding everything yields None rather than a stale candidate.
        assert!(
            tree.nearest_where(&target, &mut |_| false).is_none(),
            "an empty candidate set must yield None"
        );
    }

    #[test]
    fn nearest_unvisited_breaks_ties_by_lowest_id() {
        // ids 0..2: id 0 at origin, ids 1 and 2 exactly 1.0 away either side.
        let cities = build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![-1.0, 0.0]]);
        let tree = from_cities(&cities);
        let target = cities[0];

        let pick = tree
            .nearest_unvisited(&target, &cities, |id| id != target.id)
            .expect("two candidates exist");
        assert_approx(1.0, pick.distance);
        assert_eq!(
            pick.point.id, cities[1].id,
            "an exact tie must resolve to the lowest id, not traversal order"
        );
    }

    #[test]
    fn nearest_unvisited_matches_brute_force_over_eligible_points() {
        let cities = build_points(&[
            vec![0.0, 0.0],
            vec![3.0, 4.0],
            vec![1.0, 0.0],
            vec![10.0, 10.0],
            vec![0.0, 2.0],
        ]);
        let tree = from_cities(&cities);
        let target = cities[0];
        let excluded = [cities[1].id, cities[4].id];
        let is_candidate = |id: usize| id != target.id && !excluded.contains(&id);

        let pick = tree
            .nearest_unvisited(&target, &cities, is_candidate)
            .expect("candidates remain");

        let mut expected: Option<(f32, usize)> = None;
        for pt in &cities {
            if !is_candidate(pt.id) {
                continue;
            }
            let d = pt.distance(&target);
            let better = match expected {
                None => true,
                Some((bd, bi)) => (d, pt.id) < (bd, bi),
            };
            if better {
                expected = Some((d, pt.id));
            }
        }
        let (exp_d, exp_id) = expected.unwrap();
        assert_eq!(pick.point.id, exp_id);
        assert_approx(exp_d, pick.distance);
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

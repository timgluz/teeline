/*
    DistanceMatrix is a data collection for keeping euclidean distances between array of 2D points;

    Given the fact the teeline only tackles symmetrical TSP problems with only positive distances.
    We can optimize the memory footprint by just keeping triangle under main diagonal,
    and then flatten it into 1D array;

    Visual representation with 2 cities:
    step1: we have a full 3x3 matrix
    +---+---+---+
    |1_1|1_2|1_3|
    +---+---+---+
    |2_1|2_2|2_3|
    +---+---|---|
    |3_1|3_2|3_3|
    +---+---+---+

    step2: we take only items under main diagonal
    +---+
    |2_1|
    +---+---+
    |3_1|3_2|
    +---+---+

    step3: we flatten it into 1D matrix

    +---+---+---+
    |2_1|3_1|3_2|
    +---+---+---+

    As result, we have a array with  N_cities * ( N_cities - 1) / 2 elements

    *Lookup logic*
    As you probably noticed, that this represention has limitation:
        the first city id must be bigger than second city id;

    from_id = max(city1_id, city2_id)
    to_city = min(city1_id, city2_id)

    Then the lookup would calculate a padding before the from_id, which is
    the size of small triangle top of from_id; and then we add the to_city to the padding;

    Example:
        city1 = 3, city2 = 2;
        prev_city = city1 - 1
        padding = (2-1) * 2 / 2 = 1
        pos = padding + city2 = 3
        distance[pos-1] // as Rust start counting from 0

    ps: this high-complexity doesnt make anysense outside this hobby project,
    because it adds more complexity than any actual benefits;
*/

use std::collections::HashMap;

use super::kdtree::KDPoint;
use super::{CityTable, DistanceType, NearestResult, NearestResultItem};

pub(crate) fn geo_distance(p1: &KDPoint, p2: &KDPoint) -> f32 {
    // Full-precision PI, deliberately, not TSPLIB's truncated `PI = 3.141592` from the
    // FAQ. The two differ on 474 of the corpus's 517,680 GEO distances by exactly 1 unit,
    // but produce identical tour lengths for every GEO instance with a published optimum
    // (ulysses16/22, gr96/137/202/229/431/666), so the truncated constant buys no
    // comparability that matters while introducing a real error of ~1e-7 relative. The
    // FAQ's `deg = (int) x[i]` is matched: `trunc()` also rounds toward zero, which
    // matters because 916 corpus coordinates are negative.
    use std::f64::consts::PI;
    fn to_rad(x: f32) -> f64 {
        let deg = x.trunc() as f64;
        let min = (x - x.trunc()) as f64;
        PI * (deg + 5.0 * min / 3.0) / 180.0
    }
    let lat1 = to_rad(p1.coords[0]);
    let lon1 = to_rad(p1.coords[1]);
    let lat2 = to_rad(p2.coords[0]);
    let lon2 = to_rad(p2.coords[1]);
    let q1 = (lon1 - lon2).cos();
    let q2 = (lat1 - lat2).cos();
    let q3 = (lat1 + lat2).cos();
    const RRR: f64 = 6378.388;
    // Clamped because rounding can nudge the argument just outside acos's domain, where
    // it returns NaN and poisons the distance.
    let cos_angle = (0.5 * ((1.0 + q1) * q2 - (1.0 - q1) * q3)).clamp(-1.0, 1.0);
    (RRR * cos_angle.acos() + 1.0).floor() as f32
}

// to have similar builder as kdtree
pub fn from_cities(cities: &[KDPoint]) -> DistanceMatrix {
    DistanceMatrix::from_cities(cities).unwrap()
}

pub fn build(cities: &[KDPoint], dt: DistanceType) -> DistanceMatrix {
    DistanceMatrix::build(cities, dt).unwrap()
}

#[derive(Debug, Clone)]
pub struct DistanceMatrix {
    n: usize,    // how many cities
    size: usize, // how many distances under diagonal
    items: Vec<f32>,
    cities: CityTable,
    city_idx: HashMap<usize, usize>, // translates city_id to matrix_id
}

impl DistanceMatrix {
    pub fn new(n: usize, distances: Vec<f32>, cities: CityTable) -> Self {
        let city_idx: HashMap<usize, usize> = cities.iter().map(|(k, c)| (c.id, *k)).collect();

        assert!(n == city_idx.len(), "city_idx size differs from n cities");
        assert_eq!(
            distances.len(),
            n * (n - 1) / 2,
            "distances length {} != n*(n-1)/2={} for n={}",
            distances.len(),
            n * (n - 1) / 2,
            n
        );
        DistanceMatrix {
            n,
            size: distances.len(),
            items: distances,
            cities,
            city_idx,
        }
    }

    // assumes that cities are already sorted by id and ids are incrementally crowing
    pub fn from_cities(cities: &[KDPoint]) -> Result<Self, &'static str> {
        Self::build(cities, DistanceType::Euc2D)
    }

    pub fn build(cities: &[KDPoint], distance_type: DistanceType) -> Result<Self, &'static str> {
        let n = cities.len();
        if n < 2 {
            return Err("distance matrix requires at least 2 points");
        }

        let size = n * (n - 1) / 2; // how many items on distance vec

        let mut city_table = CityTable::new();

        let mut distances = Vec::with_capacity(size);
        for (i, pt1) in cities.iter().enumerate() {
            city_table.insert(i, *pt1);

            for pt2 in cities.iter().take(i) {
                let d = match distance_type {
                    DistanceType::Euc2D => pt1.distance(pt2),
                    DistanceType::Geo => geo_distance(pt1, pt2),
                    DistanceType::Explicit => {
                        return Err(
                            "cannot build distance matrix from coordinates for EXPLICIT type — use DistanceMatrix::new() with precomputed distances",
                        );
                    }
                };
                distances.push(d);
            }
        }

        distances.shrink_to_fit();

        Ok(DistanceMatrix::new(n, distances, city_table))
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn num_cities(&self) -> usize {
        self.n
    }

    pub fn num_distances(&self) -> usize {
        self.size
    }

    pub fn distances(&self) -> &[f32] {
        &self.items
    }

    // It returns distance by raw vector ids for backward support for some solutions
    // preferred solution: distance_between as it checks if city exists on table
    pub fn distance_by_pos(&self, pos1: usize, pos2: usize) -> Result<f32, &'static str> {
        if pos1 == pos2 {
            return Ok(0.0);
        }
        if pos1 >= self.n || pos2 >= self.n {
            return Err("position out of range");
        }
        let from = pos1.max(pos2);
        let to = pos1.min(pos2);
        let idx = from * (from - 1) / 2 + to;
        self.items
            .get(idx)
            .copied()
            .ok_or("distance index out of range")
    }

    /// returns distance between city n and m
    /// array is packed version of bottom triangle with given structure
    /// ||d2,1|d3,1|d3,2|d4,1|d4,2|d4,3||
    /// here bigger cityId works like padding, then smaller id acts as index from padding
    pub fn distance_between(&self, city_id1: usize, city_id2: usize) -> Result<f32, &'static str> {
        if city_id1 == city_id2 {
            return Ok(0.0);
        }
        let pos1 = self
            .city_idx
            .get(&city_id1)
            .copied()
            .ok_or("city_id1 not in index")?;
        let pos2 = self
            .city_idx
            .get(&city_id2)
            .copied()
            .ok_or("city_id2 not in index")?;
        self.distance_by_pos(pos1, pos2)
    }

    /// returns list of distances from city N, where 0 distance from the city;
    pub fn distances_from(&self, city_id: usize) -> Vec<f32> {
        let pos: usize = *self.city_idx.get(&city_id).expect("Unknown city id");

        self.distances_from_index(pos)
    }

    pub fn tour_length(&self, path: &[usize]) -> f32 {
        if path.len() < 2 {
            return 0.0;
        }
        // Translate city IDs to positions upfront; unknown city ID → return 0.0.
        let positions: Option<Vec<usize>> = path.iter().map(|&id| self.city_id2pos(id)).collect();
        match positions {
            None => 0.0,
            Some(pos_path) => self.tour_length_by_pos(&pos_path),
        }
    }

    /// Position-based tour length — no HashMap lookups per edge.
    /// All positions must be in 0..num_cities().
    pub fn tour_length_by_pos(&self, path: &[usize]) -> f32 {
        if path.len() < 2 {
            return 0.0;
        }
        let last = *path.last().unwrap();
        let mut total = self.distance_by_pos(last, path[0]).unwrap_or(0.0);
        for w in path.windows(2) {
            total += self.distance_by_pos(w[0], w[1]).unwrap_or(0.0);
        }
        total
    }

    pub fn city_index(&self) -> &HashMap<usize, usize> {
        &self.city_idx
    }

    pub fn pos2city_id(&self, pos: usize) -> Option<usize> {
        self.cities.get(&pos).map(|c| c.id)
    }

    pub fn city_id2pos(&self, city_id: usize) -> Option<usize> {
        self.city_idx.get(&city_id).copied()
    }

    pub fn nearest(&self, target: &KDPoint, n: usize) -> NearestResult {
        let mut search_result = NearestResult::new(*target, n);

        let city_pos = match self.city_id2pos(target.id) {
            Some(pos) => pos,
            None => return search_result, // unknown target → empty result, no panic
        };

        let distances_from_target = self.distances_from_index(city_pos);
        for (pos, distance) in distances_from_target.iter().enumerate() {
            if pos == city_pos {
                continue; // skip self (belt-and-suspenders; add() already gates on pt.id)
            }
            // Look up by matrix position, not city_id.  city_id != pos when city IDs
            // are not 0-based (e.g. 1-indexed TSPLIB files like berlin52.tsp).
            if let Some(pt) = self.cities.get(&pos) {
                search_result.add(*pt, *distance);
            }
        }

        search_result
    }

    /// Nearest city satisfying `is_candidate`, ties broken by lowest city id.
    ///
    /// A predicate rather than a visited/unvisited set: taking `&HashSet<usize> visited`
    /// let a caller pass its `unvisited` set without the compiler noticing, inverting the
    /// test and returning `None` for every query.
    ///
    /// `target` is excluded internally, so `|id| unvisited.contains(&id)` is already
    /// correct and a caller's extra `id != current_id` is harmless.
    ///
    /// O(n): the whole row is scanned. Deterministic by the total order
    /// `(distance, city_id)` rather than iteration order, which is what makes `nn`
    /// reproducible.
    pub fn nearest_unvisited(
        &self,
        target: &KDPoint,
        is_candidate: impl Fn(usize) -> bool,
    ) -> Option<NearestResultItem> {
        let city_pos = self.city_id2pos(target.id)?;
        let row = self.distances_from_index(city_pos);

        let mut best: Option<NearestResultItem> = None;
        for (pos, distance) in row.iter().enumerate() {
            if pos == city_pos {
                continue;
            }
            // Unreachable: `build()`/`new()` populate `cities` for every position 0..n.
            // Skipped to match `nearest()` above rather than asserting.
            let Some(pt) = self.cities.get(&pos) else {
                continue;
            };
            if !is_candidate(pt.id) {
                continue;
            }
            let better = match &best {
                None => true,
                // `total_cmp`, not `<`: the latter is false against NaN, so a non-finite
                // distance reaching `best` would block every later candidate and win by
                // default. `total_cmp` is a genuine total order, so the result stays
                // deterministic even on malformed input.
                Some(current) => distance
                    .total_cmp(&current.distance)
                    .then_with(|| pt.id.cmp(&current.point.id))
                    .is_lt(),
            };
            if better {
                best = Some(NearestResultItem::new(*pt, *distance));
            }
        }

        best
    }

    fn distances_from_index(&self, pos: usize) -> Vec<f32> {
        let mut distances = Vec::with_capacity(self.n);
        for i in 0..pos {
            distances.push(
                self.distance_by_pos(pos, i)
                    .expect("valid position in distances_from_index"),
            );
        }
        for i in pos..self.n {
            distances.push(
                self.distance_by_pos(i, pos)
                    .expect("valid position in distances_from_index"),
            );
        }
        distances
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::assert_approx;
    use crate::tsp::kdtree;

    #[test]
    fn nearest_unvisited_ignores_a_non_finite_distance() {
        // Hand-built matrix: distance(0,1) = NaN, distance(0,2) = 5, distance(1,2) = 1.
        // Packed lower triangle is [d(1,0), d(2,0), d(2,1)].
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![2.0, 0.0]]);
        let table: crate::tsp::CityTable =
            cities.iter().enumerate().map(|(i, c)| (i, *c)).collect();
        let dm = DistanceMatrix::new(3, vec![f32::NAN, 5.0, 1.0], table);

        // City 1 must be skipped despite being "closest" only in the sense that NaN loses
        // every ordinary comparison — a NaN in `best` would otherwise stick forever.
        let pick = dm
            .nearest_unvisited(&cities[0], |id| id != cities[0].id)
            .expect("city 2 is reachable");
        assert_eq!(pick.point.id, cities[2].id);
        assert_approx(5.0, pick.distance);
    }

    #[test]
    fn geo_distance_is_finite_and_correct() {
        // burma14 declares EDGE_WEIGHT_TYPE: GEO, so this exercises the acos path that can
        // round outside its domain. Unclamped, that yields NaN distances which then poison
        // any comparison-based selection.
        use crate::tsp::{DistanceType, tsplib};
        let data = tsplib::read_from_file(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/burma14.tsp"
        )))
        .expect("burma14 fixture must parse");
        let cities = data.cities().to_vec();
        let dm = DistanceMatrix::build(&cities, DistanceType::Geo).expect("geo matrix builds");

        for a in &cities {
            for b in &cities {
                let d = dm.distance_between(a.id, b.id).unwrap();
                assert!(d.is_finite(), "geo distance {} -> {} is {d}", a.id, b.id);
                assert!(
                    d >= 0.0,
                    "geo distance {} -> {} is negative: {d}",
                    a.id,
                    b.id
                );
            }
        }
    }

    /// Guards a real gap: the parser maps any unrecognised EDGE_WEIGHT_TYPE to the
    /// default EUC_2D (`parse::<DistanceType>().ok().unwrap_or_default()`), and
    /// `DistanceType` has no ATT or CEIL_2D variant. So att48/att532/dsj1000 are currently
    /// measured as if they were planar, and comparing their tour lengths to TSPLIB's
    /// published optima is not meaningful. If this test starts failing, the formula was
    /// implemented and the affected benchmark numbers need recomputing.
    #[test]
    fn att_is_not_silently_measured_as_euclidean() {
        use crate::tsp::tsplib;
        let data = tsplib::read_from_file(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/att48.tsp"
        )))
        .expect("att48 fixture must parse");
        assert_eq!(
            data.distance_type,
            crate::tsp::DistanceType::Euc2D,
            "att48 now uses a non-Euclidean formula: recompute the ATT benchmark rows and \
             update or delete this guard"
        );
    }

    #[test]
    fn nearest_unvisited_takes_the_closest_candidate() {
        // Collinear 5-city instance: ids 0..4 at x = 0,1,2,3,4. Starting from id 0,
        // excluding 0, the nearest candidate is id 1.
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![2.0, 0.0],
            vec![3.0, 0.0],
            vec![4.0, 0.0],
        ]);
        let dm = DistanceMatrix::from_cities(&cities).unwrap();
        let start = cities[0].id;

        let pick = dm.nearest_unvisited(&cities[0], |id| id != start).unwrap();
        assert_eq!(pick.point.id, cities[1].id);
        assert_approx(1.0, pick.distance);

        // Excluding the three nearest leaves id 4.
        let excluded: Vec<usize> = vec![start, cities[1].id, cities[2].id, cities[3].id];
        let pick = dm
            .nearest_unvisited(&cities[0], |id| !excluded.contains(&id))
            .unwrap();
        assert_eq!(pick.point.id, cities[4].id);
        assert_approx(4.0, pick.distance);
    }

    #[test]
    fn nearest_unvisited_breaks_ties_by_lowest_city_id() {
        // id 0 at origin; ids 1 and 2 are both exactly 1.0 away.
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0], vec![-1.0, 0.0]]);
        let dm = DistanceMatrix::from_cities(&cities).unwrap();
        let start = cities[0].id;

        let pick = dm.nearest_unvisited(&cities[0], |id| id != start).unwrap();
        assert_approx(1.0, pick.distance);
        assert_eq!(
            pick.point.id, cities[1].id,
            "exact tie must resolve to the lowest city id, never iteration order"
        );
    }

    #[test]
    fn nearest_unvisited_returns_none_when_nothing_is_eligible() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![1.0, 0.0]]);
        let dm = DistanceMatrix::from_cities(&cities).unwrap();

        assert!(
            dm.nearest_unvisited(&cities[0], |_| false).is_none(),
            "an empty candidate set must yield None rather than a bogus city"
        );
    }

    /// The regression that the predicate API exists to prevent.
    ///
    /// A `HashSet`-based signature made it possible to pass `unvisited` where the
    /// function expected `visited`, and every unit test above still passed because
    /// their excluded sets were trivial. This test drives the real fixture with the
    /// real polarity a caller uses, and checks the answer against an independent
    /// reference built from the public id-based distance API.
    #[test]
    fn nearest_unvisited_matches_reference_on_real_a280_data() {
        use crate::tsp::tsplib;

        // tests/fixtures/, not data/tsplib/: the latter is git-ignored and only present
        // after `download_data.sh`, so pointing at it would fail on a fresh checkout and
        // in CI. tests/fixtures/a280.tsp is tracked.
        let problem = tsplib::read_from_file(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/a280.tsp"
        )))
        .expect("a280 fixture must parse");
        let cities = problem.cities().to_vec();
        let dm = DistanceMatrix::from_cities(&cities).unwrap();

        let start = cities[0].id;
        let unvisited: std::collections::HashSet<usize> = cities
            .iter()
            .map(|c| c.id)
            .filter(|id| *id != start)
            .collect();
        assert_eq!(unvisited.len(), cities.len() - 1);

        let pick = dm
            .nearest_unvisited(&cities[0], |id| id != start && unvisited.contains(&id))
            .expect("279 cities are unvisited, so a nearest must exist");
        assert!(
            unvisited.contains(&pick.point.id),
            "must not return an already-visited city"
        );

        // Independent reference: scan the same candidate set via the public
        // id-addressed API and take the min by (distance, id).
        let mut reference: Option<(f32, usize)> = None;
        for &id in &unvisited {
            let d = dm.distance_between(start, id).unwrap_or(f32::MAX);
            let better = match reference {
                None => true,
                Some((bd, bi)) => (d, id) < (bd, bi),
            };
            if better {
                reference = Some((d, id));
            }
        }
        let (ref_dist, ref_id) = reference.unwrap();
        assert_eq!(
            pick.point.id, ref_id,
            "predicate query disagreed with the id-based reference"
        );
        assert_approx(ref_dist, pick.distance);
    }

    #[test]
    fn test_build_distance_matrix_from_empty_list() {
        let cities = kdtree::build_points(&vec![]);

        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_err());
    }

    #[test]
    fn test_build_distance_matrix_from_singleton_list() {
        let cities = kdtree::build_points(&[vec![100.0, 100.0]]);

        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_err());
    }

    #[test]
    fn test_build_distance_matrix_from_2_item_list() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![0.0, 1.0]]);

        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(&[1.0], res.distances());
    }

    #[test]
    fn test_build_distance_matrix_from_3_item_list() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![0.0, 1.0], vec![2.0, 0.0]]);

        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(3, res.len());
        assert_approx(1.0, res.distances()[0]); // from 1 to 2
        assert_approx(2.0, res.distances()[1]); // from 1 to 3
        assert_approx(2.236_068, res.distances()[2]); // from 2 to 3
    }

    #[test]
    fn test_distance_matrix_distance_between_with_3_cities_example() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![0.0, 1.0], vec![2.0, 0.0]]);
        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_ok());
        let dm = res.unwrap();
        let d23 = 2.236_068;

        // city id starts from 0
        assert_approx(1.0, dm.distance_between(0, 1).unwrap());
        assert_approx(1.0, dm.distance_between(1, 0).unwrap());
        assert_approx(2.0, dm.distance_between(0, 2).unwrap());
        assert_approx(2.0, dm.distance_between(2, 0).unwrap());
        assert_approx(d23, dm.distance_between(1, 2).unwrap());
        assert_approx(d23, dm.distance_between(2, 1).unwrap());
    }

    #[test]
    fn test_distance_matrix_distance_between_with_4_cities_example() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![2.0, 0.0],
            vec![4.0, 0.0],
        ]);
        let res = DistanceMatrix::from_cities(&cities);

        assert!(res.is_ok());
        let dm = res.unwrap();
        let d23 = 2.236_068;
        let d31 = 4.123_1055;

        assert_eq!(6, dm.len());

        // city id starts from 0
        assert_approx(1.0, dm.distance_between(1, 0).unwrap());
        assert_approx(2.0, dm.distance_between(2, 0).unwrap());
        assert_approx(d23, dm.distance_between(2, 1).unwrap());
        assert_approx(4.0, dm.distance_between(3, 0).unwrap());
        assert_approx(d31, dm.distance_between(3, 1).unwrap());
        assert_approx(2.0, dm.distance_between(3, 2).unwrap())
    }

    #[test]
    fn test_distance_matrix_distances_from_city1() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![2.0, 0.0],
            vec![4.0, 0.0],
        ]);
        let dm = DistanceMatrix::from_cities(&cities).unwrap();

        let res = dm.distances_from(0);
        assert_eq!(4, res.len());
        assert_approx(0.0, res[0]);
        assert_approx(1.0, res[1]);
        assert_approx(2.0, res[2]);
        assert_approx(4.0, res[3]);
    }

    #[test]
    fn test_distance_matrix_tour_length_with_tsp_5_1() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);

        let route = vec![0, 1, 2, 3, 4];
        let dm = DistanceMatrix::from_cities(&cities).unwrap();

        assert_approx(4.0, dm.tour_length(&route));
    }

    #[test]
    fn test_nearest_for_tsp_5_1() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);

        let dm = from_cities(&cities);

        let res = dm.nearest(&cities[0], 3);
        assert_eq!(cities[1].id, res.closest_point().unwrap().id);

        let res2 = dm.nearest(&cities[1], 3);
        assert_eq!(cities[0].id, res2.closest_point().unwrap().id);

        let res3 = dm.nearest(&cities[2], 3);
        assert_eq!(cities[1].id, res3.closest_point().unwrap().id);

        let res4 = dm.nearest(&cities[3], 3);
        assert_eq!(cities[2].id, res4.closest_point().unwrap().id);

        let res5 = dm.nearest(&cities[4], 2);
        assert_eq!(cities[0].id, res5.closest_point().unwrap().id);
    }

    #[test]
    fn geo_distance_matches_a_published_optimum() {
        // GEO had no fixture with a known optimum, so nothing in CI pinned the great-circle
        // path to external truth — only a `> 100.0` range check on burma14. This anchors it:
        // ulysses16's published optimum is 6859, and its optimal tour must measure exactly
        // that.
        //
        // It also pins the PI choice. Full-precision PI and TSPLIB's truncated 3.141592
        // disagree on individual distances (by 1) yet agree on this total, so this test
        // alone does not force the constant — it is here so a GEO distance regression of
        // any real size fails loudly.
        use crate::tsp::{DistanceType, opt_tour, tsplib};
        let data = tsplib::read_from_file(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ulysses16.tsp"
        )))
        .expect("ulysses16 fixture must parse");
        assert_eq!(data.distance_type, DistanceType::Geo, "fixture is not GEO");

        let dm = data.distance_matrix().expect("geo distance matrix builds");
        let tour = opt_tour::read_from_file(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ulysses16.opt.tour"
        )))
        .expect("ulysses16.opt.tour must parse; without it this test asserts nothing");

        assert_approx(6859.0, dm.tour_length(&tour.route));
    }

    #[test]
    fn test_geo_distance_known_pair() {
        // Issue #127 reference pair: cities A=(16.47, 96.10) & B=(23.70, 96.99)
        // TSPLIB FAQ great-circle formula (trunc-based, full-precision PI) yields 837.
        let a = KDPoint::new_with_id(1, &[16.47, 96.10]);
        let b = KDPoint::new_with_id(2, &[23.70, 96.99]);
        assert_eq!(geo_distance(&a, &b), 837.0);
    }

    // Regression: nearest must work when city IDs are 1-based (as in TSPLIB files like berlin52).
    // The old code used city_id as a matrix-position key; since city_id != matrix_pos for
    // 1-indexed cities, it returned the wrong city with distance 0.0, making every "nearest"
    // the immediately next city in the ID sequence and turning NN into a no-op sorted walk.
    #[test]
    fn test_nearest_with_one_indexed_city_ids() {
        let cities = vec![
            KDPoint::new_with_id(1, &[0.0, 0.0]),
            KDPoint::new_with_id(2, &[10.0, 0.0]), // far
            KDPoint::new_with_id(3, &[0.0, 1.0]),  // closest to city 1
        ];
        let dm = from_cities(&cities);

        let res = dm.nearest(&cities[0], 2); // nearest to city 1
        // city 3 (distance 1.0) must be reported closer than city 2 (distance 10.0)
        assert_eq!(
            res.nearest().first().unwrap().point.id,
            3,
            "nearest to city 1 should be city 3, not city 2"
        );
    }
}

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

use std::borrow::Cow;

use super::DistanceType;
use super::distance_source::DistanceSource;
use super::id_index::{IdIndex, unique_city_ids};
use super::kdtree::KDPoint;

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

/// TSPLIB EUC_2D: Euclidean distance rounded to the nearest integer.
///
/// Integer-valued, like `att_distance` and `ceil_2d_distance` below. Leaving it unrounded made
/// teeline optimise a different objective than the one `EDGE_WEIGHT_TYPE: EUC_2D` names, so its
/// costs were not comparable with any published optimum: `eil76`'s own optimal route scored 545.39
/// against a published optimum of 538, a floor no solver could ever have beaten.
///
/// This is deliberately *not* `KDPoint::distance`, which stays the raw Euclidean distance: that one
/// also answers "are these the same point" (`distance < f32::EPSILON`), where rounding would make
/// distinct points coincide.
pub(crate) fn euc_2d_distance(p1: &KDPoint, p2: &KDPoint) -> f32 {
    // Widened before subtracting, for the same reason as the neighbours below: `round` sits on an
    // integer boundary that an f32-rounded difference could cross.
    let dx = p1.x() as f64 - p2.x() as f64;
    let dy = p1.y() as f64 - p2.y() as f64;
    (dx * dx + dy * dy).sqrt().round() as f32
}

/// TSPLIB ATT ("pseudo-Euclidean"): round `r = sqrt((dx^2 + dy^2) / 10)` to the nearest
/// integer, then add one when rounding went *down*, so the result is never below `r`.
pub(crate) fn att_distance(p1: &KDPoint, p2: &KDPoint) -> f32 {
    // Widened before subtracting: `(p1.x() - p2.x()) as f64` would round the difference in
    // f32 first, and this result is compared against an integer boundary (`t < r`), so the
    // reference semantics are worth keeping for fractional coordinates.
    let dx = p1.x() as f64 - p2.x() as f64;
    let dy = p1.y() as f64 - p2.y() as f64;
    // Divided before the sqrt, matching the spec's `sqrt((dx^2 + dy^2) / 10)` and the
    // reference implementations, rather than the algebraically-equal `sqrt(d2) / sqrt(10)`.
    let r = ((dx * dx + dy * dy) / 10.0).sqrt();
    let t = r.round();
    (if t < r { t + 1.0 } else { t }) as f32
}

/// TSPLIB CEIL_2D: Euclidean distance rounded up.
pub(crate) fn ceil_2d_distance(p1: &KDPoint, p2: &KDPoint) -> f32 {
    // Widened before subtracting, for the same reason as `att_distance`: the `ceil` sits on
    // an integer boundary that an f32-rounded difference could cross.
    let dx = p1.x() as f64 - p2.x() as f64;
    let dy = p1.y() as f64 - p2.y() as f64;
    (dx * dx + dy * dy).sqrt().ceil() as f32
}

// to have similar builder as kdtree
pub fn from_cities(cities: &[KDPoint]) -> DistanceMatrix {
    DistanceMatrix::from_cities(cities).unwrap()
}

/// Plain Euclidean, unrounded — see `DistanceMatrix::from_cities_unrounded`. For tests.
#[doc(hidden)]
pub fn from_cities_unrounded(cities: &[KDPoint]) -> DistanceMatrix {
    DistanceMatrix::from_cities_unrounded(cities).unwrap()
}

pub fn build(cities: &[KDPoint], dt: DistanceType) -> DistanceMatrix {
    DistanceMatrix::build(cities, dt).unwrap()
}

/// Packed lower-triangle storage.
///
/// Coordinate matrices are computed as `f32` and stay there. EXPLICIT files usually carry
/// integer weights that fit in `u16`, which halves the stored footprint without changing any
/// query result. Wider weights stay `f32`: `u32` is the same four bytes and would only add a
/// conversion on every lookup.
#[derive(Debug, Clone)]
enum PackedDistances {
    F32(Vec<f32>),
    U16(Vec<u16>),
}

impl PackedDistances {
    fn from_explicit(distances: Vec<f32>) -> Self {
        if Self::fits_u16(&distances) {
            Self::U16(distances.iter().map(|&d| d as u16).collect())
        } else {
            Self::F32(distances)
        }
    }

    fn from_explicit_slice(distances: &[f32]) -> Self {
        if Self::fits_u16(distances) {
            Self::U16(distances.iter().map(|&d| d as u16).collect())
        } else {
            Self::F32(distances.to_vec())
        }
    }

    /// True when every value is a non-negative integer ≤ `u16::MAX`.
    ///
    /// `-0.0` is rejected so the sign bit survives; the compact representation has no way to
    /// carry it, and "preserve every distance exactly" is the contract.
    fn fits_u16(distances: &[f32]) -> bool {
        distances.iter().all(|&distance| {
            distance.is_finite()
                && !distance.is_sign_negative()
                && distance.fract() == 0.0
                && distance <= f32::from(u16::MAX)
        })
    }

    fn len(&self) -> usize {
        match self {
            Self::F32(items) => items.len(),
            Self::U16(items) => items.len(),
        }
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get(&self, index: usize) -> Option<f32> {
        match self {
            Self::F32(items) => items.get(index).copied(),
            Self::U16(items) => items.get(index).map(|&value| f32::from(value)),
        }
    }

    fn widen(&self) -> Cow<'_, [f32]> {
        match self {
            Self::F32(items) => Cow::Borrowed(items),
            Self::U16(items) => Cow::Owned(items.iter().map(|&value| f32::from(value)).collect()),
        }
    }

    fn bytes(&self) -> usize {
        self.len() * self.element_bytes()
    }

    fn element_bytes(&self) -> usize {
        match self {
            Self::F32(_) => 4,
            Self::U16(_) => 2,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::F32(_) => "f32",
            Self::U16(_) => "u16",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DistanceMatrix {
    n: usize, // how many cities
    items: PackedDistances,
    cities: Vec<KDPoint>, // position-indexed, same order as the matrix rows
    city_idx: IdIndex,    // translates city_id to matrix position
    is_coordinate_metric: bool,
}

impl DistanceMatrix {
    /// Build an EXPLICIT matrix from packed lower-triangle weights.
    ///
    /// Values that fit in `u16` are stored at two bytes each; everything else stays `f32`.
    /// Both representations preserve every value exactly.
    pub fn new(n: usize, distances: Vec<f32>, cities: Vec<KDPoint>) -> Result<Self, &'static str> {
        Self::from_parts(n, PackedDistances::from_explicit(distances), cities, false)
    }

    /// Like [`Self::new`], but packs straight from a borrowed slice, so a caller holding the
    /// parsed weights does not have to clone them first.
    pub fn from_explicit_slice(
        n: usize,
        distances: &[f32],
        cities: Vec<KDPoint>,
    ) -> Result<Self, &'static str> {
        Self::from_parts(
            n,
            PackedDistances::from_explicit_slice(distances),
            cities,
            false,
        )
    }

    fn from_parts(
        n: usize,
        items: PackedDistances,
        cities: Vec<KDPoint>,
        is_coordinate_metric: bool,
    ) -> Result<Self, &'static str> {
        if cities.len() != n {
            return Err("city count does not match n");
        }
        let city_idx = IdIndex::build(&cities)?;

        let expected = n
            .checked_sub(1)
            .and_then(|previous| n.checked_mul(previous))
            .map(|pairs| pairs / 2)
            .unwrap_or(0);
        if items.len() != expected {
            return Err("distance count does not match n*(n-1)/2");
        }
        Ok(DistanceMatrix {
            n,
            items,
            cities,
            city_idx,
            is_coordinate_metric,
        })
    }

    fn new_with_coordinates(
        n: usize,
        distances: Vec<f32>,
        cities: Vec<KDPoint>,
        is_coordinate_metric: bool,
    ) -> Result<Self, &'static str> {
        Self::from_parts(
            n,
            PackedDistances::F32(distances),
            cities,
            is_coordinate_metric,
        )
    }

    // assumes that cities are already sorted by id and ids are incrementally crowing
    pub fn from_cities(cities: &[KDPoint]) -> Result<Self, &'static str> {
        Self::build(cities, DistanceType::Euc2D)
    }

    /// Plain Euclidean distances — **not** TSPLIB's `EUC_2D`, which rounds to the nearest integer.
    ///
    /// An addition rather than a change, so `from_cities` keeps building the metric that
    /// `EDGE_WEIGHT_TYPE: EUC_2D` names; `teeline-api` and `teeline-wasm` call it directly and must
    /// stay conformant.
    ///
    /// This exists for tests that exercise tour *mechanics* on coordinates whose rounded distances
    /// collapse. The five-city chain the solver tests share has every pairwise distance round to 1,
    /// so every tour costs the same and a test can no longer tell a good tour from a bad one.
    #[doc(hidden)]
    pub fn from_cities_unrounded(cities: &[KDPoint]) -> Result<Self, &'static str> {
        Self::build_with(cities, |a, b| a.distance(b))
    }

    pub fn build(cities: &[KDPoint], distance_type: DistanceType) -> Result<Self, &'static str> {
        if distance_type == DistanceType::Explicit {
            return Err(
                "cannot build distance matrix from coordinates for EXPLICIT type — use DistanceMatrix::new() with precomputed distances",
            );
        }
        Self::build_with(cities, |pt1, pt2| match distance_type {
            DistanceType::Euc2D => euc_2d_distance(pt1, pt2),
            DistanceType::Geo => geo_distance(pt1, pt2),
            DistanceType::Att => att_distance(pt1, pt2),
            DistanceType::Ceil2D => ceil_2d_distance(pt1, pt2),
            DistanceType::Explicit => unreachable!("rejected above"),
        })
    }

    fn build_with(
        cities: &[KDPoint],
        distance: impl Fn(&KDPoint, &KDPoint) -> f32,
    ) -> Result<Self, &'static str> {
        let n = cities.len();
        if n < 2 {
            return Err("distance matrix requires at least 2 points");
        }
        // Reject duplicate ids before building the O(n^2) distance vector.
        unique_city_ids(cities)?;

        let size = n * (n - 1) / 2; // how many items on distance vec

        let mut distances = Vec::with_capacity(size);
        for (i, pt1) in cities.iter().enumerate() {
            for pt2 in cities.iter().take(i) {
                distances.push(distance(pt1, pt2));
            }
        }

        distances.shrink_to_fit();

        DistanceMatrix::new_with_coordinates(n, distances, cities.to_vec(), true)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn num_distances(&self) -> usize {
        self.items.len()
    }

    /// Expand the packed lower triangle to `f32` values.
    ///
    /// A coordinate or wide EXPLICIT matrix borrows its storage; a `u16` EXPLICIT matrix
    /// allocates the widened copy. Use [`Self::stored_bytes`] or [`Self::storage_kind`] for the
    /// actual footprint.
    pub fn distances(&self) -> Cow<'_, [f32]> {
        self.items.widen()
    }

    /// Bytes actually held by the packed lower triangle.
    pub fn stored_bytes(&self) -> usize {
        self.items.bytes()
    }

    /// Bytes per stored distance: `2` for `u16`, `4` for `f32`.
    pub fn storage_width(&self) -> usize {
        self.items.element_bytes()
    }

    /// `"u16"` or `"f32"` — the storage chosen for this matrix.
    pub fn storage_kind(&self) -> &'static str {
        self.items.kind()
    }
}

impl DistanceSource for DistanceMatrix {
    fn num_cities(&self) -> usize {
        self.n
    }

    fn distance_by_pos(&self, pos1: usize, pos2: usize) -> Result<f32, &'static str> {
        if pos1 == pos2 {
            return Ok(0.0);
        }
        if pos1 >= self.n || pos2 >= self.n {
            return Err("position out of range");
        }
        let from = pos1.max(pos2);
        let to = pos1.min(pos2);
        let idx = from * (from - 1) / 2 + to;
        self.items.get(idx).ok_or("distance index out of range")
    }

    /// Hot-path override: the default would dispatch `city_id2pos` twice and
    /// `distance_by_pos` once through the vtable for every lookup. This one resolves the
    /// index directly and calls the primitive statically. The conformance suite proves it
    /// returns the same values as the default.
    fn distance_between(&self, city_id1: usize, city_id2: usize) -> Result<f32, &'static str> {
        if city_id1 == city_id2 {
            return Ok(0.0);
        }
        let pos1 = self.city_idx.get(city_id1).ok_or("city_id1 not in index")?;
        let pos2 = self.city_idx.get(city_id2).ok_or("city_id2 not in index")?;
        self.distance_by_pos(pos1, pos2)
    }

    fn pos2city_id(&self, pos: usize) -> Option<usize> {
        self.cities.get(pos).map(|c| c.id)
    }

    fn city_id2pos(&self, city_id: usize) -> Option<usize> {
        self.city_idx.get(city_id)
    }

    fn point_at(&self, pos: usize) -> Option<KDPoint> {
        self.cities.get(pos).copied()
    }

    fn is_coordinate_metric(&self) -> bool {
        self.is_coordinate_metric
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
        let dm = DistanceMatrix::new(3, vec![f32::NAN, 5.0, 1.0], cities.clone()).unwrap();

        // City 1 must be skipped despite being "closest" only in the sense that NaN loses
        // every ordinary comparison — a NaN in `best` would otherwise stick forever.
        let pick = dm
            .nearest_unvisited(&cities[0], &|id| id != cities[0].id)
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

        let pick = dm.nearest_unvisited(&cities[0], &|id| id != start).unwrap();
        assert_eq!(pick.point.id, cities[1].id);
        assert_approx(1.0, pick.distance);

        // Excluding the three nearest leaves id 4.
        let excluded: Vec<usize> = vec![start, cities[1].id, cities[2].id, cities[3].id];
        let pick = dm
            .nearest_unvisited(&cities[0], &|id| !excluded.contains(&id))
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

        let pick = dm.nearest_unvisited(&cities[0], &|id| id != start).unwrap();
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
            dm.nearest_unvisited(&cities[0], &|_| false).is_none(),
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
            .nearest_unvisited(&cities[0], &|id| id != start && unvisited.contains(&id))
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
    fn att_distance_follows_the_tsplib_definition() {
        // TSPLIB ATT: r = sqrt((dx^2 + dy^2) / 10); t = nint(r); d = t, or t + 1 when the
        // rounding went down.
        let city = |id: usize, x: f32, y: f32| kdtree::KDPoint::new_with_id(id, &[x, y]);

        // r = sqrt(100/10) = 3.16228 -> nint 3, and 3 < r -> 4.
        assert_approx(4.0, att_distance(&city(0, 0.0, 0.0), &city(1, 0.0, 10.0)));
        // r = sqrt(500/10) = 7.07107 -> 7 < r -> 8. The branch a plain `round` gets wrong.
        assert_approx(8.0, att_distance(&city(0, 0.0, 0.0), &city(1, 10.0, 20.0)));
        assert_approx(5.0, att_distance(&city(0, 0.0, 0.0), &city(1, 10.0, 10.0)));
        // Exactly-integral r must not be nudged up by rounding: dx=10, dy=30 -> r = 10.
        assert_approx(10.0, att_distance(&city(0, 0.0, 0.0), &city(1, 10.0, 30.0)));

        // ATT is not Euclidean: the integer rounding distinguishes it.
        let p = city(0, 0.0, 0.0);
        let q = city(1, 3.0, 4.0);
        assert_approx(5.0, p.distance(&q));
        assert_approx(2.0, att_distance(&p, &q)); // sqrt(25/10) = 1.5811 -> 2
    }

    #[test]
    fn ceil_2d_distance_rounds_up() {
        let city = |id: usize, x: f32, y: f32| kdtree::KDPoint::new_with_id(id, &[x, y]);
        assert_approx(
            5.0,
            ceil_2d_distance(&city(0, 0.0, 0.0), &city(1, 3.0, 4.0)),
        );
        assert_approx(
            2.0,
            ceil_2d_distance(&city(0, 0.0, 0.0), &city(1, 1.0, 1.0)),
        ); // 1.4142
        assert_approx(
            4.0,
            ceil_2d_distance(&city(0, 0.0, 0.0), &city(1, 3.0, 2.0)),
        ); // 3.6056
    }

    #[test]
    fn att_tour_length_matches_the_known_optimum() {
        // External anchor: att48's published ATT optimum is 10628, and its tracked optimal
        // tour must measure exactly that. (33523.71, often quoted for att48, is the
        // *Euclidean* length of the same tour - a different metric.)
        use crate::tsp::{DistanceType, opt_tour, tsplib};
        let data = tsplib::read_from_file(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/att48.tsp"
        )))
        .expect("att48 fixture must parse");
        assert_eq!(
            data.distance_type,
            DistanceType::Att,
            "att48 no longer declares ATT"
        );
        let cities = data.cities().to_vec();
        let dm = DistanceMatrix::build(&cities, data.distance_type).expect("matrix builds");

        let tour = opt_tour::read_from_file(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/att48.opt.tour"
        )))
        .expect("att48.opt.tour must parse; without it this test asserts nothing");
        assert_approx(10628.0, dm.tour_length(&tour.route));
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

        assert_eq!(res.distances().as_ref(), &[1.0]);
    }

    /// The memory model the benchmark and the 100k claim rest on: a coordinate matrix stores
    /// four bytes per city pair. A wider element (or a padded layout) must fail here rather
    /// than silently doubling the footprint.
    #[test]
    fn coordinate_matrix_stores_at_most_four_bytes_per_pair() {
        let n = 64;
        let coords: Vec<Vec<f32>> = (0..n)
            .map(|i| vec![(i % 8) as f32, (i / 8) as f32])
            .collect();
        let cities = kdtree::build_points(&coords);
        let dm = DistanceMatrix::build(&cities, DistanceType::Euc2D).unwrap();

        let pairs = n * (n - 1) / 2;
        let bytes = dm.stored_bytes();
        assert!(
            bytes <= 4 * pairs,
            "coordinate matrix stores {bytes} bytes for {pairs} pairs, exceeding 4 bytes/pair"
        );
        assert_eq!(dm.storage_kind(), "f32");
    }

    fn explicit_cities(n: usize) -> Vec<KDPoint> {
        (0..n)
            .map(|i| KDPoint::new_with_id(i + 1, &[i as f32, 0.0]))
            .collect()
    }

    #[test]
    fn explicit_integers_use_u16_and_wider_values_stay_f32() {
        let cities = explicit_cities(3);
        let packed = vec![0.0, 65_535.0, 1.0];

        let narrow = DistanceMatrix::new(3, packed.clone(), cities.clone()).unwrap();
        assert_eq!(narrow.storage_kind(), "u16");
        assert_eq!(narrow.stored_bytes(), packed.len() * 2);
        assert_eq!(narrow.distance_by_pos(2, 0).unwrap(), 65_535.0);
        assert_eq!(narrow.distance_by_pos(2, 1).unwrap(), 1.0);

        for (packed, expected_kind) in [
            (vec![1.0, 65_536.0, 2.0], "f32"), // does not fit u16
            (vec![1.0, 1.5, 2.0], "f32"),
            (vec![1.0, -1.0, 2.0], "f32"),
            (vec![1.0, -0.0, 2.0], "f32"), // keeps the sign bit
            (vec![1.0, f32::NAN, 2.0], "f32"),
            (vec![1.0, f32::INFINITY, 2.0], "f32"),
            (vec![1.0, f32::NEG_INFINITY, 2.0], "f32"),
        ] {
            let matrix = DistanceMatrix::new(3, packed.clone(), cities.clone()).unwrap();
            assert_eq!(
                matrix.storage_kind(),
                expected_kind,
                "packed values {packed:?}"
            );
            for (got, want) in matrix.distances().iter().zip(&packed) {
                assert_eq!(got.to_bits(), want.to_bits(), "packed values {packed:?}");
            }
        }
    }

    #[test]
    fn from_explicit_slice_packs_without_an_owned_vec() {
        let cities = explicit_cities(3);

        let narrow = [0.0, 65_535.0, 1.0];
        let matrix = DistanceMatrix::from_explicit_slice(3, &narrow, cities.clone()).unwrap();
        assert_eq!(matrix.storage_kind(), "u16");
        assert_eq!(matrix.distances().as_ref(), narrow.as_slice());

        let wide = [0.0, 65_536.0, 1.0];
        let matrix = DistanceMatrix::from_explicit_slice(3, &wide, cities).unwrap();
        assert_eq!(matrix.storage_kind(), "f32");
        assert_eq!(matrix.distances().as_ref(), wide.as_slice());
    }

    /// Compact and `f32` storage must produce bit-identical tours: compaction is a storage
    /// change, never a numeric one.
    #[test]
    fn compact_storage_matches_f32_storage_for_every_pair() {
        let n = 16;
        let values: Vec<f32> = (0..n * (n - 1) / 2).map(|i| (i % 60_000) as f32).collect();
        let cities = explicit_cities(n);

        let compact = DistanceMatrix::new(n, values.clone(), cities.clone()).unwrap();
        assert_eq!(compact.storage_kind(), "u16");
        let reference =
            DistanceMatrix::from_parts(n, PackedDistances::F32(values.clone()), cities, false)
                .unwrap();

        for a in 0..n {
            for b in 0..n {
                assert_eq!(
                    compact.distance_by_pos(a, b).unwrap(),
                    reference.distance_by_pos(a, b).unwrap(),
                    "pair ({a}, {b})"
                );
            }
        }

        let route: Vec<usize> = (1..=n).collect();
        assert_eq!(
            compact.tour_length(&route),
            reference.tour_length(&route),
            "compaction changed the tour cost"
        );
    }

    /// TSPLIB's EUC_2D is `nint(sqrt(dx^2 + dy^2))`. The rounding *is* the metric: unrounded,
    /// teeline optimises a different objective than `EDGE_WEIGHT_TYPE` names and cannot reproduce
    /// any published optimum. A regression to the raw square root has to fail here.
    #[test]
    fn euc_2d_rounds_to_the_nearest_integer() {
        let origin = KDPoint::new(&[0.0, 0.0]);
        for (target, expected) in [
            ([1.0, 0.0], 1.0),
            ([3.0, 4.0], 5.0), // exact
            ([2.0, 1.0], 2.0), // sqrt(5) = 2.236 -> 2
            ([1.0, 1.0], 1.0), // sqrt(2) = 1.414 -> 1
            ([0.0, 0.5], 1.0), // exactly .5 rounds up, as nint does
            ([0.0, 0.4], 0.0), // below .5 rounds down
        ] {
            assert_approx(expected, euc_2d_distance(&origin, &KDPoint::new(&target)));
        }
    }

    /// Both builders must apply that same rounding, or a solver's reported cost would not be the
    /// objective it optimised.
    #[test]
    fn the_matrix_builders_round_euc_2d() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![2.0, 1.0], vec![3.0, 4.0]]);
        for dm in [
            DistanceMatrix::from_cities(&cities).unwrap(),
            DistanceMatrix::build(&cities, DistanceType::Euc2D).unwrap(),
        ] {
            assert_approx(2.0, dm.distance_between(0, 1).unwrap()); // sqrt(5)  = 2.236
            assert_approx(3.0, dm.distance_between(1, 2).unwrap()); // sqrt(10) = 3.162
            assert_approx(5.0, dm.distance_between(0, 2).unwrap()); // exact
        }
    }

    #[test]
    fn test_build_distance_matrix_from_3_item_list() {
        let cities = kdtree::build_points(&[vec![0.0, 0.0], vec![0.0, 1.0], vec![2.0, 0.0]]);

        let res = DistanceMatrix::from_cities_unrounded(&cities);

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
        let res = DistanceMatrix::from_cities_unrounded(&cities);

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
        let res = DistanceMatrix::from_cities_unrounded(&cities);

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
        let dm = DistanceMatrix::from_cities_unrounded(&cities).unwrap();

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

        let dm = from_cities_unrounded(&cities);

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

    #[test]
    fn sparse_ids_resolve_through_the_sparse_index() {
        // max_id (30) exceeds 4 * n (12), so `IdIndex` must take the sparse branch; the
        // matrix still has to answer every position/id query exactly.
        let cities = vec![
            KDPoint::new_with_id(10, &[0.0, 0.0]),
            KDPoint::new_with_id(20, &[3.0, 4.0]),
            KDPoint::new_with_id(30, &[0.0, 1.0]),
        ];
        let dm = DistanceMatrix::from_cities_unrounded(&cities).unwrap();
        assert!(
            matches!(dm.city_idx, IdIndex::Sparse(_)),
            "the test must exercise the sparse branch, not the dense fallback"
        );

        assert_eq!(dm.point_at(0).map(|c| c.id), Some(10));
        assert_eq!(dm.pos2city_id(1), Some(20));
        assert_eq!(dm.pos2city_id(3), None);
        assert_eq!(dm.city_id2pos(30), Some(2));
        assert_eq!(dm.city_id2pos(99), None);
        assert_approx(5.0, dm.distance_between(10, 20).unwrap());
        assert!(dm.distance_between(10, 99).is_err());

        let d20_30 = 18.0_f32.sqrt();
        assert_approx(1.0 + 5.0 + d20_30, dm.tour_length(&[10, 20, 30]));
    }

    #[test]
    fn duplicate_ids_are_rejected_not_panicked() {
        let cities = vec![
            KDPoint::new_with_id(1, &[0.0, 0.0]),
            KDPoint::new_with_id(1, &[1.0, 0.0]),
            KDPoint::new_with_id(2, &[0.0, 1.0]),
        ];
        let result = DistanceMatrix::build(&cities, DistanceType::Euc2D);
        assert!(result.is_err(), "duplicate IDs must return an error");
    }
}

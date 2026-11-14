use std::sync::mpsc;

use super::progress::ProgressMessage;
use super::route::Route;
use super::{KDPoint, Solution, TspProblem};

/// Below this span the grid collapses; identical or collinear cities must not divide by zero.
const EPSILON: f64 = 1e-12;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SfcCurveType {
    #[default]
    Hilbert,
    Morton,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AspectScaling {
    #[default]
    Uniform,
    StretchToSquare,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TieBreaker {
    #[default]
    InputOrder,
    /// Order same-cell cities by their original coordinates.
    Lexicographic,
    /// Greedily pick the bucket city nearest the last visited one. Buckets larger than
    /// `MAX_NEAREST_BUCKET` fall back to input order so clustered instances stay O(n log n).
    NearestPrev,
}

/// Above this bucket size the greedy `nearest_prev` scan (O(k²)) would dominate;
/// oversized buckets keep input order instead.
const MAX_NEAREST_BUCKET: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct SfcOptions {
    pub curve_type: SfcCurveType,
    /// Grid order `p`: the grid is `2^p × 2^p`.
    pub order: u8,
    pub aspect_scaling: AspectScaling,
    pub rotation: u16,
    pub mirror: bool,
    /// 0 disables the ensemble; `N in 1..=8` evaluates at most `N` orientations.
    pub multi_orientation: u8,
    pub tie_breaker: TieBreaker,
}

impl Default for SfcOptions {
    fn default() -> Self {
        SfcOptions {
            curve_type: SfcCurveType::Hilbert,
            order: 16,
            aspect_scaling: AspectScaling::Uniform,
            rotation: 0,
            mirror: false,
            multi_orientation: 0,
            tie_breaker: TieBreaker::InputOrder,
        }
    }
}

impl SfcOptions {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=16).contains(&self.order) {
            return Err(format!("order must be in 1..=16, got {}", self.order));
        }
        if ![0, 90, 180, 270].contains(&self.rotation) {
            return Err(format!(
                "rotation must be 0, 90, 180 or 270, got {}",
                self.rotation
            ));
        }
        if self.multi_orientation > 8 {
            return Err(format!(
                "multi_orientation must be in 0..=8, got {}",
                self.multi_orientation
            ));
        }
        Ok(())
    }

    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut opts = SfcOptions::default();
        for (key, value) in table.iter() {
            match key.as_str() {
                "curve_type" => {
                    let s = value.as_str().ok_or_else(|| {
                        format!("config: `curve_type` must be a string, got {value}")
                    })?;
                    opts.curve_type = SfcCurveType::parse(s)?;
                }
                "order" => {
                    let n = value.as_integer().ok_or_else(|| {
                        format!("config: `order` must be an integer, got {value}")
                    })?;
                    opts.order = u8::try_from(n)
                        .map_err(|_| format!("config: `order` must be in 1..=16, got {n}"))?;
                }
                "aspect_scaling" => {
                    let s = value.as_str().ok_or_else(|| {
                        format!("config: `aspect_scaling` must be a string, got {value}")
                    })?;
                    opts.aspect_scaling = AspectScaling::parse(s)?;
                }
                "rotation" => {
                    let n = value.as_integer().ok_or_else(|| {
                        format!("config: `rotation` must be an integer, got {value}")
                    })?;
                    opts.rotation = u16::try_from(n).map_err(|_| {
                        format!("config: `rotation` must be 0, 90, 180 or 270, got {n}")
                    })?;
                }
                "mirror" => {
                    opts.mirror = value.as_bool().ok_or_else(|| {
                        format!("config: `mirror` must be a boolean, got {value}")
                    })?;
                }
                "multi_orientation" => {
                    if let Some(enabled) = value.as_bool() {
                        opts.multi_orientation = if enabled { 8 } else { 0 };
                    } else if let Some(n) = value.as_integer() {
                        opts.multi_orientation = u8::try_from(n).map_err(|_| {
                            format!("config: `multi_orientation` must be in 0..=8, got {n}")
                        })?;
                    } else {
                        return Err(format!(
                            "config: `multi_orientation` must be a boolean or an integer in \
                             0..=8, got {value}"
                        ));
                    }
                }
                "tie_breaker" => {
                    let s = value.as_str().ok_or_else(|| {
                        format!("config: `tie_breaker` must be a string, got {value}")
                    })?;
                    opts.tie_breaker = TieBreaker::parse(s)?;
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` in [sfc] — valid: curve_type, order, \
                         aspect_scaling, rotation, mirror, multi_orientation, tie_breaker"
                    ));
                }
            }
        }
        opts.validate()?;
        Ok(opts)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        let mut opts = SfcOptions::default();
        if let Some(v) = args.get_one::<String>("curve_type") {
            opts.curve_type = SfcCurveType::parse(v)?;
        }
        if let Some(v) = args.get_one::<u8>("sfc_order") {
            opts.order = *v;
        }
        if let Some(v) = args.get_one::<String>("aspect_scaling") {
            opts.aspect_scaling = AspectScaling::parse(v)?;
        }
        if let Some(v) = args.get_one::<String>("sfc_rotation") {
            opts.rotation = v
                .parse::<u16>()
                .map_err(|_| format!("--sfc-rotation: invalid integer `{v}`"))?;
        }
        if args.get_flag("sfc_mirror") {
            opts.mirror = true;
        }
        if let Some(v) = args.get_one::<String>("multi_orientation") {
            opts.multi_orientation = match v.as_str() {
                "true" => 8,
                "false" => 0,
                _ => v
                    .parse::<u8>()
                    .map_err(|_| format!("--multi-orientation: invalid integer `{v}`"))?,
            };
        }
        if let Some(v) = args.get_one::<String>("tie_breaker") {
            opts.tie_breaker = TieBreaker::parse(v)?;
        }
        opts.validate()?;
        Ok(opts)
    }
}

impl SfcCurveType {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "hilbert" => Ok(Self::Hilbert),
            "morton" => Ok(Self::Morton),
            other => Err(format!(
                "invalid curve_type `{other}` — expected hilbert or morton"
            )),
        }
    }
}

impl AspectScaling {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "uniform" => Ok(Self::Uniform),
            "stretch_to_square" => Ok(Self::StretchToSquare),
            other => Err(format!(
                "invalid aspect_scaling `{other}` — expected uniform or stretch_to_square"
            )),
        }
    }
}

impl TieBreaker {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "input_order" => Ok(Self::InputOrder),
            "lexicographic" => Ok(Self::Lexicographic),
            "nearest_prev" => Ok(Self::NearestPrev),
            other => Err(format!(
                "invalid tie_breaker `{other}` — expected input_order, lexicographic or \
                 nearest_prev"
            )),
        }
    }
}

/// Map `(x, y)` on a `2^p` grid to its Hilbert index.
///
/// Standard Lam–Shapiro `xy2d`; the reflection uses the full grid size `n - 1 - x`
/// (not the current quadrant size). Consecutive indices are grid neighbours.
pub(crate) fn hilbert_xy_to_d(p: u8, x: u32, y: u32) -> u64 {
    let n = 1u64 << p;
    let mut x = u64::from(x);
    let mut y = u64::from(y);
    let mut d = 0u64;
    let mut s = n / 2;
    while s > 0 {
        let rx = u64::from((x & s) > 0);
        let ry = u64::from((y & s) > 0);
        d += s * s * ((3 * rx) ^ ry);
        if ry == 0 {
            if rx == 1 {
                x = n - 1 - x;
                y = n - 1 - y;
            }
            std::mem::swap(&mut x, &mut y);
        }
        s /= 2;
    }
    d
}

/// Interleave the bits of `x` and `y` (Morton/Z-order); distinct cells get distinct keys.
pub(crate) fn morton_xy_to_d(x: u32, y: u32) -> u64 {
    fn spread(v: u32) -> u64 {
        let mut v = u64::from(v);
        v = (v | (v << 16)) & 0x0000_FFFF_0000_FFFF;
        v = (v | (v << 8)) & 0x00FF_00FF_00FF_00FF;
        v = (v | (v << 4)) & 0x0F0F_0F0F_0F0F_0F0F;
        v = (v | (v << 2)) & 0x3333_3333_3333_3333;
        v = (v | (v << 1)) & 0x5555_5555_5555_5555;
        v
    }
    (spread(y) << 1) | spread(x)
}

/// Project city coordinates onto the `2^p × 2^p` integer grid.
fn normalize_coords(cities: &[KDPoint], p: u8, mode: AspectScaling) -> Vec<(u32, u32)> {
    let max = (1u64 << p) as f64 - 1.0;
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for c in cities {
        let x = f64::from(c.coords[0]);
        let y = f64::from(c.coords[1]);
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    let span_x = (max_x - min_x).max(EPSILON);
    let span_y = (max_y - min_y).max(EPSILON);
    let (sx, sy) = match mode {
        AspectScaling::Uniform => {
            let span = span_x.max(span_y);
            (span, span)
        }
        AspectScaling::StretchToSquare => (span_x, span_y),
    };

    cities
        .iter()
        .map(|c| {
            let gx = (((f64::from(c.coords[0]) - min_x) / sx) * max)
                .round()
                .clamp(0.0, max) as u32;
            let gy = (((f64::from(c.coords[1]) - min_y) / sy) * max)
                .round()
                .clamp(0.0, max) as u32;
            (gx, gy)
        })
        .collect()
}

/// Rotate/mirror a grid cell. Mirroring is applied first, then rotation.
fn transform_grid(gx: u32, gy: u32, max: u32, rotation: u16, mirror: bool) -> (u32, u32) {
    let (mut x, mut y) = (gx, gy);
    if mirror {
        x = max - x;
    }
    match rotation {
        0 => {}
        90 => {
            let (nx, ny) = (y, max - x);
            x = nx;
            y = ny;
        }
        180 => {
            x = max - x;
            y = max - y;
        }
        270 => {
            let (nx, ny) = (max - y, x);
            x = nx;
            y = ny;
        }
        _ => unreachable!("rotation is validated in SfcOptions::validate"),
    }
    (x, y)
}

const ALL_ORIENTATIONS: [(u16, bool); 8] = [
    (0, false),
    (0, true),
    (90, false),
    (90, true),
    (180, false),
    (180, true),
    (270, false),
    (270, true),
];

/// The configured orientation first, then enough others to reach `multi_orientation`
/// (capped at the eight possible transforms). `0` and `1` both mean "configured only".
fn orientation_set(opts: &SfcOptions) -> Vec<(u16, bool)> {
    let configured = (opts.rotation, opts.mirror);
    let count = if opts.multi_orientation == 0 {
        1
    } else {
        usize::from(opts.multi_orientation).min(ALL_ORIENTATIONS.len())
    };
    let mut set = Vec::with_capacity(count);
    set.push(configured);
    for &orientation in &ALL_ORIENTATIONS {
        if set.len() >= count {
            break;
        }
        if orientation != configured {
            set.push(orientation);
        }
    }
    set
}

/// Order city positions by curve key, resolving same-cell ties with `tie_breaker`.
fn order_cities(cities: &[KDPoint], keys: &[u64], tie_breaker: TieBreaker) -> Vec<usize> {
    match tie_breaker {
        TieBreaker::InputOrder => {
            let mut order: Vec<usize> = (0..cities.len()).collect();
            order.sort_unstable_by_key(|&i| (keys[i], i));
            order
        }
        TieBreaker::Lexicographic => {
            // Curve keys are a bijection over cells, so equal keys mean the same cell;
            // ordering by the original coordinates is what makes this distinct from
            // `input_order` for cities that land in the same (coarse) cell.
            let mut order: Vec<usize> = (0..cities.len()).collect();
            order.sort_unstable_by(|&a, &b| {
                keys[a]
                    .cmp(&keys[b])
                    .then_with(|| cities[a].coords[0].total_cmp(&cities[b].coords[0]))
                    .then_with(|| cities[a].coords[1].total_cmp(&cities[b].coords[1]))
                    .then_with(|| a.cmp(&b))
            });
            order
        }
        TieBreaker::NearestPrev => {
            use std::collections::BTreeMap;

            let mut buckets: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
            for (i, &key) in keys.iter().enumerate() {
                buckets.entry(key).or_default().push(i);
            }

            let mut order = Vec::with_capacity(cities.len());
            let mut last: Option<usize> = None;
            for (_, mut bucket) in buckets {
                if bucket.len() > MAX_NEAREST_BUCKET {
                    // Degenerate/clustered input: the greedy scan would be O(k²) and can
                    // dominate the sort. The bucket already preserves input order.
                    order.extend(bucket);
                    last = order.last().copied();
                    continue;
                }
                while !bucket.is_empty() {
                    let pick = match last {
                        None => 0,
                        Some(prev) => {
                            let mut best = 0;
                            let mut best_distance = f64::INFINITY;
                            for (j, &idx) in bucket.iter().enumerate() {
                                let dx = f64::from(cities[idx].coords[0] - cities[prev].coords[0]);
                                let dy = f64::from(cities[idx].coords[1] - cities[prev].coords[1]);
                                let distance = dx * dx + dy * dy;
                                if distance < best_distance {
                                    best_distance = distance;
                                    best = j;
                                }
                            }
                            best
                        }
                    };
                    // `remove` preserves input order, keeping ties deterministic.
                    let idx = bucket.remove(pick);
                    order.push(idx);
                    last = Some(idx);
                }
            }
            order
        }
    }
}

fn curve_index(curve_type: SfcCurveType, order: u8, gx: u32, gy: u32) -> u64 {
    match curve_type {
        SfcCurveType::Hilbert => hilbert_xy_to_d(order, gx, gy),
        SfcCurveType::Morton => morton_xy_to_d(gx, gy),
    }
}

/// Clamp invalid values rather than panicking: `solve` is public and `SfcOptions`
/// fields are public, but only `solve_with_context` validates strictly.
fn sanitized_options(opts: &SfcOptions) -> SfcOptions {
    let mut opts = opts.clone();
    if !(1..=16).contains(&opts.order) {
        tracing::warn!(order = opts.order, "invalid SFC grid order; using 16");
        opts.order = 16;
    }
    if ![0, 90, 180, 270].contains(&opts.rotation) {
        tracing::warn!(rotation = opts.rotation, "invalid SFC rotation; using 0");
        opts.rotation = 0;
    }
    if opts.multi_orientation > 8 {
        tracing::warn!(
            multi_orientation = opts.multi_orientation,
            "invalid SFC multi-orientation; using 8"
        );
        opts.multi_orientation = 8;
    }
    opts
}

/// Space-filling-curve construction: project cities onto a `2^p × 2^p` grid, sort them
/// by their Hilbert/Morton index, and emit that order as the tour.
///
/// Deterministic and `O(n log n)` for `input_order` and after `nearest_prev`'s oversized
/// bucket guard. It ignores `init_tour` like the other constructive solvers, which makes
/// it a cheap warm-start producer for `two_opt`, `or_opt`, and `lin_kernighan`.
///
/// Invalid option values are clamped defensively; `solve_with_context` performs strict
/// validation and returns an error before calling this function.
pub fn solve(
    problem: &TspProblem,
    opts: &SfcOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let n = cities.len();
    let opts = sanitized_options(opts);

    tracing::info!(
        cities = n,
        curve = ?opts.curve_type,
        order = opts.order,
        "space_filling_curve starting"
    );

    if n <= 2 {
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::Done);
        }
        return Solution::from_parts(&path, cities, &problem.distances);
    }

    let base_grid = normalize_coords(cities, opts.order, opts.aspect_scaling);
    let max = (1u32 << opts.order) - 1;

    // No meaningful intermediate state to stream; mirror greedy_edge's placeholder so the
    // visualisation window does not look frozen during the sort.
    if let Some(tx) = progress_tx {
        let identity: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&identity), 0.0));
    }

    let mut best: Option<(Vec<usize>, f32)> = None;
    for (rotation, mirror) in orientation_set(&opts) {
        let grid: Vec<(u32, u32)> = base_grid
            .iter()
            .map(|&(x, y)| transform_grid(x, y, max, rotation, mirror))
            .collect();
        let keys: Vec<u64> = grid
            .iter()
            .map(|&(x, y)| curve_index(opts.curve_type, opts.order, x, y))
            .collect();
        let tour: Vec<usize> = order_cities(cities, &keys, opts.tie_breaker)
            .into_iter()
            .map(|i| cities[i].id)
            .collect();
        let cost = problem.distances.tour_length(&tour);
        if best.as_ref().is_none_or(|(_, best_cost)| cost < *best_cost) {
            best = Some((tour, cost));
        }
    }

    let (path, total) = best.expect("orientation set is never empty");
    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&path), total));
        let _ = tx.send(ProgressMessage::Done);
    }

    Solution::from_parts(&path, cities, &problem.distances)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn city(id: usize, x: f32, y: f32) -> KDPoint {
        KDPoint { id, coords: [x, y] }
    }

    #[test]
    fn hilbert_reference_p1() {
        assert_eq!(hilbert_xy_to_d(1, 0, 0), 0);
        assert_eq!(hilbert_xy_to_d(1, 0, 1), 1);
        assert_eq!(hilbert_xy_to_d(1, 1, 1), 2);
        assert_eq!(hilbert_xy_to_d(1, 1, 0), 3);
    }

    #[test]
    fn hilbert_reference_p2() {
        let expected = [[0, 1, 14, 15], [3, 2, 13, 12], [4, 7, 8, 11], [5, 6, 9, 10]];
        for (y, row) in expected.iter().enumerate() {
            for (x, &d) in row.iter().enumerate() {
                assert_eq!(hilbert_xy_to_d(2, x as u32, y as u32), d);
            }
        }
    }

    #[test]
    fn hilbert_is_bijection_and_adjacent_for_p1_to_p6() {
        for p in 1..=6 {
            let n = 1u32 << p;
            let total = (n * n) as usize;
            let mut cells = vec![(0u32, 0u32); total];
            let mut seen = vec![false; total];
            for y in 0..n {
                for x in 0..n {
                    let d = hilbert_xy_to_d(p, x, y) as usize;
                    assert!(d < total, "p={p}: index {d} out of range");
                    assert!(!seen[d], "p={p}: duplicate index {d} at ({x},{y})");
                    seen[d] = true;
                    cells[d] = (x, y);
                }
            }
            for w in cells.windows(2) {
                let (x0, y0) = w[0];
                let (x1, y1) = w[1];
                assert_eq!(
                    x0.abs_diff(x1) + y0.abs_diff(y1),
                    1,
                    "p={p}: {w:?} are not adjacent"
                );
            }
        }
    }

    #[test]
    fn morton_is_bijection_for_p1_to_p6() {
        for p in 1..=6 {
            let n = 1u32 << p;
            let mut seen = std::collections::HashSet::new();
            for y in 0..n {
                for x in 0..n {
                    assert!(
                        seen.insert(morton_xy_to_d(x, y)),
                        "p={p}: duplicate Morton key at ({x},{y})"
                    );
                }
            }
            assert_eq!(seen.len(), (n * n) as usize);
        }
    }

    #[test]
    fn morton_reference_p1() {
        assert_eq!(morton_xy_to_d(0, 0), 0);
        assert_eq!(morton_xy_to_d(1, 0), 1);
        assert_eq!(morton_xy_to_d(0, 1), 2);
        assert_eq!(morton_xy_to_d(1, 1), 3);
    }

    #[test]
    fn hilbert_p16_boundary_does_not_overflow() {
        let max = (1u32 << 16) - 1;
        assert!(hilbert_xy_to_d(16, max, max) < (1u64 << 32));
    }

    #[test]
    fn normalization_clamps_max_edge() {
        let cities = vec![city(0, 0.0, 0.0), city(1, 10.0, 10.0)];
        let grid = normalize_coords(&cities, 4, AspectScaling::Uniform);
        let max = (1u32 << 4) - 1;
        assert_eq!(grid[0], (0, 0));
        assert_eq!(grid[1], (max, max));
    }

    #[test]
    fn normalization_handles_identical_and_collinear() {
        let identical = vec![city(0, 5.0, 5.0), city(1, 5.0, 5.0)];
        assert_eq!(
            normalize_coords(&identical, 8, AspectScaling::Uniform),
            vec![(0, 0), (0, 0)]
        );

        let collinear = vec![city(0, 0.0, 0.0), city(1, 1.0, 0.0), city(2, 2.0, 0.0)];
        let grid = normalize_coords(&collinear, 4, AspectScaling::StretchToSquare);
        assert_eq!(grid[0], (0, 0));
        assert_eq!(grid[2], (15, 0));
    }

    #[test]
    fn normalization_uniform_preserves_aspect() {
        let cities = vec![city(0, 0.0, 0.0), city(1, 2.0, 1.0)];
        let uniform = normalize_coords(&cities, 4, AspectScaling::Uniform);
        let max = (1u32 << 4) - 1;
        // The short axis maps to roughly half the grid (7.5 rounds to 8), not to `max`.
        assert_eq!(uniform[1], (max, 8));

        let stretch = normalize_coords(&cities, 4, AspectScaling::StretchToSquare);
        assert_eq!(stretch[1], (max, max));
    }

    #[test]
    fn order_validation_rejects_out_of_range() {
        let mut opts = SfcOptions::default();
        assert!(opts.validate().is_ok());
        opts.order = 0;
        assert!(opts.validate().is_err());
        opts.order = 17;
        assert!(opts.validate().is_err());
        opts.order = 16;
        assert!(opts.validate().is_ok());
    }

    #[test]
    fn rotation_and_multi_orientation_validation() {
        let mut opts = SfcOptions::default();
        opts.rotation = 45;
        assert!(opts.validate().is_err());
        opts.rotation = 270;
        opts.multi_orientation = 9;
        assert!(opts.validate().is_err());
        opts.multi_orientation = 8;
        assert!(opts.validate().is_ok());
    }

    // ------------------------------------------------------------------
    // Tour assembly
    // ------------------------------------------------------------------

    use crate::tsp::{distance_matrix, kdtree, progress::ProgressMessage};
    use std::sync::mpsc;

    fn make_problem(coords: &[[f32; 2]]) -> TspProblem {
        let cities =
            kdtree::build_points(&coords.iter().map(|c| vec![c[0], c[1]]).collect::<Vec<_>>());
        let distances = distance_matrix::from_cities(&cities);
        TspProblem::new(cities, distances)
    }

    fn assert_valid_tour(problem: &TspProblem, solution: &Solution) {
        let n = problem.cities.len();
        let route = solution.route();
        assert_eq!(route.len(), n);

        let mut seen = route.to_vec();
        seen.sort_unstable();
        let mut expected: Vec<usize> = problem.cities.iter().map(|c| c.id).collect();
        expected.sort_unstable();
        assert_eq!(seen, expected);

        let recomputed: f32 = (0..n)
            .map(|i| {
                problem
                    .distances
                    .distance_between(route[i], route[(i + 1) % n])
                    .unwrap_or(f32::MAX)
            })
            .sum();
        assert!(
            (solution.total - recomputed).abs() < 1e-3,
            "reported total {} != recomputed {recomputed}",
            solution.total
        );
    }

    #[test]
    fn input_order_and_lexicographic_tie_breakers() {
        // Same cell (same curve key) but different original coordinates: this is the only
        // state a real solve can produce for a tie, and where the two modes must differ.
        let cities = vec![city(10, 10.0, 10.0), city(11, 0.0, 0.0)];
        let keys = vec![5, 5];
        assert_eq!(
            order_cities(&cities, &keys, TieBreaker::InputOrder),
            vec![0, 1]
        );
        assert_eq!(
            order_cities(&cities, &keys, TieBreaker::Lexicographic),
            vec![1, 0]
        );
    }

    #[test]
    fn nearest_prev_picks_the_closest_bucket_city() {
        let cities = vec![city(0, 0.0, 0.0), city(1, 10.0, 0.0), city(2, 1.0, 0.0)];
        let keys = vec![0, 0, 0];
        let order = order_cities(&cities, &keys, TieBreaker::NearestPrev);
        assert_eq!(order, vec![0, 2, 1]);
        assert_eq!(
            order_cities(&cities, &keys, TieBreaker::NearestPrev),
            order,
            "nearest_prev must be deterministic"
        );
    }

    #[test]
    fn nearest_prev_falls_back_to_input_order_for_oversized_buckets() {
        let n = MAX_NEAREST_BUCKET + 1;
        let cities: Vec<KDPoint> = (0..n).map(|i| city(i, i as f32, 0.0)).collect();
        let keys = vec![7u64; n];
        assert_eq!(
            order_cities(&cities, &keys, TieBreaker::NearestPrev),
            (0..n).collect::<Vec<_>>()
        );
    }

    #[test]
    fn solve_sanitizes_invalid_options_instead_of_panicking() {
        let problem = make_problem(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        let invalid = SfcOptions {
            order: 32,
            rotation: 45,
            multi_orientation: 99,
            ..SfcOptions::default()
        };
        assert_valid_tour(&problem, &solve(&problem, &invalid, None, None));
    }

    #[test]
    fn solve_ignores_the_init_tour() {
        let problem = make_problem(&[[0.0, 0.0], [8.0, 1.0], [2.0, 6.0], [5.0, 3.0], [9.0, 9.0]]);
        let seed: Vec<usize> = problem.cities.iter().rev().map(|c| c.id).collect();
        let with_seed = solve(&problem, &SfcOptions::default(), None, Some(&seed));
        let without_seed = solve(&problem, &SfcOptions::default(), None, None);
        assert_eq!(with_seed.route(), without_seed.route());
    }

    #[test]
    fn transforms_are_bijections_over_the_grid() {
        let max = 15u32;
        for rotation in [0, 90, 180, 270] {
            for mirror in [false, true] {
                let mut seen = std::collections::HashSet::new();
                for y in 0..=max {
                    for x in 0..=max {
                        let cell = transform_grid(x, y, max, rotation, mirror);
                        assert!(cell.0 <= max && cell.1 <= max);
                        assert!(seen.insert(cell), "duplicate transform result {cell:?}");
                    }
                }
                assert_eq!(seen.len(), 16 * 16);
            }
        }
    }

    #[test]
    fn rotation_and_mirror_move_the_expected_cell() {
        assert_eq!(transform_grid(1, 0, 15, 0, false), (1, 0));
        assert_eq!(transform_grid(1, 0, 15, 0, true), (14, 0));
        assert_eq!(transform_grid(1, 0, 15, 90, false), (0, 14));
        assert_eq!(transform_grid(1, 0, 15, 180, false), (14, 15));
        assert_eq!(transform_grid(1, 0, 15, 270, false), (15, 1));
    }

    #[test]
    fn multi_orientation_never_worse_than_any_single_orientation() {
        let coords: Vec<[f32; 2]> = (0..5)
            .flat_map(|y| (0..5).map(move |x| [x as f32 * 3.0, y as f32 * 3.0]))
            .collect();
        let problem = make_problem(&coords);

        let ensemble = solve(
            &problem,
            &SfcOptions {
                multi_orientation: 8,
                ..Default::default()
            },
            None,
            None,
        );

        for rotation in [0, 90, 180, 270] {
            for mirror in [false, true] {
                let single = solve(
                    &problem,
                    &SfcOptions {
                        rotation,
                        mirror,
                        ..Default::default()
                    },
                    None,
                    None,
                );
                assert!(
                    ensemble.total <= single.total + 1e-3,
                    "ensemble {:.2} worse than single rotation={rotation} mirror={mirror} {:.2}",
                    ensemble.total,
                    single.total
                );
            }
        }

        let again = solve(
            &problem,
            &SfcOptions {
                multi_orientation: 8,
                ..Default::default()
            },
            None,
            None,
        );
        assert_eq!(ensemble.route(), again.route());
    }

    #[test]
    fn degenerate_inputs_return_valid_tours() {
        let two = make_problem(&[[0.0, 0.0], [1.0, 0.0]]);
        let sol = solve(&two, &SfcOptions::default(), None, None);
        assert_valid_tour(&two, &sol);

        let identical = make_problem(&[[3.0, 3.0]; 5]);
        let sol = solve(&identical, &SfcOptions::default(), None, None);
        assert_valid_tour(&identical, &sol);

        let collinear = make_problem(&[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0], [4.0, 0.0]]);
        let sol = solve(&collinear, &SfcOptions::default(), None, None);
        assert_valid_tour(&collinear, &sol);
    }

    #[test]
    fn solve_emits_placeholder_update_and_done() {
        let problem = make_problem(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        let (tx, rx) = mpsc::channel();
        let _ = solve(&problem, &SfcOptions::default(), Some(&tx), None);
        let messages: Vec<ProgressMessage> = rx.try_iter().collect();
        assert!(matches!(
            messages.first(),
            Some(ProgressMessage::PathUpdate(..))
        ));
        assert!(matches!(messages.last(), Some(ProgressMessage::Done)));
    }

    #[test]
    fn curve_type_and_aspect_modes_produce_valid_tours() {
        let coords = [[0.0, 0.0], [8.0, 1.0], [2.0, 6.0], [5.0, 3.0], [9.0, 9.0]];
        let problem = make_problem(&coords);
        for curve_type in [SfcCurveType::Hilbert, SfcCurveType::Morton] {
            for aspect_scaling in [AspectScaling::Uniform, AspectScaling::StretchToSquare] {
                for tie_breaker in [
                    TieBreaker::InputOrder,
                    TieBreaker::Lexicographic,
                    TieBreaker::NearestPrev,
                ] {
                    let opts = SfcOptions {
                        curve_type,
                        aspect_scaling,
                        tie_breaker,
                        ..Default::default()
                    };
                    assert_valid_tour(&problem, &solve(&problem, &opts, None, None));
                }
            }
        }
    }

    #[test]
    fn options_defaults_match_documented_values() {
        let opts = SfcOptions::default();
        assert_eq!(opts.curve_type, SfcCurveType::Hilbert);
        assert_eq!(opts.order, 16);
        assert_eq!(opts.aspect_scaling, AspectScaling::Uniform);
        assert_eq!(opts.rotation, 0);
        assert!(!opts.mirror);
        assert_eq!(opts.multi_orientation, 0);
        assert_eq!(opts.tie_breaker, TieBreaker::InputOrder);
    }

    #[test]
    fn options_toml_round_trip() {
        let table: toml::Table = toml::from_str(
            r#"
curve_type = "morton"
order = 8
aspect_scaling = "stretch_to_square"
rotation = 90
mirror = true
multi_orientation = 4
tie_breaker = "nearest_prev"
"#,
        )
        .unwrap();
        let opts = SfcOptions::from_toml(&table).unwrap();
        assert_eq!(opts.curve_type, SfcCurveType::Morton);
        assert_eq!(opts.order, 8);
        assert_eq!(opts.aspect_scaling, AspectScaling::StretchToSquare);
        assert_eq!(opts.rotation, 90);
        assert!(opts.mirror);
        assert_eq!(opts.multi_orientation, 4);
        assert_eq!(opts.tie_breaker, TieBreaker::NearestPrev);
    }

    #[test]
    fn options_toml_accepts_bool_multi_orientation_and_rejects_unknown_fields() {
        let table: toml::Table = toml::from_str("multi_orientation = true").unwrap();
        assert_eq!(SfcOptions::from_toml(&table).unwrap().multi_orientation, 8);

        let table: toml::Table = toml::from_str("multi_orientation = false").unwrap();
        assert_eq!(SfcOptions::from_toml(&table).unwrap().multi_orientation, 0);

        let table: toml::Table = toml::from_str("bogus = 1").unwrap();
        let err = SfcOptions::from_toml(&table).unwrap_err();
        assert!(err.contains("unknown field"), "got: {err}");
    }

    #[test]
    fn options_toml_rejects_invalid_values() {
        for (body, needle) in [
            ("order = 0", "order"),
            ("order = 17", "order"),
            ("rotation = 45", "rotation"),
            ("multi_orientation = 9", "multi_orientation"),
            ("curve_type = \"z\"", "curve_type"),
            ("aspect_scaling = \"z\"", "aspect_scaling"),
            ("tie_breaker = \"z\"", "tie_breaker"),
        ] {
            let table: toml::Table = toml::from_str(body).unwrap();
            let err = SfcOptions::from_toml(&table).unwrap_err();
            assert!(err.contains(needle), "{body}: {err}");
        }
    }
}

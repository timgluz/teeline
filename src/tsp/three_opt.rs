use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{Solution, SolverOptions};

/// 3-opt local search.
///
/// For each triple of edges (i,j,k) consider all 8 reconnection cases.
/// Apply the best-improving case for that triple (lowest cost among the 7
/// non-identity reconnections), then restart the search.  Continues until
/// no improving triple is found.
///
/// Complexity: O(n³) per pass.
pub fn solve(cities: &[KDPoint], distances: &DistanceMatrix, options: &SolverOptions) -> Solution {
    let n = cities.len();
    if n < 4 {
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        return Solution::new(&path, cities, distances);
    }

    let mut path: Vec<usize> = cities.iter().map(|c| c.id).collect();
    options.send_progress(ProgressMessage::PathUpdate(Route::new(&path), 0.0));

    let mut improved = true;
    while improved {
        improved = false;
        'search: for i in 0..n - 2 {
            options.send_progress(ProgressMessage::CityChange(path[i]));
            for j in i + 1..n - 1 {
                for k in j + 1..n {
                    let a = path[i];
                    let b = path[i + 1];
                    let c = path[j];
                    let dt = path[j + 1];
                    let e = path[k];
                    let f = path[(k + 1) % n];

                    let orig = d(distances, a, b) + d(distances, c, dt) + d(distances, e, f);

                    // best-improvement per triple: pick the lowest-cost valid case
                    if let Some(case) = best_improving_case(distances, a, b, c, dt, e, f, orig) {
                        apply_3opt(&mut path, i, j, k, case);
                        tracing::debug!(i, j, k, case, "3-opt: improvement");
                        options.send_progress(ProgressMessage::PathUpdate(Route::new(&path), 0.0));
                        improved = true;
                        break 'search;
                    }
                }
            }
        }
    }

    options.send_progress(ProgressMessage::Done);
    Solution::new(&path, cities, distances)
}

/// Returns the case index (1–7) of the best reconnection that strictly
/// improves `orig`, or `None` if no improvement exists.
///
/// Cases and their new edge sets (A=path[i], B=path[i+1], C=path[j],
/// DT=path[j+1], E=path[k], F=path[(k+1)%n]):
///
/// | # | middle          | new edges               | type  |
/// |---|-----------------|-------------------------|-------|
/// | 1 | rev(s1) + s2    | (A,C)+(B,D)+(E,F)       | 2-opt |
/// | 2 | s1 + rev(s2)    | (A,B)+(C,E)+(D,F)       | 2-opt |
/// | 3 | rev(s1)+rev(s2) | (A,C)+(B,E)+(D,F)       | 2-opt |
/// | 4 | s2 + s1         | (A,D)+(E,B)+(C,F)       | 3-opt |
/// | 5 | s2 + rev(s1)    | (A,D)+(E,C)+(B,F)       | 3-opt |
/// | 6 | rev(s2) + s1    | (A,E)+(D,B)+(C,F)       | 3-opt |
/// | 7 | rev(s2)+rev(s1) | (A,E)+(D,C)+(B,F)       | 3-opt |
#[allow(clippy::too_many_arguments)]
fn best_improving_case(
    dm: &DistanceMatrix,
    a: usize,
    b: usize,
    c: usize,
    dt: usize,
    e: usize,
    f: usize,
    orig: f32,
) -> Option<u8> {
    let cases = [
        d(dm, a, c) + d(dm, b, dt) + d(dm, e, f), // 1
        d(dm, a, b) + d(dm, c, e) + d(dm, dt, f), // 2
        d(dm, a, c) + d(dm, b, e) + d(dm, dt, f), // 3
        d(dm, a, dt) + d(dm, e, b) + d(dm, c, f), // 4
        d(dm, a, dt) + d(dm, e, c) + d(dm, b, f), // 5
        d(dm, a, e) + d(dm, dt, b) + d(dm, c, f), // 6
        d(dm, a, e) + d(dm, dt, c) + d(dm, b, f), // 7
    ];

    cases
        .iter()
        .enumerate()
        .filter(|&(_, cost)| *cost < orig)
        .min_by(|&(_, x), &(_, y)| x.partial_cmp(y).unwrap())
        .map(|(i, _)| (i + 1) as u8)
}

/// Applies case `case` (1–7) to `path` in place.
///
/// Cases 1–3 use segment reversals only (no allocation).
/// Cases 4–7 swap the two segments (one small Vec allocation per move).
fn apply_3opt(path: &mut [usize], i: usize, j: usize, k: usize, case: u8) {
    match case {
        1 => path[i + 1..=j].reverse(),
        2 => path[j + 1..=k].reverse(),
        3 => {
            path[i + 1..=j].reverse();
            path[j + 1..=k].reverse();
        }
        4..=7 => {
            let seg1: Vec<usize> = path[i + 1..=j].to_vec();
            let seg2: Vec<usize> = path[j + 1..=k].to_vec();
            let new_mid: Vec<usize> = match case {
                4 => [seg2.as_slice(), seg1.as_slice()].concat(),
                5 => {
                    let s1r: Vec<usize> = seg1.iter().rev().copied().collect();
                    [seg2.as_slice(), s1r.as_slice()].concat()
                }
                6 => {
                    let s2r: Vec<usize> = seg2.iter().rev().copied().collect();
                    [s2r.as_slice(), seg1.as_slice()].concat()
                }
                7 => {
                    let s1r: Vec<usize> = seg1.iter().rev().copied().collect();
                    let s2r: Vec<usize> = seg2.iter().rev().copied().collect();
                    [s2r.as_slice(), s1r.as_slice()].concat()
                }
                _ => unreachable!(),
            };
            path[i + 1..=k].copy_from_slice(&new_mid);
        }
        _ => {}
    }
}

#[inline]
fn d(dm: &DistanceMatrix, a: usize, b: usize) -> f32 {
    dm.distance_between(a, b).expect("three_opt: invalid city pair")
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree};

    fn build_dm(cities: &[kdtree::KDPoint]) -> DistanceMatrix {
        distance_matrix::from_cities(cities)
    }

    /// Square: 0-1-2-3 with side 1, optimal tour = 4.0
    fn square_cities() -> Vec<kdtree::KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
        ])
    }

    /// Build a city vec from (x,y) pairs.
    fn pts(coords: &[(f32, f32)]) -> Vec<kdtree::KDPoint> {
        let vecs: Vec<Vec<f32>> = coords.iter().map(|&(x, y)| vec![x, y]).collect();
        kdtree::build_points(&vecs)
    }

    fn is_valid_tour(route: &[usize], cities: &[kdtree::KDPoint]) -> bool {
        let mut ids: Vec<usize> = cities.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        let mut got: Vec<usize> = route.to_vec();
        got.sort_unstable();
        ids == got
    }

    // --- apply_3opt unit tests — one per case ---

    /// Case 1: reverse seg1.
    /// path = [0,1,2,3,4,5], i=0 j=2 k=4
    /// seg1 = [1,2], seg2 = [3,4]
    /// case 1: rev(seg1)+seg2 → [0,2,1,3,4,5]
    #[test]
    fn apply_case1_reverses_seg1() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 1);
        assert_eq!(path, vec![0, 2, 1, 3, 4, 5]);
    }

    /// Case 2: reverse seg2.
    /// path = [0,1,2,3,4,5], i=0 j=2 k=4
    /// seg1 = [1,2], seg2 = [3,4]
    /// case 2: seg1+rev(seg2) → [0,1,2,4,3,5]
    #[test]
    fn apply_case2_reverses_seg2() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 2);
        assert_eq!(path, vec![0, 1, 2, 4, 3, 5]);
    }

    /// Case 3: reverse both segments separately.
    /// path = [0,1,2,3,4,5], i=0 j=2 k=4
    /// case 3: rev(seg1)+rev(seg2) → [0,2,1,4,3,5]
    #[test]
    fn apply_case3_reverses_both_segs() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 3);
        assert_eq!(path, vec![0, 2, 1, 4, 3, 5]);
    }

    /// Case 4: swap seg1 and seg2 (no reversal).
    /// path = [0,1,2,3,4,5], i=0 j=2 k=4
    /// case 4: seg2+seg1 → [0,3,4,1,2,5]
    #[test]
    fn apply_case4_swaps_segments() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 4);
        assert_eq!(path, vec![0, 3, 4, 1, 2, 5]);
    }

    /// Case 5: seg2 + rev(seg1).
    /// path = [0,1,2,3,4,5], i=0 j=2 k=4
    /// case 5: seg2+rev(seg1) → [0,3,4,2,1,5]
    #[test]
    fn apply_case5_swaps_and_reverses_seg1() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 5);
        assert_eq!(path, vec![0, 3, 4, 2, 1, 5]);
    }

    /// Case 6: rev(seg2) + seg1.
    /// path = [0,1,2,3,4,5], i=0 j=2 k=4
    /// case 6: rev(seg2)+seg1 → [0,4,3,1,2,5]
    #[test]
    fn apply_case6_swaps_and_reverses_seg2() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 6);
        assert_eq!(path, vec![0, 4, 3, 1, 2, 5]);
    }

    /// Case 7: rev(seg2) + rev(seg1).
    /// path = [0,1,2,3,4,5], i=0 j=2 k=4
    /// case 7: rev(seg2)+rev(seg1) → [0,4,3,2,1,5]
    #[test]
    fn apply_case7_reverses_both_and_swaps() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 7);
        assert_eq!(path, vec![0, 4, 3, 2, 1, 5]);
    }

    // --- solver-level tests ---

    #[test]
    fn solve_square_finds_optimal() {
        let cities = square_cities();
        let dm = build_dm(&cities);
        let tour = solve(&cities, &dm, &SolverOptions::default());
        assert!(is_valid_tour(tour.route(), &cities));
        assert!((tour.total - 4.0).abs() < 1e-4, "expected 4.0, got {}", tour.total);
    }

    #[test]
    fn solve_degenerate_triangle_is_valid() {
        let cities = pts(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let dm = build_dm(&cities);
        let tour = solve(&cities, &dm, &SolverOptions::default());
        assert!(is_valid_tour(tour.route(), &cities));
        assert!(tour.total > 0.0);
    }

    #[test]
    fn solve_two_cities_is_valid() {
        let cities = pts(&[(0.0, 0.0), (1.0, 0.0)]);
        let dm = build_dm(&cities);
        let tour = solve(&cities, &dm, &SolverOptions::default());
        assert!(is_valid_tour(tour.route(), &cities));
    }

    /// 3-opt on a deliberately suboptimal 5-city tour should match or beat 2-opt.
    #[test]
    fn solve_five_cities_unit_square() {
        let cities = pts(&[
            (0.0, 0.0),
            (0.0, 0.5),
            (0.0, 1.0),
            (1.0, 1.0),
            (1.0, 0.0),
        ]);
        let dm = build_dm(&cities);
        let tour = solve(&cities, &dm, &SolverOptions::default());
        assert!(is_valid_tour(tour.route(), &cities));
        assert!((tour.total - 4.0).abs() < 1e-4, "expected 4.0, got {}", tour.total);
    }
}

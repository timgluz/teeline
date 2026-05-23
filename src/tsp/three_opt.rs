use std::sync::mpsc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{HeuristicOptions, Solution};

/// 3-opt local search.
///
/// Starts from a nearest-neighbor seed. Each pass scans all O(n³) triples and
/// applies the globally best improving move (best-improvement-per-pass), then
/// repeats until no triple yields an improvement.
///
/// Complexity: O(n³) per pass. The number of passes is bounded by the number
/// of distinct improving moves, which is small on a NN-seeded tour.
pub fn solve(
    cities: &[KDPoint],
    distances: &DistanceMatrix,
    _opts: &HeuristicOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    initial_tour: Option<&[usize]>,
) -> Solution {
    let n = cities.len();
    if n < 4 {
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        return Solution::from_parts(&path, cities, distances);
    }

    let mut path: Vec<usize> = initial_tour
        .map(|t| t.to_vec())
        .unwrap_or_else(|| cities.iter().map(|c| c.id).collect());

    send_path(progress_tx, &path);

    let mut improved = true;
    while improved {
        improved = false;
        if let Some((i, j, k, case)) = find_best_move(&path, distances) {
            tracing::debug!(i, j, k, case, "3-opt: best improvement");
            apply_3opt(&mut path, i, j, k, case);
            send_path(progress_tx, &path);
            improved = true;
        }
    }

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&path, cities, distances)
}

fn find_best_move(
    path: &[usize],
    distances: &DistanceMatrix,
) -> Option<(usize, usize, usize, u8)> {
    let n = path.len();
    let mut best_move: Option<(usize, usize, usize, u8)> = None;
    let mut best_savings = 0.0_f32;

    for i in 0..n - 2 {
        let a = path[i];
        let b = path[i + 1];
        let d_ab = d(distances, a, b);

        for j in i + 1..n - 1 {
            let c = path[j];
            let dt = path[j + 1];
            let d_c_dt = d(distances, c, dt);
            let d_ac   = d(distances, a, c);
            let d_b_dt = d(distances, b, dt);
            let d_a_dt = d(distances, a, dt);

            for k in j + 1..n {
                if i == 0 && k == n - 1 {
                    continue;
                }

                let e = path[k];
                let f = path[(k + 1) % n];

                let d_ef   = d(distances, e, f);
                let d_ce   = d(distances, c, e);
                let d_dt_f = d(distances, dt, f);
                let d_be   = d(distances, b, e);
                let d_cf   = d(distances, c, f);
                let d_bf   = d(distances, b, f);
                let d_ae   = d(distances, a, e);

                let orig = d_ab + d_c_dt + d_ef;
                let te = TripleEdges {
                    d_ab, d_c_dt, d_ac, d_b_dt, d_a_dt,
                    d_ef, d_ce, d_dt_f, d_be, d_cf, d_bf, d_ae,
                };
                let costs = reconnection_costs(&te);

                let maybe_best = costs
                    .iter()
                    .enumerate()
                    .filter(|&(_, &cost)| cost < orig)
                    .min_by(|&(_, x), &(_, y)| x.partial_cmp(y).unwrap());

                if let Some((ci, &new_cost)) = maybe_best {
                    let savings = orig - new_cost;
                    if savings > best_savings {
                        best_savings = savings;
                        best_move = Some((i, j, k, (ci + 1) as u8));
                    }
                }
            }
        }
    }

    best_move
}

struct TripleEdges {
    d_ab: f32,
    d_c_dt: f32, d_ac: f32, d_b_dt: f32, d_a_dt: f32,
    d_ef: f32, d_ce: f32, d_dt_f: f32, d_be: f32, d_cf: f32, d_bf: f32, d_ae: f32,
}

fn reconnection_costs(e: &TripleEdges) -> [f32; 7] {
    [
        e.d_ac  + e.d_b_dt + e.d_ef,
        e.d_ab  + e.d_ce   + e.d_dt_f,
        e.d_ac  + e.d_be   + e.d_dt_f,
        e.d_a_dt + e.d_be  + e.d_cf,
        e.d_a_dt + e.d_ce  + e.d_bf,
        e.d_ae  + e.d_b_dt + e.d_cf,
        e.d_ae  + e.d_c_dt + e.d_bf,
    ]
}

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
        _ => unreachable!("apply_3opt: case must be 1–7, got {case}"),
    }
}

#[inline]
fn d(dm: &DistanceMatrix, a: usize, b: usize) -> f32 {
    dm.distance_between(a, b).expect("three_opt: invalid city pair")
}

fn send_path(tx: Option<&mpsc::Sender<ProgressMessage>>, path: &[usize]) {
    if let Some(t) = tx {
        let _ = t.send(ProgressMessage::PathUpdate(Route::new(path), 0.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, HeuristicOptions};

    fn build_dm(cities: &[kdtree::KDPoint]) -> DistanceMatrix {
        distance_matrix::from_cities(cities)
    }

    fn square_cities() -> Vec<kdtree::KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
        ])
    }

    fn pts(coords: &[(f32, f32)]) -> Vec<kdtree::KDPoint> {
        let vecs: Vec<Vec<f32>> = coords.iter().map(|&(x, y)| vec![x, y]).collect();
        kdtree::build_points(&vecs)
    }

    #[test]
    fn test_three_opt_respects_initial_tour() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);
        let dm = build_dm(&cities);
        let optimal: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let result = solve(&cities, &dm, &HeuristicOptions::default(), None,Some(&optimal));
        assert_eq!(result.route(), optimal.as_slice());
    }

    fn is_valid_tour(route: &[usize], cities: &[kdtree::KDPoint]) -> bool {
        let mut ids: Vec<usize> = cities.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        let mut got: Vec<usize> = route.to_vec();
        got.sort_unstable();
        ids == got
    }

    #[test]
    fn apply_case1_reverses_seg1() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 1);
        assert_eq!(path, vec![0, 2, 1, 3, 4, 5]);
    }

    #[test]
    fn apply_case2_reverses_seg2() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 2);
        assert_eq!(path, vec![0, 1, 2, 4, 3, 5]);
    }

    #[test]
    fn apply_case3_reverses_both_segs() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 3);
        assert_eq!(path, vec![0, 2, 1, 4, 3, 5]);
    }

    #[test]
    fn apply_case4_swaps_segments() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 4);
        assert_eq!(path, vec![0, 3, 4, 1, 2, 5]);
    }

    #[test]
    fn apply_case5_swaps_and_reverses_seg1() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 5);
        assert_eq!(path, vec![0, 3, 4, 2, 1, 5]);
    }

    #[test]
    fn apply_case6_swaps_and_reverses_seg2() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 6);
        assert_eq!(path, vec![0, 4, 3, 1, 2, 5]);
    }

    #[test]
    fn apply_case7_reverses_both_and_swaps() {
        let mut path = vec![0usize, 1, 2, 3, 4, 5];
        apply_3opt(&mut path, 0, 2, 4, 7);
        assert_eq!(path, vec![0, 4, 3, 2, 1, 5]);
    }

    #[test]
    fn find_best_move_returns_none_on_optimal() {
        let cities = square_cities();
        let dm = build_dm(&cities);
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        assert_eq!(find_best_move(&path, &dm), None);
    }

    #[test]
    fn find_best_move_finds_improvement() {
        let cities = pts(&[
            (0.0, 0.0),
            (0.0, 0.5),
            (0.0, 1.0),
            (1.0, 1.0),
            (1.0, 0.0),
        ]);
        let dm = build_dm(&cities);
        let bad_path = vec![0usize, 2, 4, 1, 3];
        let orig_cost: f32 = dm.tour_length(&bad_path);

        let result = find_best_move(&bad_path, &dm);
        assert!(result.is_some(), "expected an improving move on the crossed tour");

        let (i, j, k, case) = result.unwrap();
        let mut improved_path = bad_path.clone();
        apply_3opt(&mut improved_path, i, j, k, case);
        let new_cost = dm.tour_length(&improved_path);
        assert!(
            new_cost < orig_cost,
            "applying the move should reduce cost ({new_cost:.4} < {orig_cost:.4})"
        );
    }

    fn sample_edges() -> TripleEdges {
        TripleEdges {
            d_ab: 1.0, d_c_dt: 2.0, d_ac: 3.0, d_b_dt: 4.0, d_a_dt: 5.0,
            d_ef: 6.0, d_ce: 7.0, d_dt_f: 8.0, d_be: 9.0, d_cf: 10.0, d_bf: 11.0, d_ae: 12.0,
        }
    }

    #[test]
    fn reconnection_costs_case1() {
        assert_eq!(reconnection_costs(&sample_edges())[0], 13.0);
    }

    #[test]
    fn reconnection_costs_case2() {
        assert_eq!(reconnection_costs(&sample_edges())[1], 16.0);
    }

    #[test]
    fn reconnection_costs_case3() {
        assert_eq!(reconnection_costs(&sample_edges())[2], 20.0);
    }

    #[test]
    fn reconnection_costs_case4() {
        assert_eq!(reconnection_costs(&sample_edges())[3], 24.0);
    }

    #[test]
    fn reconnection_costs_case5() {
        assert_eq!(reconnection_costs(&sample_edges())[4], 23.0);
    }

    #[test]
    fn reconnection_costs_case6() {
        assert_eq!(reconnection_costs(&sample_edges())[5], 26.0);
    }

    #[test]
    fn reconnection_costs_case7() {
        assert_eq!(reconnection_costs(&sample_edges())[6], 25.0);
    }

    #[test]
    fn solve_square_finds_optimal() {
        let cities = square_cities();
        let dm = build_dm(&cities);
        let tour = solve(&cities, &dm, &HeuristicOptions::default(), None,None);
        assert!(is_valid_tour(tour.route(), &cities));
        assert!((tour.total - 4.0).abs() < 1e-4, "expected 4.0, got {}", tour.total);
    }

    #[test]
    fn solve_degenerate_triangle_is_valid() {
        let cities = pts(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let dm = build_dm(&cities);
        let tour = solve(&cities, &dm, &HeuristicOptions::default(), None,None);
        assert!(is_valid_tour(tour.route(), &cities));
        assert!(tour.total > 0.0);
    }

    #[test]
    fn solve_two_cities_is_valid() {
        let cities = pts(&[(0.0, 0.0), (1.0, 0.0)]);
        let dm = build_dm(&cities);
        let tour = solve(&cities, &dm, &HeuristicOptions::default(), None,None);
        assert!(is_valid_tour(tour.route(), &cities));
    }

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
        let tour = solve(&cities, &dm, &HeuristicOptions::default(), None,None);
        assert!(is_valid_tour(tour.route(), &cities));
        assert!((tour.total - 4.0).abs() < 1e-4, "expected 4.0, got {}", tour.total);
    }
}

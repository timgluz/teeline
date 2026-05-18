use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{nearest_neighbor, Solution, SolverOptions};

/// 3-opt local search.
///
/// Starts from a nearest-neighbor seed. Each pass scans all O(n³) triples and
/// applies the globally best improving move (best-improvement-per-pass), then
/// repeats until no triple yields an improvement.
///
/// Complexity: O(n³) per pass. The number of passes is bounded by the number
/// of distinct improving moves, which is small on a NN-seeded tour.
pub fn solve(cities: &[KDPoint], distances: &DistanceMatrix, options: &SolverOptions) -> Solution {
    let n = cities.len();
    if n < 4 {
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        return Solution::new(&path, cities, distances);
    }

    // Seed with a nearest-neighbor tour. Progress messages from the NN phase
    // are suppressed so the caller only sees 3-opt updates.
    let seed_opts = SolverOptions { progress_tx: None, ..options.clone() };
    let mut path: Vec<usize> = nearest_neighbor::solve(cities, distances, &seed_opts)
        .route()
        .to_vec();

    send_path(options, &path);

    let mut improved = true;
    while improved {
        improved = false;
        if let Some((i, j, k, case)) = find_best_move(&path, distances) {
            tracing::debug!(i, j, k, case, "3-opt: best improvement");
            apply_3opt(&mut path, i, j, k, case);
            send_path(options, &path);
            improved = true;
        }
    }

    options.send_progress(ProgressMessage::Done);
    Solution::new(&path, cities, distances)
}

/// Scans all C(n,3) triples and returns the coordinates of the globally best
/// improving 3-opt move `(i, j, k, case)`, or `None` if the tour is already
/// locally optimal.
///
/// Pure: no side effects, no progress messages.
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
        // Hoist edge (A,B) — invariant for all j, k given i.
        let d_ab = d(distances, a, b);

        for j in i + 1..n - 1 {
            let c = path[j];
            let dt = path[j + 1];
            // Hoist (i,j)-invariant distances: 4 lookups saved per k iteration.
            let d_c_dt = d(distances, c, dt);
            let d_ac   = d(distances, a, c);
            let d_b_dt = d(distances, b, dt);
            let d_a_dt = d(distances, a, dt);

            for k in j + 1..n {
                // Skip the degenerate triple where i==0, k==n-1: the wrap-around
                // edge makes F==A, causing the formula to measure A→A.
                if i == 0 && k == n - 1 {
                    continue;
                }

                let e = path[k];
                let f = path[(k + 1) % n];

                // Per-k distances, each shared by multiple cases below.
                let d_ef   = d(distances, e, f);
                let d_ce   = d(distances, c, e);
                let d_dt_f = d(distances, dt, f);
                let d_be   = d(distances, b, e);
                let d_cf   = d(distances, c, f);
                let d_bf   = d(distances, b, f);
                let d_ae   = d(distances, a, e);

                let orig = d_ab + d_c_dt + d_ef;

                // All 7 non-identity reconnection costs.
                // Cases and their new edge sets (A=path[i], B=path[i+1],
                // C=path[j], D=path[j+1], E=path[k], F=path[(k+1)%n]):
                //
                // | # | middle           | new edges              |
                // |---|------------------|------------------------|
                // | 1 | rev(s1)+s2       | (A,C)+(B,D)+(E,F)      |
                // | 2 | s1+rev(s2)       | (A,B)+(C,E)+(D,F)      |
                // | 3 | rev(s1)+rev(s2)  | (A,C)+(B,E)+(D,F)      |
                // | 4 | s2+s1            | (A,D)+(E,B)+(C,F)      |
                // | 5 | s2+rev(s1)       | (A,D)+(E,C)+(B,F)      |
                // | 6 | rev(s2)+s1       | (A,E)+(D,B)+(C,F)      |
                // | 7 | rev(s2)+rev(s1)  | (A,E)+(D,C)+(B,F)      |
                let costs = [
                    d_ac + d_b_dt + d_ef,   // 1
                    d_ab + d_ce   + d_dt_f, // 2
                    d_ac + d_be   + d_dt_f, // 3
                    d_a_dt + d_be + d_cf,   // 4
                    d_a_dt + d_ce + d_bf,   // 5
                    d_ae + d_b_dt + d_cf,   // 6
                    d_ae + d_c_dt + d_bf,   // 7
                ];

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

fn send_path(options: &SolverOptions, path: &[usize]) {
    if options.progress_tx.is_some() {
        options.send_progress(ProgressMessage::PathUpdate(Route::new(path), 0.0));
    }
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

    // --- find_best_move unit tests ---

    /// Optimal 4-city square tour should have no improving 3-opt move.
    #[test]
    fn find_best_move_returns_none_on_optimal() {
        let cities = square_cities();
        let dm = build_dm(&cities);
        // NN produces the optimal tour for the square.
        let path: Vec<usize> = cities.iter().map(|c| c.id).collect();
        assert_eq!(find_best_move(&path, &dm), None);
    }

    /// A deliberately bad 5-city tour (with crossings) must have an improving move,
    /// and applying it must strictly lower the tour cost.
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
        // Crossed tour: 0→2→4→1→3 (cost ≈ 6.06, well above optimal 4.0).
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

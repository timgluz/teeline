//! Shared exact fixed-endpoint shortest-path dynamic program.
//!
//! Both the exact Bellman–Held–Karp solver and the POPMUSIC decomposition solver need the
//! cheapest open path through a small set of interior nodes with fixed endpoints. Keeping
//! one implementation means the two cannot drift in reconstruction or tie-breaking.

/// Hard cap on the interior-node count. `m` interior nodes need `m · 2^m` f64 cells; the
/// exact solver's 24-city cap produces `m = 23`, which is already about 1.5 GB, so this is
/// deliberately the largest set that cap can produce rather than a comfortable allocation
/// target.
pub(crate) const MAX_INTERIOR: usize = 23;

/// Cheapest open path from `start` through exactly the `m` interior nodes (each once, any
/// order), ending at `end`.
///
/// `matrix` is a packed row-major `(m + 2)²` distance table over local indices: `0..m` are
/// the interior nodes, `m` is the start and `m + 1` is the end. It is the only source of
/// distances, so the DP's hot loop performs array reads instead of dynamic queries.
///
/// Distances are widened to `f64` before they are summed, matching
/// [`DistanceSource::tour_length`](crate::tsp::DistanceSource::tour_length), which also
/// accumulates in `f64`. An `f32` recurrence ranks tours by their rounded running sums; at
/// the edge lengths of large TSPLIB instances (near `2^24`) rounding can make it pick a
/// tour whose exact cost is not the minimum.
///
/// Returns `(open-path cost, interior visit order as local indices)`, or `None` when no
/// finite path exists. Ties are broken by first scan order, so the result is deterministic
/// for a given matrix.
///
/// # Panics
///
/// Panics when `m > [`MAX_INTERIOR`]` or `matrix` is not exactly `(m + 2)²` entries.
pub(crate) fn held_karp_path(matrix: &[f32], m: usize) -> Option<(f64, Vec<usize>)> {
    let mut dp = Vec::new();
    held_karp_path_with_scratch(matrix, m, &mut dp)
}

/// [`held_karp_path`] with a caller-owned `dp` buffer, so a solver that runs the DP once
/// per window (POPMUSIC) reuses one allocation instead of allocating `m · 2^m` cells per
/// window.
pub(crate) fn held_karp_path_with_scratch(
    matrix: &[f32],
    m: usize,
    dp: &mut Vec<f64>,
) -> Option<(f64, Vec<usize>)> {
    assert!(
        m <= MAX_INTERIOR,
        "held_karp_path: {m} interior nodes exceeds MAX_INTERIOR={MAX_INTERIOR}"
    );
    let stride = m + 2;
    assert_eq!(
        matrix.len(),
        stride * stride,
        "held_karp_path: matrix must be a packed {stride}x{stride} table"
    );

    let d = |a: usize, b: usize| f64::from(matrix[a * stride + b]);

    if m == 0 {
        let cost = d(m, m + 1);
        return cost.is_finite().then(|| (cost, Vec::new()));
    }

    let states = 1usize << m;
    // dp[mask * m + last] = cheapest path from `start` through exactly `mask`, ending at
    // `interior[last]`. INFINITY (not f32::MAX) because MAX + d stays MAX under rounding.
    // Reserve fallibly: the f64 table is about 1.5 GB at the exact solver's cap, and an
    // allocation failure should reach the caller as `None`, not abort the process.
    dp.clear();
    dp.try_reserve_exact(states * m).ok()?;
    dp.resize(states * m, f64::INFINITY);
    for i in 0..m {
        dp[(1 << i) * m + i] = d(m, i);
    }

    for mask in 1..states {
        for last in 0..m {
            if mask & (1 << last) == 0 {
                continue;
            }
            let prev_mask = mask ^ (1 << last);
            if prev_mask == 0 {
                continue;
            }
            let mut best = f64::INFINITY;
            for prev in 0..m {
                if prev_mask & (1 << prev) == 0 {
                    continue;
                }
                let candidate = dp[prev_mask * m + prev] + d(prev, last);
                if candidate < best {
                    best = candidate;
                }
            }
            dp[mask * m + last] = best;
        }
    }

    let full = states - 1;
    let mut best_cost = f64::INFINITY;
    let mut best_last = None;
    for last in 0..m {
        let candidate = dp[full * m + last] + d(last, m + 1);
        if candidate < best_cost {
            best_cost = candidate;
            best_last = Some(last);
        }
    }
    if !best_cost.is_finite() {
        return None;
    }
    let mut last = best_last?;

    // Walk the table backward with the same first-strict-`<` tie-break the forward pass
    // used, so the predecessor chosen here is the one that produced `dp[mask][last]`.
    // Recomputing `dp[prev] + d(prev, last)` repeats the forward operands exactly; no
    // parent table (which would double BHK's memory) and no tolerance are needed.
    let mut order = Vec::with_capacity(m);
    let mut mask = full;
    loop {
        order.push(last);
        let prev_mask = mask ^ (1 << last);
        if prev_mask == 0 {
            break;
        }
        let mut best_prev = None;
        let mut best_prev_cost = f64::INFINITY;
        for prev in 0..m {
            if prev_mask & (1 << prev) == 0 {
                continue;
            }
            let candidate = dp[prev_mask * m + prev] + d(prev, last);
            if candidate < best_prev_cost {
                best_prev_cost = candidate;
                best_prev = Some(prev);
            }
        }
        last = best_prev?;
        mask = prev_mask;
    }
    order.reverse();
    Some((best_cost, order))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngExt;
    use rand::SeedableRng;

    fn packed_matrix(m: usize, mut fill: impl FnMut(usize, usize) -> f32) -> Vec<f32> {
        let stride = m + 2;
        let mut matrix = vec![0.0; stride * stride];
        for a in 0..stride {
            for b in 0..stride {
                if a != b {
                    matrix[a * stride + b] = fill(a, b);
                }
            }
        }
        matrix
    }

    fn path_cost(matrix: &[f32], m: usize, order: &[usize]) -> f64 {
        let stride = m + 2;
        let d = |a: usize, b: usize| f64::from(matrix[a * stride + b]);
        if order.is_empty() {
            return d(m, m + 1);
        }
        let mut total = d(m, order[0]);
        for pair in order.windows(2) {
            total += d(pair[0], pair[1]);
        }
        total + d(order[order.len() - 1], m + 1)
    }

    fn for_each_permutation(m: usize, f: &mut impl FnMut(&[usize])) {
        fn rec(items: &mut [usize], k: usize, f: &mut impl FnMut(&[usize])) {
            if k == 1 {
                f(items);
                return;
            }
            for i in 0..k {
                rec(items, k - 1, f);
                if k % 2 == 0 {
                    items.swap(i, k - 1);
                } else {
                    items.swap(0, k - 1);
                }
            }
        }
        let mut items: Vec<usize> = (0..m).collect();
        if m == 0 {
            f(&items);
        } else {
            rec(&mut items, m, f);
        }
    }

    fn brute_force(matrix: &[f32], m: usize) -> f64 {
        let mut best = f64::INFINITY;
        for_each_permutation(m, &mut |order| {
            let cost = path_cost(matrix, m, order);
            if cost < best {
                best = cost;
            }
        });
        best
    }

    fn assert_matches_brute_force(matrix: &[f32], m: usize) {
        let (cost, order) = held_karp_path(matrix, m).expect("a finite path exists");
        let expected = brute_force(matrix, m);
        assert_eq!(cost, expected, "m={m}: DP cost disagrees with brute force");
        let realized = path_cost(matrix, m, &order);
        assert_eq!(
            realized, cost,
            "m={m}: returned order {order:?} does not realize the reported cost"
        );
    }

    #[test]
    fn empty_interior_is_the_direct_edge() {
        let matrix = packed_matrix(0, |a, b| (a * 10 + b) as f32);
        let (cost, order) = held_karp_path(&matrix, 0).unwrap();
        assert_eq!(cost, f64::from(matrix[0 * 2 + 1]));
        assert!(order.is_empty());
    }

    #[test]
    fn single_interior_node_has_a_two_edge_path() {
        let matrix = packed_matrix(1, |a, b| (a * 10 + b) as f32);
        let (cost, order) = held_karp_path(&matrix, 1).unwrap();
        let expected = f64::from(matrix[1 * 3 + 0]) + f64::from(matrix[0 * 3 + 2]);
        assert_eq!(cost, expected);
        assert_eq!(order, vec![0]);
    }

    #[test]
    fn matches_brute_force_symmetric() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        for m in 3..=8 {
            for _ in 0..3 {
                let stride = m + 2;
                let mut matrix = vec![0.0f32; stride * stride];
                for a in 0..stride {
                    for b in (a + 1)..stride {
                        let value = rng.random_range(1.0f32..100.0);
                        matrix[a * stride + b] = value;
                        matrix[b * stride + a] = value;
                    }
                }
                assert_matches_brute_force(&matrix, m);
            }
        }
    }

    #[test]
    fn matches_brute_force_asymmetric() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(11);
        for m in 3..=7 {
            for _ in 0..3 {
                let matrix = packed_matrix(m, |_, _| rng.random_range(1.0f32..100.0));
                assert_matches_brute_force(&matrix, m);
            }
        }
    }

    #[test]
    fn matches_brute_force_with_ties() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(13);
        for m in 3..=7 {
            // Integer entries in a tiny range guarantee many equal edge costs.
            let matrix = packed_matrix(m, |_, _| rng.random_range(0..4) as f32);
            assert_matches_brute_force(&matrix, m);
        }
    }

    /// Regression: an `f32` recurrence ranked `[1, 2, 0]` (exact cost 67,108,870) above the
    /// true optimum `[1, 0, 2]` (67,108,866) once edges reached `2^24`. The matrix is the
    /// same one the review reported; all values are exactly representable in `f32`.
    #[test]
    fn f64_accumulation_keeps_the_optimal_tour() {
        let matrix: Vec<f32> = vec![
            0.0, 16777216.0, 16777216.0, 33554432.0, 16777220.0, //
            16777216.0, 0.0, 16777216.0, 16777218.0, 16777220.0, //
            16777216.0, 16777216.0, 0.0, 16777220.0, 16777216.0, //
            33554432.0, 16777218.0, 16777220.0, 0.0, 16777218.0, //
            16777220.0, 16777220.0, 16777216.0, 16777218.0, 0.0,
        ];
        let (cost, order) = held_karp_path(&matrix, 3).unwrap();
        assert_eq!(cost, 67_108_866.0);
        assert_eq!(order, vec![1, 0, 2]);
        assert_eq!(path_cost(&matrix, 3, &order), cost);
    }

    #[test]
    fn scratch_buffer_matches_the_allocating_wrapper() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(19);
        let m = 6;
        let matrix = packed_matrix(m, |_, _| rng.random_range(1.0f32..20.0));
        let expected = held_karp_path(&matrix, m).unwrap();
        let mut dp = Vec::new();
        let first = held_karp_path_with_scratch(&matrix, m, &mut dp).unwrap();
        let second = held_karp_path_with_scratch(&matrix, m, &mut dp).unwrap();
        assert_eq!(expected, first);
        assert_eq!(first, second);
    }

    #[test]
    fn reconstruction_is_deterministic() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(17);
        let m = 6;
        let matrix = packed_matrix(m, |_, _| rng.random_range(1.0f32..20.0));
        let first = held_karp_path(&matrix, m).unwrap();
        let second = held_karp_path(&matrix, m).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn unreachable_interior_returns_none() {
        let m = 2;
        let stride = m + 2;
        let mut matrix = packed_matrix(m, |_, _| 1.0);
        // Cut every edge leaving the start, so no interior node is reachable.
        for interior in 0..m {
            matrix[m * stride + interior] = f32::INFINITY;
        }
        assert!(held_karp_path(&matrix, m).is_none());
    }

    #[test]
    #[should_panic(expected = "MAX_INTERIOR")]
    fn rejects_an_interior_above_the_bound() {
        let m = MAX_INTERIOR + 1;
        let matrix = vec![0.0; (m + 2) * (m + 2)];
        let _ = held_karp_path(&matrix, m);
    }

    #[test]
    #[should_panic(expected = "packed")]
    fn rejects_a_matrix_with_the_wrong_size() {
        let _ = held_karp_path(&[0.0; 5], 2);
    }
}

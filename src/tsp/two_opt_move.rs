//! Reusable 2-opt move primitives.
//!
//! Both the 2-opt solver and the memetic EAX polish pass cut two tour edges and reconnect
//! the endpoints; only their search strategy differs (full best-improvement vs dirty-edge
//! first-improvement). Keeping the delta and reversal here stops the two from drifting.

/// Edge delta of replacing `(tour[i], tour[i+1])` and `(tour[j], tour[j+1])` with
/// `(tour[i], tour[j])` and `(tour[i+1], tour[j+1])`.
///
/// Computed in `f64`: adding four separately rounded `f32` distances in `f32` can make a
/// move and its reverse both look improving at large-instance edge lengths.
pub(crate) fn delta(
    tour: &[usize],
    i: usize,
    j: usize,
    distance: impl Fn(usize, usize) -> f32,
) -> f64 {
    let n = tour.len();
    let i_next = (i + 1) % n;
    let j_next = (j + 1) % n;
    distance(tour[i], tour[j]) as f64 + distance(tour[i_next], tour[j_next]) as f64
        - distance(tour[i], tour[i_next]) as f64
        - distance(tour[j], tour[j_next]) as f64
}

/// Reverses `tour[from..=to]`; a no-op when `from >= to`.
pub(crate) fn reverse_segment(tour: &mut [usize], from: usize, to: usize) {
    if from >= to {
        return;
    }
    tour[from..=to].reverse();
}

/// Reverses `tour[from..=to]` and keeps `pos` consistent with the new ranks.
pub(crate) fn reverse_segment_with_pos(
    tour: &mut [usize],
    pos: &mut [usize],
    from: usize,
    to: usize,
) {
    let mut left = from;
    let mut right = to;
    while left < right {
        tour.swap(left, right);
        pos[tour[left]] = left;
        pos[tour[right]] = right;
        left += 1;
        right -= 1;
    }
    if left == right {
        pos[tour[left]] = left;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_matches_the_hand_computed_edges() {
        // Tour [0, 1, 2, 3]; cut (0,1) and (2,3), reconnect (0,2) and (1,3).
        let tour = [0usize, 1, 2, 3];
        let unit = |a: usize, b: usize| if a == b { 0.0 } else { 1.0 };
        let d = delta(&tour, 0, 2, unit);
        assert!(
            (d - 0.0).abs() < 1e-12,
            "unit square delta should be zero, got {d}"
        );
    }

    #[test]
    fn reverse_segment_updates_the_tour() {
        let mut tour = [0usize, 1, 2, 3, 4];
        reverse_segment(&mut tour, 1, 3);
        assert_eq!(tour, [0, 3, 2, 1, 4]);
    }

    #[test]
    fn reverse_segment_with_pos_keeps_ranks_consistent() {
        let mut tour = [0usize, 1, 2, 3, 4];
        let mut pos = [0usize; 5];
        for (rank, &city) in tour.iter().enumerate() {
            pos[city] = rank;
        }
        reverse_segment_with_pos(&mut tour, &mut pos, 1, 3);
        for (rank, &city) in tour.iter().enumerate() {
            assert_eq!(pos[city], rank);
        }
    }
}

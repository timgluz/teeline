//! Deterministic 2D layout derived from a distance matrix.
//!
//! TSPLIB EXPLICIT instances may carry distances without coordinates. Geometry-based solvers
//! (SFC, Fourier, SOM) and KD-tree consumers need a 2D arrangement, so the parser derives one
//! with classical multidimensional scaling (MDS): double-centre the squared distances and keep
//! the top two eigenvectors. The result is a distance-preserving sketch, not a georeference —
//! every move is still scored with the exact distance source.
//!
//! Two details are load-bearing:
//!
//! - A double-centred matrix annihilates the all-ones vector (`B·1 = 0`), so all-ones is a
//!   fixed point of the shifted iteration and never reaches a real eigenvector. The starts are
//!   zero-mean trigonometric vectors, re-centred at every step.
//! - Deflating the *shifted* matrix leaves the first eigenvector with eigenvalue `shift`, which
//!   dominates whenever the second eigenvalue is non-positive. The second axis is therefore
//!   deflated from the unshifted matrix and orthogonalized against the first axis.
//!
//! The work is O(n²·iterations) with O(n²) scratch, the same order as the EXPLICIT matrix it is
//! derived from; it never runs on the on-the-fly backend, and coordinate-metric sources keep
//! their file coordinates. All internal arithmetic is `f64`: squared distances and the
//! double-centring subtraction lose too much precision in `f32`, which is used only for the
//! returned coordinates.
//!
//! The algorithm mirrors the minimap layout in `scripts/lib/problem-minimap.mjs`, with two
//! corrections: that script starts from all-ones (the nullspace of a double-centred matrix) and
//! deflates the shifted matrix.

use std::f64::consts::PI;

const MAX_ITERATIONS: usize = 512;
const CONVERGENCE: f64 = 1e-12;
const SIGN_TOLERANCE: f64 = 1e-12;
const NORM_EPSILON: f64 = 1e-12;

/// Derive a 2D layout from `n` cities and a symmetric distance query.
///
/// The returned vector is position-indexed and preserves the caller's order. Non-finite
/// distances are treated as zero: a layout is still produced rather than poisoning every
/// coordinate with `NaN`.
pub(crate) fn mds_embed(n: usize, distance: impl Fn(usize, usize) -> f32) -> Vec<[f32; 2]> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![[0.0, 0.0]];
    }
    if n == 2 {
        let distance = finite_distance(distance(0, 1));
        return vec![[0.0, 0.0], [distance as f32, 0.0]];
    }

    let mut squared = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                let distance = finite_distance(distance(i, j));
                squared[i * n + j] = distance * distance;
            }
        }
    }

    let mut row_means = vec![0.0f64; n];
    let mut grand_total = 0.0f64;
    for i in 0..n {
        let row_total: f64 = squared[i * n..(i + 1) * n].iter().sum();
        row_means[i] = row_total / n as f64;
        grand_total += row_total;
    }
    let grand_mean = grand_total / (n * n) as f64;

    let mut centred = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..n {
            centred[i * n + j] =
                -0.5 * (squared[i * n + j] - row_means[i] - row_means[j] + grand_mean);
        }
    }

    let first = power_iteration(&centred, n, None, Seed::Cosine);
    let first_value = rayleigh(&centred, &first, n);

    let mut deflated = centred;
    for i in 0..n {
        for j in 0..n {
            deflated[i * n + j] -= first_value * first[i] * first[j];
        }
    }

    let second = power_iteration(&deflated, n, Some(&first), Seed::Sine);
    let second_value = rayleigh(&deflated, &second, n);

    let first_scale = first_value.max(0.0).sqrt();
    let second_scale = second_value.max(0.0).sqrt();

    // Centre the scaled axes in one pass. Subtracting the mean of the scaled values is the same
    // arithmetic the per-axis `centre` helper did, so the coordinates stay bit-identical.
    let first_mean = first.iter().map(|&x| x * first_scale).sum::<f64>() / n as f64;
    let second_mean = second.iter().map(|&y| y * second_scale).sum::<f64>() / n as f64;

    first
        .iter()
        .zip(&second)
        .map(|(&x, &y)| {
            [
                (x * first_scale - first_mean) as f32,
                (y * second_scale - second_mean) as f32,
            ]
        })
        .collect()
}

fn finite_distance(distance: f32) -> f64 {
    let distance = f64::from(distance);
    if distance.is_finite() {
        distance.max(0.0)
    } else {
        0.0
    }
}

#[derive(Clone, Copy)]
enum Seed {
    Cosine,
    Sine,
}

/// Shift-and-invert power iteration for the largest algebraic eigenpair of a symmetric matrix.
///
/// The shift (`∞`-norm) makes every eigenvalue non-negative, so the iteration targets the
/// algebraically largest one instead of the largest magnitude. Re-centring removes the
/// nullspace component that would otherwise absorb the iteration.
fn power_iteration(
    matrix: &[f64],
    n: usize,
    orthogonal_to: Option<&[f64]>,
    seed: Seed,
) -> Vec<f64> {
    let mut vector: Vec<f64> = (0..n)
        .map(|i| {
            let angle = 2.0 * PI * i as f64 / n as f64;
            match seed {
                Seed::Cosine => angle.cos(),
                Seed::Sine => angle.sin(),
            }
        })
        .collect();
    normalize(&mut vector);
    if let Some(other) = orthogonal_to {
        orthogonalize(&mut vector, other);
        normalize(&mut vector);
    }

    let shift = infinity_norm(matrix, n);
    for _ in 0..MAX_ITERATIONS {
        let mut next = multiply(matrix, &vector, n);
        for (value, &component) in next.iter_mut().zip(&vector) {
            *value += shift * component;
        }
        centre(&mut next);
        if let Some(other) = orthogonal_to {
            orthogonalize(&mut next, other);
        }
        if !normalize(&mut next) {
            return vec![0.0; n];
        }

        let delta: f64 = next.iter().zip(&vector).map(|(a, b)| (a - b).abs()).sum();
        vector = next;
        if delta < CONVERGENCE {
            break;
        }
    }

    canonicalize_sign(&mut vector);
    vector
}

fn multiply(matrix: &[f64], vector: &[f64], n: usize) -> Vec<f64> {
    let mut result = vec![0.0f64; n];
    for (i, value) in result.iter_mut().enumerate() {
        let row = &matrix[i * n..(i + 1) * n];
        *value = row.iter().zip(vector).map(|(a, b)| a * b).sum();
    }
    result
}

fn infinity_norm(matrix: &[f64], n: usize) -> f64 {
    (0..n)
        .map(|i| {
            matrix[i * n..(i + 1) * n]
                .iter()
                .map(|v| v.abs())
                .sum::<f64>()
        })
        .fold(0.0, f64::max)
}

fn rayleigh(matrix: &[f64], vector: &[f64], n: usize) -> f64 {
    multiply(matrix, vector, n)
        .iter()
        .zip(vector)
        .map(|(a, b)| a * b)
        .sum()
}

fn centre(values: &mut [f64]) {
    if values.is_empty() {
        return;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    for value in values {
        *value -= mean;
    }
}

fn orthogonalize(vector: &mut [f64], other: &[f64]) {
    let projection: f64 = vector.iter().zip(other).map(|(a, b)| a * b).sum();
    for (value, &component) in vector.iter_mut().zip(other) {
        *value -= projection * component;
    }
}

fn normalize(vector: &mut [f64]) -> bool {
    let norm = vector.iter().map(|value| value * value).sum::<f64>().sqrt();
    if norm <= NORM_EPSILON {
        return false;
    }
    for value in vector {
        *value /= norm;
    }
    true
}

/// Fix the arbitrary sign so repeated runs are bit-identical: the lowest index attaining the
/// largest magnitude is made non-negative.
fn canonicalize_sign(vector: &mut [f64]) {
    let largest = vector.iter().map(|value| value.abs()).fold(0.0, f64::max);
    if largest <= SIGN_TOLERANCE {
        return;
    }
    let negative = vector
        .iter()
        .find(|value| value.abs() >= largest - SIGN_TOLERANCE)
        .is_some_and(|value| *value < 0.0);
    if negative {
        for value in vector {
            *value = -*value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn euclidean(points: &[[f64; 2]]) -> impl Fn(usize, usize) -> f32 + '_ {
        move |i, j| {
            let dx = points[i][0] - points[j][0];
            let dy = points[i][1] - points[j][1];
            (dx * dx + dy * dy).sqrt() as f32
        }
    }

    fn assert_reconstructs(points: &[[f64; 2]], tolerance: f64) {
        let embedded = mds_embed(points.len(), euclidean(points));
        for i in 0..points.len() {
            for j in 0..points.len() {
                let want = {
                    let dx = points[i][0] - points[j][0];
                    let dy = points[i][1] - points[j][1];
                    (dx * dx + dy * dy).sqrt()
                };
                let got = {
                    let dx = f64::from(embedded[i][0]) - f64::from(embedded[j][0]);
                    let dy = f64::from(embedded[i][1]) - f64::from(embedded[j][1]);
                    (dx * dx + dy * dy).sqrt()
                };
                assert!(
                    (got - want).abs() <= tolerance,
                    "distance {i}->{j}: got {got}, want {want}"
                );
            }
        }
    }

    #[test]
    fn planar_square_is_reconstructed() {
        let points = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        assert_reconstructs(&points, 1e-3);
    }

    #[test]
    fn isotropic_shape_keeps_two_dimensions() {
        let points: Vec<[f64; 2]> = (0..6)
            .map(|i| {
                let angle = 2.0 * PI * i as f64 / 6.0;
                [angle.cos(), angle.sin()]
            })
            .collect();
        let embedded = mds_embed(points.len(), euclidean(&points));
        let y_min = embedded.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
        let y_max = embedded
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            y_max - y_min > 0.1,
            "isotropic shape collapsed to one dimension: {embedded:?}"
        );
        assert_reconstructs(&points, 1e-3);
    }

    #[test]
    fn collinear_points_have_zero_second_axis() {
        let points = [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]];
        let embedded = mds_embed(points.len(), euclidean(&points));
        for point in &embedded {
            assert!(point[1].abs() < 1e-4, "collinear point has y={}", point[1]);
        }
        assert_reconstructs(&points, 1e-3);
    }

    #[test]
    fn high_aspect_ratio_is_reconstructed() {
        let points = [[0.0, 0.0], [20.0, 0.0], [20.0, 1.0], [0.0, 1.0]];
        assert_reconstructs(&points, 1e-2);
    }

    #[test]
    fn two_cities_use_the_distance_along_x() {
        let embedded = mds_embed(2, |i, j| if i == j { 0.0 } else { 7.0 });
        assert_eq!(embedded[0], [0.0, 0.0]);
        assert!((embedded[1][0] - 7.0).abs() < 1e-6);
        assert_eq!(embedded[1][1], 0.0);
    }

    #[test]
    fn fewer_than_two_cities_are_degenerate() {
        assert!(mds_embed(0, |_, _| 0.0).is_empty());
        assert_eq!(mds_embed(1, |_, _| 0.0), vec![[0.0, 0.0]]);
    }

    #[test]
    fn all_equal_distances_are_finite_and_deterministic() {
        let distance = |i, j| if i == j { 0.0 } else { 5.0 };
        let first = mds_embed(4, distance);
        let second = mds_embed(4, distance);
        assert_eq!(first, second);
        assert!(
            first
                .iter()
                .all(|point| point[0].is_finite() && point[1].is_finite())
        );
    }

    #[test]
    fn non_euclidean_matrix_is_finite_and_deterministic() {
        // ring6_explicit: nearest neighbours 10, everyone else 100. Not realizable in 2D, so
        // the layout is a sketch; the contract is finite, deterministic coordinates.
        let matrix = [
            [0.0, 10.0, 100.0, 100.0, 100.0, 10.0],
            [10.0, 0.0, 10.0, 100.0, 100.0, 100.0],
            [100.0, 10.0, 0.0, 10.0, 100.0, 100.0],
            [100.0, 100.0, 10.0, 0.0, 10.0, 100.0],
            [100.0, 100.0, 100.0, 10.0, 0.0, 10.0],
            [10.0, 100.0, 100.0, 100.0, 10.0, 0.0],
        ];
        let distance = |i: usize, j: usize| matrix[i][j];
        let first = mds_embed(6, distance);
        let second = mds_embed(6, distance);
        assert_eq!(first, second);
        assert!(
            first
                .iter()
                .all(|point| point[0].is_finite() && point[1].is_finite())
        );
    }

    #[test]
    fn non_finite_distances_do_not_poison_the_layout() {
        let embedded = mds_embed(3, |i, j| if (i, j) == (0, 1) { f32::NAN } else { 1.0 });
        assert!(
            embedded
                .iter()
                .all(|point| point[0].is_finite() && point[1].is_finite())
        );
    }
}

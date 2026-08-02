/// Asserts that two `f32` values are approximately equal using a hybrid
/// absolute + relative tolerance.
///
/// The tolerance scales with the magnitude of the inputs (`1e-5` floor, plus
/// `1e-5 × max(|expected|, |actual|)`), so it stays valid across the range of
/// distances and tour costs the test suite exercises (from `0.0` up to the
/// thousands for large TSPLIB instances). The previous implementation compared
/// against a bare `f32::EPSILON` (~`1.19e-7`), which is the ULP at `1.0` and
/// becomes far too tight for any value larger than ~`16`.
pub fn assert_approx(expected_val: f32, actual_val: f32) {
    let abs_diff = (expected_val - actual_val).abs();
    let magnitude = expected_val.abs().max(actual_val.abs());
    let tol = 1e-5_f32.max(magnitude * 1e-5);
    assert!(
        abs_diff <= tol,
        "assert_approx: expected {expected_val}, got {actual_val} \
         (diff {abs_diff} > tol {tol})"
    );
}

#[cfg(test)]
mod tests {
    use super::assert_approx;

    #[test]
    fn assert_approx_equal_values_pass() {
        assert_approx(1.0, 1.0);
        assert_approx(0.0, 0.0);
        assert_approx(-2.5, -2.5);
    }

    #[test]
    fn assert_approx_near_values_pass() {
        // sqrt(8) vs its decimal literal — the original tight case.
        assert_approx(2.828_427, 2.0_f32.mul_add(2.0, 4.0).sqrt());
        // Small absolute difference within the 1e-5 floor.
        assert_approx(1.0, 1.000_005);
        assert_approx(0.0, 1e-6);
    }

    #[test]
    fn assert_approx_large_magnitude_uses_relative_tolerance() {
        // At magnitude ~5000 the absolute floor would be far too tight;
        // the relative branch allows ~0.05 of slack.
        assert_approx(5_000.0, 5_000.03);
        assert_approx(12_345.0, 12_345.1);
    }

    #[test]
    #[should_panic(expected = "assert_approx")]
    fn assert_approx_far_values_fail() {
        assert_approx(1.0, 1.1);
    }

    #[test]
    #[should_panic(expected = "assert_approx")]
    fn assert_approx_large_magnitude_far_values_fail() {
        // 0.5% difference is well outside the 0.001% relative tolerance.
        assert_approx(10_000.0, 10_050.0);
    }
}

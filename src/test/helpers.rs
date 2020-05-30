pub fn assert_approx(expected_val: f32, actual_val: f32) {
    assert!(
        (expected_val - actual_val).abs() < f32::EPSILON,
        "res was: {}",
        actual_val
    );
}

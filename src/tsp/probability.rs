use rand::Rng;

pub const BETA: f64 = 1.5;
// Mantegna (1994) σ_u for β=1.5: (Γ(1+β)·sin(πβ/2)/(Γ((1+β)/2)·β·2^((β-1)/2)))^(1/β)
pub const SIGMA_U: f64 = 0.6966;

/// Box-Muller normal sample; guards ln(0) with MIN_POSITIVE floor.
pub fn normal_sample(rng: &mut impl Rng) -> f64 {
    let u1: f64 = rng.random::<f64>().max(f64::MIN_POSITIVE);
    let u2: f64 = rng.random();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

/// Mantegna's Lévy-stable step (β=1.5). Resamples v to avoid ÷0 → inf.
pub fn levy_step(rng: &mut impl Rng) -> f64 {
    let u = normal_sample(rng) * SIGMA_U;
    let mut v = normal_sample(rng);
    while v.abs() < f64::MIN_POSITIVE {
        v = normal_sample(rng);
    }
    u / v.abs().powf(1.0 / BETA)
}

/// Boltzmann acceptance: exp(-(e2-e1)/t). Caller handles the e2<e1 case.
pub fn metropolis(t: f32, e1: f32, e2: f32) -> f32 {
    (-(e2 - e1) / t).exp()
}

/// Geometric cooling: temperature × (1 - cooling_rate).
pub fn cooling(temperature: f32, cooling_rate: f32) -> f32 {
    temperature - cooling_rate * temperature
}

/// Bernoulli trial — returns true with probability p. Constructs its own RNG; for hot loops use `bernoulli`.
pub fn probability(p: f32) -> bool {
    let mut rng = rand::rng();
    p > rng.random::<f32>()
}

/// Bernoulli trial using a caller-supplied RNG — zero allocation, suitable for inner loops.
pub fn bernoulli(rng: &mut impl Rng, p: f64) -> bool {
    rng.random::<f64>() < p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_sample_is_finite() {
        let mut rng = rand::rng();
        for _ in 0..100 {
            let s = normal_sample(&mut rng);
            assert!(s.is_finite(), "normal_sample returned non-finite: {s}");
        }
    }

    #[test]
    fn test_levy_step_is_finite() {
        let mut rng = rand::rng();
        for _ in 0..10_000 {
            let s = levy_step(&mut rng);
            assert!(s.is_finite(), "levy_step produced non-finite: {s}");
        }
    }

    #[test]
    fn test_metropolis_equal_energies_returns_one() {
        let result = metropolis(100.0, 10.0, 10.0);
        assert!((result - 1.0).abs() < 1e-5, "expected 1.0, got {result}");
    }

    #[test]
    fn test_metropolis_worsening_returns_in_0_1() {
        let result = metropolis(100.0, 10.0, 11.0);
        assert!(result > 0.0 && result < 1.0, "expected (0,1), got {result}");
    }

    #[test]
    fn test_cooling_reduces_temperature() {
        let t = cooling(100.0, 0.01);
        assert!(t < 100.0, "cooling should reduce temperature, got {t}");
        assert!((t - 99.0).abs() < 1e-4, "expected ~99.0, got {t}");
    }

    #[test]
    fn test_probability_always_false_at_zero() {
        for _ in 0..100 {
            assert!(!probability(0.0));
        }
    }

    #[test]
    fn test_probability_always_true_above_one() {
        for _ in 0..100 {
            assert!(probability(1.1));
        }
    }

    #[test]
    fn test_bernoulli_always_false_at_zero() {
        let mut rng = rand::rng();
        for _ in 0..100 {
            assert!(!bernoulli(&mut rng, 0.0));
        }
    }

    #[test]
    fn test_bernoulli_always_true_above_one() {
        let mut rng = rand::rng();
        for _ in 0..100 {
            assert!(bernoulli(&mut rng, 1.1));
        }
    }
}

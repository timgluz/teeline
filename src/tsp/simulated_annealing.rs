use rand::Rng;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::progress::{send_progress, ProgressMessage};
use super::route::Route;
use super::{Solution, SolverOptions};

pub fn solve(cities: &[KDPoint], distances: &DistanceMatrix, options: &SolverOptions) -> Solution {
    let cooling_rate = options.cooling_rate;
    let mut epoch = 0;

    tracing::info!(
        epochs = options.epochs,
        max_temp = options.max_temperature,
        cooling_rate = options.cooling_rate,
        "SA starting"
    );

    let mut best_route = Route::from_cities(cities);
    let mut best_distance = distances.tour_length(best_route.route());

    send_progress(ProgressMessage::PathUpdate(
        best_route.clone(),
        best_distance,
    ));

    let mut temperature = options.max_temperature;
    while epoch < options.epochs || temperature > options.min_temperature {
        let candidate = best_route.random_successor();
        let candidate_distance = distances.tour_length(candidate.route());

        if is_acceptable(temperature, best_distance, candidate_distance) {
            best_route = candidate;
            best_distance = candidate_distance;

            send_progress(ProgressMessage::PathUpdate(
                best_route.clone(),
                best_distance,
            ));
            tracing::info!(epoch, tour_length = best_distance, "SA: new best");
        }

        tracing::debug!(epoch, temperature, "SA: tick");
        temperature = cooling(temperature, cooling_rate);
        epoch += 1;
    }

    send_progress(ProgressMessage::Done);
    Solution::new(best_route.route(), cities, distances)
}

fn cooling(temperature: f32, cooling_rate: f32) -> f32 {
    temperature - cooling_rate * temperature
}

fn is_acceptable(temperature: f32, old_distance: f32, new_distance: f32) -> bool {
    if new_distance < old_distance {
        return true;
    }

    // if they are basically same - then false
    if (new_distance - old_distance).abs() < f32::EPSILON {
        return false;
    }

    let mut rng = rand::rng();
    let p: f32 = rng.random();
    let criteria = metropolis(temperature, old_distance, new_distance);

    p < criteria
}

// Boltzmann acceptance probability: exp(-(e2 - e1) / t)
// For worsening moves (e2 > e1): result is in (0, 1) and decreases as temperature drops.
// For improving moves (e2 < e1): result would be > 1, but is_acceptable handles those before
// calling metropolis.
fn metropolis(t: f32, e1: f32, e2: f32) -> f32 {
    (-(e2 - e1) / t).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    const APPROX_EPSILON: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < APPROX_EPSILON
    }

    // metropolis(t, e1, e2) = exp(-(e2-e1)/t)
    // For e2 > e1 (worsening): result is in (0,1)
    // For e2 == e1: result is exp(0) = 1.0
    // For e2 < e1 (improving): result is > 1.0 (handled by is_acceptable before reaching here)

    #[test]
    fn test_metropolis_equal_energies_returns_one() {
        // exp(-(10-10)/100) = exp(0) = 1.0
        let result = metropolis(100.0, 10.0, 10.0);
        assert!(approx_eq(1.0, result), "got {result}");
    }

    #[test]
    fn test_metropolis_worsening_move_at_high_temperature() {
        // exp(-(11-10)/100) = exp(-0.01) ≈ 0.99005
        let result = metropolis(100.0, 10.0, 11.0);
        assert!(result > 0.0 && result < 1.0, "expected (0,1), got {result}");
        assert!(approx_eq((-0.01_f32).exp(), result), "got {result}");
    }

    #[test]
    fn test_metropolis_worsening_move_at_low_temperature() {
        // exp(-(11-10)/0.1) = exp(-10) ≈ 0.0000454
        let result = metropolis(0.1, 10.0, 11.0);
        assert!(result > 0.0 && result < 1.0, "expected (0,1), got {result}");
        assert!(approx_eq((-10.0_f32).exp(), result), "got {result}");
    }

    #[test]
    fn test_metropolis_higher_temperature_gives_higher_acceptance() {
        // At higher temperature we accept worse solutions more readily
        let hot = metropolis(1000.0, 10.0, 15.0);
        let cold = metropolis(1.0, 10.0, 15.0);
        assert!(hot > cold, "hot={hot}, cold={cold}");
    }

    #[test]
    fn test_metropolis_larger_worsening_gives_lower_acceptance() {
        // A bigger step backward should be less likely to be accepted
        let small_step = metropolis(100.0, 10.0, 11.0);
        let large_step = metropolis(100.0, 10.0, 20.0);
        assert!(small_step > large_step, "small={small_step}, large={large_step}");
    }

    #[test]
    fn test_is_acceptable_always_accepts_improvement() {
        // lower distance = better, so new_distance < old_distance must always be accepted
        let temperature = 0.001; // near-zero — almost no random acceptance
        assert!(is_acceptable(temperature, 100.0, 50.0));
        assert!(is_acceptable(temperature, 100.0, 99.999));
    }

    #[test]
    fn test_is_acceptable_never_accepts_equal_distance() {
        // Equal energies: the function returns false (not an improvement, no chance roll)
        let temperature = 1_000_000.0; // very high — would normally accept anything
        assert!(!is_acceptable(temperature, 10.0, 10.0));
    }

    #[test]
    fn test_is_acceptable_probabilistic_for_worsening_at_high_temperature() {
        // At very high temperature the acceptance probability approaches 1.
        // Run many trials; almost all should be accepted.
        let temperature = 1_000_000.0;
        let accepted = (0..1000)
            .filter(|_| is_acceptable(temperature, 10.0, 10.001))
            .count();
        assert!(accepted > 900, "expected >90% acceptance at high T, got {accepted}/1000");
    }

    #[test]
    fn test_is_acceptable_rarely_accepts_worsening_at_low_temperature() {
        // At near-zero temperature, worsening moves should almost never be accepted.
        let temperature = 0.0001;
        let accepted = (0..1000)
            .filter(|_| is_acceptable(temperature, 10.0, 20.0))
            .count();
        assert!(accepted < 100, "expected <10% acceptance at low T, got {accepted}/1000");
    }
}

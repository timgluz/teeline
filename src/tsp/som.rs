use crate::tsp::budget::Budget;
use crate::tsp::progress::ProgressMessage;
use crate::tsp::route::Route;
use crate::tsp::{SOMOptions, Solution, TspProblem};
use rand::RngExt;
use std::sync::mpsc;

#[inline]
fn sq_dist(a: &[f64; 2], b: &[f64; 2]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}

pub fn solve(
    problem: &TspProblem,
    opts: &SOMOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let n = cities.len();

    if n < 2 {
        let tour: Vec<usize> = cities.iter().map(|c| c.id).collect();
        return Solution::new(&tour, problem);
    }

    // Normalize city coordinates to [0,1]² for stable training
    let (min_x, max_x, min_y, max_y) = cities.iter().fold(
        (f64::MAX, f64::MIN, f64::MAX, f64::MIN),
        |(mnx, mxx, mny, mxy), c| {
            let x = c.coords[0] as f64;
            let y = c.coords[1] as f64;
            (mnx.min(x), mxx.max(x), mny.min(y), mxy.max(y))
        },
    );
    let span_x = (max_x - min_x).max(1e-9);
    let span_y = (max_y - min_y).max(1e-9);

    let norm_cities: Vec<[f64; 2]> = cities
        .iter()
        .map(|c| {
            [
                (c.coords[0] as f64 - min_x) / span_x,
                (c.coords[1] as f64 - min_y) / span_y,
            ]
        })
        .collect();

    // Centroid in normalized space
    let cx: f64 = norm_cities.iter().map(|c| c[0]).sum::<f64>() / n as f64;
    let cy: f64 = norm_cities.iter().map(|c| c[1]).sum::<f64>() / n as f64;

    // Initialize neurons in a small circle (radius 0.1) around centroid
    let num_neurons = n * opts.neuron_multiplier;
    let mut neurons: Vec<[f64; 2]> = (0..num_neurons)
        .map(|i| {
            let theta = 2.0 * std::f64::consts::PI * i as f64 / num_neurons as f64;
            [cx + 0.1 * theta.cos(), cy + 0.1 * theta.sin()]
        })
        .collect();

    let sigma_floor = 1.0_f64;
    let epochs = opts.epochs;
    let eta0 = opts.learning_rate;
    let sigma0 = opts.radius_fraction * num_neurons as f64;
    let checkpoint = (epochs / 10).max(1);

    tracing::info!(
        epochs,
        neurons = num_neurons,
        learning_rate = eta0,
        radius_fraction = opts.radius_fraction,
        "SOM starting"
    );

    let mut rng = rand::rng();

    // SOM has no incumbent tour to watch — the map is only decoded at the end — so "progress" is the
    // quantisation error: how far the cities still sit from their best-matching neurons. That is the
    // standard convergence measure for a SOM, and it falls as the map fits the cities.
    //
    // Measuring it costs O(n * neurons), about n times an ordinary epoch, so it is sampled: the
    // interval widens with n so the total sampling cost stays a modest fraction of training rather
    // than overtaking it on large instances, and sampling is skipped entirely when the stop is off.
    // The first and last epochs are always sampled so a short run still yields a comparison.
    let tracking = opts.stagnation_epochs > 0;
    let measure_interval = if tracking {
        // Widen with n so the sampled work stays a fraction of training (a sample costs about n times
        // an epoch). Capped so there are always several samples: an interval approaching `epochs`
        // would leave only the forced first and last samples, making the stop unable to fire early.
        let for_cost = (n * opts.neuron_multiplier / 8).max(1) * n / 10;
        let interval = (epochs / 1000).max(for_cost).max(1);
        let interval = interval.min((epochs / 8).max(1));
        if interval >= epochs / 2 {
            tracing::warn!(
                measure_interval = interval,
                epochs,
                "SOM: the sampling interval is too coarse for the plateau stop to fire before the \
                 run ends; increase --epochs or lower --neuron-multiplier"
            );
        }
        interval
    } else {
        1
    };
    // Staleness is counted in *samples*, not epochs: a limit expressed in epochs would fire on a
    // single noisy sample whenever it fell below the sampling interval, and the signal is noisy by
    // construction (one random city trains each epoch). Rounding up means a limit below one interval
    // still requires one full interval of evidence.
    let sample_limit = if tracking {
        opts.stagnation_epochs.div_ceil(measure_interval).max(1)
    } else {
        0
    };
    let mut budget = Budget::new(epochs, sample_limit);

    let mut best_error = f64::INFINITY;

    // Training loop
    let mut finished = false;
    for t in 1..=epochs {
        let t_f = t as f64;
        let eta = eta0 * (-t_f / epochs as f64).exp();
        let sigma = (sigma0 * (-t_f / epochs as f64).exp()).max(sigma_floor);
        let two_sigma_sq = 2.0 * sigma * sigma;

        // Pick a random city (with replacement)
        let city_idx = rng.random_range(0..n);
        let city = norm_cities[city_idx];

        // Find BMU: neuron closest to the city (tie-break: lowest index)
        let bmu = neurons
            .iter()
            .enumerate()
            .min_by(|&(_, a), &(_, b)| {
                sq_dist(a, &city)
                    .partial_cmp(&sq_dist(b, &city))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .expect("neurons is always non-empty: n >= 2 and neuron_multiplier >= 1");

        // Update neurons within neighborhood; skip those with negligible influence
        let bmu_i = bmu as isize;
        let n_neurons_i = num_neurons as isize;
        for (i, neuron) in neurons.iter_mut().enumerate() {
            let d_ring = {
                let d = (i as isize - bmu_i).abs();
                d.min(n_neurons_i - d) as f64
            };
            let h = (-d_ring * d_ring / two_sigma_sq).exp();
            if h < 1e-3 {
                continue;
            }
            neuron[0] += eta * h * (city[0] - neuron[0]);
            neuron[1] += eta * h * (city[1] - neuron[1]);
        }

        // Send progress at each 10% milestone
        if t % checkpoint == 0 {
            tracing::debug!(
                epoch = t,
                pct = t * 100 / epochs,
                eta,
                sigma,
                "SOM: checkpoint"
            );
            if let Some(tx) = progress_tx {
                let snapshot = extract_tour(&norm_cities, &neurons, cities);
                let cost = problem.distances.tour_length(&snapshot);
                let _ = tx.send(ProgressMessage::EpochUpdate(t));
                let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&snapshot), cost));
            }
        }

        if tracking && (t == 1 || t == epochs || t % measure_interval == 0) {
            let error = quantisation_error(&norm_cities, &neurons);
            let still_improving = error < best_error;
            best_error = best_error.min(error);
            // Only sampled epochs consult the budget, and the budget counts samples, so a limit below
            // the sampling interval still needs a full interval of evidence before it can fire.
            if !budget.record(still_improving) {
                if budget.converged() {
                    tracing::info!(
                        samples = budget.epoch(),
                        stagnation_samples = budget.stale_epochs(),
                        "SOM: converged, quantisation error stopped improving"
                    );
                }
                // The loop can stop between checkpoints, so publish the map as it stands rather than
                // leaving the last reported state up to 10% of the run out of date.
                // The checkpoint block above already published this epoch when it lands on one, so
                // only publish here otherwise.
                if let Some(tx) = progress_tx.filter(|_| t % checkpoint != 0) {
                    let snapshot = extract_tour(&norm_cities, &neurons, cities);
                    let cost = problem.distances.tour_length(&snapshot);
                    let _ = tx.send(ProgressMessage::EpochUpdate(t));
                    let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&snapshot), cost));
                }
                finished = true;
                break;
            }
        }
        // Unmeasured epochs inherit the last measured outcome. Reporting progress instead would reset
        // the streak every `measure_interval` epochs, so a limit above 1 could never be reached — the
        // plateau stop would silently never fire, which is worse than not sampling at all.
    }

    let tour = extract_tour(&norm_cities, &neurons, cities);
    let final_cost = problem.distances.tour_length(&tour);

    tracing::info!(tour_length = final_cost, "SOM done");

    if let Some(tx) = progress_tx {
        // The break path already published the final state, so sending again would duplicate it.
        // Only send a final PathUpdate if the last checkpoint didn't already cover it
        if !finished && !epochs.is_multiple_of(checkpoint) {
            let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&tour), final_cost));
        }
        let _ = tx.send(ProgressMessage::Done);
    }

    Solution::new(&tour, problem)
}

/// Mean distance from each city to its closest neuron — the standard SOM fit measure.
fn quantisation_error(norm_cities: &[[f64; 2]], neurons: &[[f64; 2]]) -> f64 {
    if norm_cities.is_empty() {
        return 0.0;
    }
    let total: f64 = norm_cities
        .iter()
        .map(|city| {
            neurons
                .iter()
                .map(|neuron| sq_dist(neuron, city))
                .fold(f64::INFINITY, f64::min)
                .sqrt()
        })
        .sum();
    total / norm_cities.len() as f64
}

/// Extract a tour from current neuron state.
/// Assigns each city to its closest neuron, sorts by ring index.
/// Collision tie-break: closer city wins; city array index as final tie-breaker.
fn extract_tour(
    norm_cities: &[[f64; 2]],
    neurons: &[[f64; 2]],
    cities: &[crate::tsp::kdtree::KDPoint],
) -> Vec<usize> {
    let n = norm_cities.len();
    let city_bmu: Vec<(usize, f64)> = norm_cities
        .iter()
        .map(|city| {
            neurons
                .iter()
                .enumerate()
                .map(|(ni, neuron)| (ni, sq_dist(neuron, city)))
                .min_by(|(_, da), (_, db)| da.partial_cmp(db).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or((0, f64::MAX))
        })
        .collect();

    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        let (bmu_a, dist_a) = city_bmu[a];
        let (bmu_b, dist_b) = city_bmu[b];
        bmu_a
            .cmp(&bmu_b)
            .then_with(|| {
                dist_a
                    .partial_cmp(&dist_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.cmp(&b))
    });

    order.iter().map(|&ci| cities[ci].id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{TspProblem, distance_matrix, kdtree::KDPoint};
    use std::f32::consts::PI;

    fn circle_cities(n: usize) -> Vec<KDPoint> {
        (0..n)
            .map(|i| KDPoint {
                id: i,
                coords: [
                    (2.0 * PI * i as f32 / n as f32).cos(),
                    (2.0 * PI * i as f32 / n as f32).sin(),
                ],
            })
            .collect()
    }

    fn make_problem(cities: Vec<KDPoint>) -> TspProblem {
        let dm = distance_matrix::from_cities(&cities);
        TspProblem::new(cities, dm)
    }

    fn is_valid_tour(route: &[usize], cities: &[KDPoint]) -> bool {
        let mut expected: Vec<usize> = cities.iter().map(|c| c.id).collect();
        expected.sort_unstable();
        let mut got = route.to_vec();
        got.sort_unstable();
        got == expected
    }

    fn fast_opts() -> SOMOptions {
        SOMOptions {
            epochs: 500,
            ..SOMOptions::default()
        }
    }

    #[test]
    fn test_som_valid_permutation_tiny() {
        let cities = circle_cities(3);
        let problem = make_problem(cities.clone());
        let sol = solve(&problem, &fast_opts(), None, None);
        assert_eq!(sol.route().len(), 3, "tour must visit all 3 cities");
        assert!(
            is_valid_tour(sol.route(), &cities),
            "tour must be a valid permutation"
        );
        assert!(sol.total > 0.0, "tour distance must be positive");
    }

    #[test]
    fn test_som_valid_permutation_circle() {
        let cities = circle_cities(10);
        let problem = make_problem(cities.clone());
        let sol = solve(&problem, &fast_opts(), None, None);
        assert_eq!(sol.route().len(), 10, "tour must visit all 10 cities");
        assert!(
            is_valid_tour(sol.route(), &cities),
            "tour must be a valid permutation"
        );
    }

    #[test]
    fn test_som_non_contiguous_city_ids() {
        let cities: Vec<KDPoint> = vec![5usize, 10, 15, 20, 25]
            .into_iter()
            .enumerate()
            .map(|(i, id)| KDPoint {
                id,
                coords: [
                    (2.0 * PI * i as f32 / 5.0).cos(),
                    (2.0 * PI * i as f32 / 5.0).sin(),
                ],
            })
            .collect();
        let problem = make_problem(cities.clone());
        let sol = solve(&problem, &fast_opts(), None, None);
        let mut got = sol.route().to_vec();
        got.sort_unstable();
        assert_eq!(
            got,
            vec![5, 10, 15, 20, 25],
            "tour must contain original city IDs, not array positions"
        );
    }

    #[test]
    fn test_som_two_cities() {
        let cities = vec![
            KDPoint {
                id: 0,
                coords: [0.0, 0.0],
            },
            KDPoint {
                id: 1,
                coords: [1.0, 0.0],
            },
        ];
        let problem = make_problem(cities.clone());
        let sol = solve(&problem, &fast_opts(), None, None);
        assert_eq!(sol.route().len(), 2);
        assert!(is_valid_tour(sol.route(), &cities));
    }

    #[test]
    fn test_som_finite_positive_tour_cost() {
        let cities = circle_cities(8);
        let problem = make_problem(cities.clone());
        let sol = solve(&problem, &fast_opts(), None, None);
        assert!(sol.total.is_finite(), "tour distance must be finite");
        assert!(sol.total > 0.0, "tour distance must be positive");
    }
}

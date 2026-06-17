use crate::tsp::{SOMOptions, Solution, TspProblem};
use crate::tsp::progress::ProgressMessage;
use crate::tsp::route::Route;
use std::sync::mpsc;
use rand::RngExt;

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

    // Training loop
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
                sq_dist(a, &city).partial_cmp(&sq_dist(b, &city)).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

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
            tracing::debug!(epoch = t, pct = t * 100 / epochs, eta, sigma, "SOM: checkpoint");
            if let Some(tx) = progress_tx {
                let snapshot = extract_tour(&norm_cities, &neurons, cities);
                let cost = problem.distances.tour_length(&snapshot);
                let _ = tx.send(ProgressMessage::EpochUpdate(t));
                let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&snapshot), cost));
            }
        }
    }

    let tour = extract_tour(&norm_cities, &neurons, cities);
    let final_cost = problem.distances.tour_length(&tour);

    tracing::info!(tour_length = final_cost, "SOM done");

    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&tour), final_cost));
        let _ = tx.send(ProgressMessage::Done);
    }

    Solution::new(&tour, problem)
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
            .then_with(|| dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.cmp(&b))
    });

    order.iter().map(|&ci| cities[ci].id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;
    use crate::tsp::{distance_matrix, kdtree::KDPoint, TspProblem};

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
        assert!(is_valid_tour(sol.route(), &cities), "tour must be a valid permutation");
        assert!(sol.total > 0.0, "tour distance must be positive");
    }

    #[test]
    fn test_som_valid_permutation_circle() {
        let cities = circle_cities(10);
        let problem = make_problem(cities.clone());
        let sol = solve(&problem, &fast_opts(), None, None);
        assert_eq!(sol.route().len(), 10, "tour must visit all 10 cities");
        assert!(is_valid_tour(sol.route(), &cities), "tour must be a valid permutation");
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
            KDPoint { id: 0, coords: [0.0, 0.0] },
            KDPoint { id: 1, coords: [1.0, 0.0] },
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

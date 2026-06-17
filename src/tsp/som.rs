use crate::tsp::{SOMOptions, Solution, TspProblem};
use crate::tsp::progress::ProgressMessage;
use std::sync::mpsc;
use rand::RngExt;

pub fn solve(
    problem: &TspProblem,
    opts: &SOMOptions,
    _progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
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
                let da = (a[0] - city[0]).powi(2) + (a[1] - city[1]).powi(2);
                let db = (b[0] - city[0]).powi(2) + (b[1] - city[1]).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
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
    }

    // Tour extraction: assign each city to its closest neuron (BMU), sort by ring index
    // Collision tie-break: closer city wins; city index as final tie-breaker
    let city_bmu: Vec<(usize, usize, f64)> = norm_cities
        .iter()
        .enumerate()
        .map(|(city_idx, city)| {
            let (bmu_idx, dist) = neurons
                .iter()
                .enumerate()
                .map(|(ni, n)| {
                    let d = (n[0] - city[0]).powi(2) + (n[1] - city[1]).powi(2);
                    (ni, d)
                })
                .min_by(|(_, da), (_, db)| da.partial_cmp(db).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(ni, d)| (ni, d))
                .unwrap_or((0, f64::MAX));
            (city_idx, bmu_idx, dist)
        })
        .collect();

    // Sort by (bmu_ring_index, dist_to_neuron, city_array_idx) for stable collision handling
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        let (_, bmu_a, dist_a) = city_bmu[a];
        let (_, bmu_b, dist_b) = city_bmu[b];
        bmu_a
            .cmp(&bmu_b)
            .then_with(|| dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.cmp(&b))
    });

    let tour: Vec<usize> = order.iter().map(|&ci| cities[ci].id).collect();
    Solution::new(&tour, problem)
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

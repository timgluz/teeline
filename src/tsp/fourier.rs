use crate::tsp::progress::ProgressMessage;
use crate::tsp::{FourierOptions, Solution, TspProblem, kdtree::KDPoint};
use num_complex::Complex;
use std::f64::consts::PI;
use std::sync::mpsc;

pub fn solve(
    problem: &TspProblem,
    opts: &FourierOptions,
    _progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let n = cities.len();
    if n < 2 {
        let tour: Vec<usize> = cities.iter().map(|c| c.id).collect();
        return Solution::new(&tour, problem);
    }

    let cities_cx: Vec<Complex<f64>> = cities
        .iter()
        .map(|c| Complex::new(c.coords[0] as f64, c.coords[1] as f64))
        .collect();

    let centroid: Complex<f64> = cities_cx.iter().sum::<Complex<f64>>() / n as f64;
    let radius = cities_cx.iter().map(|z| (z - centroid).norm()).sum::<f64>() / n as f64;

    let ks = ks_array(opts.k_max);
    let mut c = init_coefficients(centroid, radius, opts.k_max);

    let basis = compute_basis(&ks, opts.m);
    let mut lambda = opts.lambda;

    for k_active in 1..=opts.k_max {
        for _ in 0..opts.epochs {
            gradient_step(&mut c, &ks, &basis, &cities_cx, lambda, opts.lr, k_active);
        }
        lambda *= opts.lambda_decay;
    }

    let gamma = eval_curve(&c, &ks, opts.m);
    let tour = decode_tour(&gamma, cities);
    Solution::new(&tour, problem)
}

fn ks_array(k_max: usize) -> Vec<i64> {
    (-(k_max as i64)..=(k_max as i64)).collect()
}

fn init_coefficients(centroid: Complex<f64>, radius: f64, k_max: usize) -> Vec<Complex<f64>> {
    let mut c = vec![Complex::new(0.0, 0.0); 2 * k_max + 1];
    c[k_max] = centroid;
    c[k_max + 1] = Complex::new(radius * 0.5, 0.0);
    c
}

fn eval_curve(c: &[Complex<f64>], ks: &[i64], m: usize) -> Vec<Complex<f64>> {
    (0..m)
        .map(|j| {
            let s = j as f64 / m as f64;
            c.iter()
                .zip(ks.iter())
                .map(|(ck, &k)| ck * (Complex::new(0.0, 2.0 * PI * k as f64 * s)).exp())
                .sum()
        })
        .collect()
}

fn nearest_sample(gamma: &[Complex<f64>], city: Complex<f64>) -> usize {
    gamma
        .iter()
        .enumerate()
        .min_by(|&(_, a), &(_, b)| {
            (*a - city)
                .norm_sqr()
                .partial_cmp(&(*b - city).norm_sqr())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn decode_tour(gamma: &[Complex<f64>], cities: &[KDPoint]) -> Vec<usize> {
    let sample_indices: Vec<usize> = cities
        .iter()
        .map(|c| nearest_sample(gamma, Complex::new(c.coords[0] as f64, c.coords[1] as f64)))
        .collect();
    let mut order: Vec<usize> = (0..cities.len()).collect();
    order.sort_by_key(|&i| sample_indices[i]);
    order.iter().map(|&pos| cities[pos].id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    use crate::tsp::{TspProblem, distance_matrix};

    fn circle_cities(n: usize) -> Vec<KDPoint> {
        (0..n)
            .map(|i| KDPoint {
                id: i,
                coords: [
                    (2.0 * PI * i as f64 / n as f64).cos() as f32,
                    (2.0 * PI * i as f64 / n as f64).sin() as f32,
                ],
            })
            .collect()
    }

    fn fast_opts() -> FourierOptions {
        FourierOptions {
            k_max: 2,
            m: 50,
            epochs: 20,
            ..FourierOptions::default()
        }
    }

    #[test]
    fn test_fourier_decode_5city_circle() {
        let cities = circle_cities(5);
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities.clone(), dm);
        let sol = solve(&problem, &fast_opts(), None, None);
        assert_eq!(sol.route().len(), 5, "tour must visit all 5 cities");
        let mut sorted = sol.route().to_vec();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            vec![0, 1, 2, 3, 4],
            "tour must contain all city IDs exactly once"
        );
    }

    #[test]
    fn test_fourier_decode_non_contiguous_ids() {
        let cities: Vec<KDPoint> = vec![5, 10, 15, 20, 25]
            .into_iter()
            .enumerate()
            .map(|(i, id)| KDPoint {
                id,
                coords: [
                    (2.0 * PI * i as f64 / 5.0).cos() as f32,
                    (2.0 * PI * i as f64 / 5.0).sin() as f32,
                ],
            })
            .collect();
        let dm = distance_matrix::from_cities(&cities);
        let problem = TspProblem::new(cities.clone(), dm);
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
    fn test_fourier_gradient_reduces_energy() {
        // 3 cities arranged around a circle; one gradient step should pull the curve toward them
        let cities: Vec<Complex<f64>> = (0..3)
            .map(|i| {
                Complex::new(
                    (2.0 * PI * i as f64 / 3.0).cos(),
                    (2.0 * PI * i as f64 / 3.0).sin(),
                )
            })
            .collect();
        let k_max = 1usize;
        let ks = ks_array(k_max);
        let m = 30;
        let lambda = 0.05f64;
        let lr = 0.05f64;

        let centroid: Complex<f64> = cities.iter().sum::<Complex<f64>>() / cities.len() as f64;
        let radius =
            cities.iter().map(|z| (z - centroid).norm()).sum::<f64>() / cities.len() as f64;
        let mut c = init_coefficients(centroid, radius, k_max);

        let energy_before = compute_energy(&c, &ks, m, &cities, lambda);

        // one gradient step with k_active = 1
        let basis = compute_basis(&ks, m);
        gradient_step(&mut c, &ks, &basis, &cities, lambda, lr, 1);

        let energy_after = compute_energy(&c, &ks, m, &cities, lambda);
        assert!(
            energy_after < energy_before,
            "energy must decrease after one gradient step: before={energy_before:.6} after={energy_after:.6}"
        );
    }

    #[test]
    fn test_fourier_eval_curve_circle_case() {
        // c[k_max] = centroid at (1,0), c[k_max+1] = radius 0.5
        // The curve should be a circle of radius 0.5 centered at (1,0)
        let k_max = 1usize;
        let ks = ks_array(k_max);
        let centroid = Complex::new(1.0, 0.0);
        let radius = 1.0f64;
        let c = init_coefficients(centroid, radius, k_max);
        let m = 100;
        let gamma = eval_curve(&c, &ks, m);
        let max_dist_from_centroid = gamma
            .iter()
            .map(|g| (g - centroid).norm())
            .fold(0.0f64, f64::max);
        // c[k_max+1] = radius*0.5, so the curve traces a circle of radius ~0.5
        assert!(
            (max_dist_from_centroid - 0.5).abs() < 0.05,
            "curve should have max radius ~0.5 from centroid, got {max_dist_from_centroid:.4}"
        );
    }

    fn compute_energy(
        c: &[Complex<f64>],
        ks: &[i64],
        m: usize,
        cities: &[Complex<f64>],
        lambda: f64,
    ) -> f64 {
        let gamma = eval_curve(c, ks, m);
        let attraction: f64 = cities
            .iter()
            .map(|&z| {
                gamma
                    .iter()
                    .map(|&g| (g - z).norm_sqr())
                    .fold(f64::MAX, f64::min)
            })
            .sum();
        let tension: f64 = c
            .iter()
            .zip(ks.iter())
            .map(|(ck, &k)| lambda * (2.0 * PI * k as f64).powi(2) * ck.norm_sqr())
            .sum();
        attraction + tension
    }
}

fn compute_basis(ks: &[i64], m: usize) -> Vec<Vec<Complex<f64>>> {
    ks.iter()
        .map(|&k| {
            (0..m)
                .map(|j| {
                    let s = j as f64 / m as f64;
                    (Complex::new(0.0, 2.0 * PI * k as f64 * s)).exp()
                })
                .collect()
        })
        .collect()
}

fn gradient_step(
    c: &mut [Complex<f64>],
    ks: &[i64],
    basis: &[Vec<Complex<f64>>],
    cities: &[Complex<f64>],
    lambda: f64,
    lr: f64,
    k_active: usize,
) {
    let m = basis.first().map_or(0, |b| b.len());
    let gamma = eval_curve(c, ks, m);
    let n = cities.len() as f64;
    let mut grad = vec![Complex::new(0.0, 0.0); c.len()];

    // attraction gradient
    for &city in cities {
        let j = nearest_sample(&gamma, city);
        let err = gamma[j] - city;
        for (ki, gk) in grad.iter_mut().enumerate() {
            *gk += err * basis[ki][j].conj();
        }
    }

    // tension gradient
    for (ki, gk) in grad.iter_mut().enumerate() {
        let k = ks[ki] as f64;
        *gk += lambda * (2.0 * PI * k).powi(2) * c[ki];
    }

    // update active modes
    for (ki, ck) in c.iter_mut().enumerate() {
        if (ks[ki].unsigned_abs() as usize) <= k_active {
            *ck -= (lr / n) * grad[ki];
        }
    }
}

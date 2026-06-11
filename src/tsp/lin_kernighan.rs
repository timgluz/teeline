use crate::tsp::{
    distance_matrix::DistanceMatrix,
    kdtree::KDPoint,
    Solution, LKOptions, TspProblem,
};
use std::sync::mpsc;
use crate::tsp::progress::ProgressMessage;

pub(crate) fn build_candidates(cities: &[KDPoint], dm: &DistanceMatrix, k: usize) -> Vec<Vec<usize>> {
    let n = cities.len();
    let k = k.min(n.saturating_sub(1));
    cities
        .iter()
        .map(|city| {
            let mut others: Vec<(usize, f32)> = cities
                .iter()
                .filter(|c| c.id != city.id)
                .map(|c| {
                    let dist = dm.distance_between(city.id, c.id).unwrap_or(f32::MAX);
                    (c.id, dist)
                })
                .collect();
            others.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            others.into_iter().take(k).map(|(id, _)| id).collect()
        })
        .collect()
}

pub fn solve(
    problem: &TspProblem,
    opts: &LKOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree::KDPoint};

    fn build_points(n: usize) -> Vec<KDPoint> {
        (0..n)
            .map(|i| KDPoint {
                id: i,
                coords: [i as f32, 0.0],
            })
            .collect()
    }

    #[test]
    fn build_candidates_returns_k_nearest_for_each_city() {
        let pts = build_points(6);
        let dm = distance_matrix::from_cities(&pts);
        let cands = build_candidates(&pts, &dm, 3);
        // each city should have 3 nearest neighbors (excluding itself)
        assert_eq!(cands.len(), 6);
        for (i, row) in cands.iter().enumerate() {
            assert_eq!(row.len(), 3, "city {i} must have 3 candidates");
            assert!(!row.contains(&i), "city {i} must not be its own candidate");
        }
    }

    #[test]
    fn build_candidates_neighbors_are_sorted_by_distance() {
        let pts = build_points(6);
        let dm = distance_matrix::from_cities(&pts);
        let cands = build_candidates(&pts, &dm, 4);
        for (i, row) in cands.iter().enumerate() {
            let dists: Vec<f32> = row
                .iter()
                .map(|&j| dm.distance_between(i, j).unwrap_or(f32::MAX))
                .collect();
            let mut sorted = dists.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            assert_eq!(dists, sorted, "city {i} candidates must be distance-sorted");
        }
    }

    #[test]
    fn build_candidates_k_clamps_to_n_minus_1() {
        let pts = build_points(4);
        let dm = distance_matrix::from_cities(&pts);
        // ask for 10 neighbors from a 4-city graph
        let cands = build_candidates(&pts, &dm, 10);
        for row in &cands {
            // max possible is n-1 = 3
            assert!(row.len() <= 3);
        }
    }
}

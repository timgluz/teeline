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

fn d(dm: &DistanceMatrix, a: usize, b: usize) -> f32 {
    dm.distance_between(a, b).unwrap_or(f32::MAX)
}

fn make_pos(tour: &[usize]) -> Vec<usize> {
    let max_id = *tour.iter().max().unwrap_or(&0);
    let mut pos = vec![0usize; max_id + 1];
    for (rank, &city) in tour.iter().enumerate() {
        pos[city] = rank;
    }
    pos
}

fn tour_distance(tour: &[usize], dm: &DistanceMatrix) -> f32 {
    let n = tour.len();
    (0..n).map(|i| d(dm, tour[i], tour[(i + 1) % n])).sum()
}

fn apply_2opt_at(tour: &mut Vec<usize>, pos: &mut Vec<usize>, i: usize, j: usize) {
    tour[i..=j].reverse();
    for rank in i..=j {
        pos[tour[rank]] = rank;
    }
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

    #[test]
    fn make_pos_round_trips_with_tour() {
        let tour = vec![3usize, 1, 4, 0, 2];
        let pos = make_pos(&tour);
        for (rank, &city) in tour.iter().enumerate() {
            assert_eq!(pos[city], rank, "pos[{city}] should be {rank}");
        }
    }

    #[test]
    fn tour_distance_sums_consecutive_edges() {
        // 4 cities at (0,0),(1,0),(2,0),(3,0): each edge = 1.0
        let pts: Vec<KDPoint> = (0..4)
            .map(|i| KDPoint { id: i, coords: [i as f32, 0.0] })
            .collect();
        let dm = distance_matrix::from_cities(&pts);
        let tour = vec![0usize, 1, 2, 3];
        // edges: 0-1=1, 1-2=1, 2-3=1, 3-0=3 => total=6
        let dist = tour_distance(&tour, &dm);
        assert!((dist - 6.0).abs() < 1e-4, "expected 6.0, got {dist}");
    }

    #[test]
    fn apply_2opt_at_reverses_segment() {
        // tour: [0,1,2,3,4], 2-opt at (1,3) reverses [1..=3] => [0,3,2,1,4]
        let mut tour = vec![0usize, 1, 2, 3, 4];
        let mut pos = make_pos(&tour);
        apply_2opt_at(&mut tour, &mut pos, 1, 3);
        assert_eq!(tour, vec![0, 3, 2, 1, 4]);
        // pos must be consistent
        for (rank, &city) in tour.iter().enumerate() {
            assert_eq!(pos[city], rank);
        }
    }
}

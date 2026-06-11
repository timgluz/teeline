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

fn find_2opt_lk(
    tour: &[usize],
    pos: &[usize],
    candidates: &[Vec<usize>],
    dm: &DistanceMatrix,
) -> Option<(usize, usize)> {
    let n = tour.len();
    // Iterate over each consecutive (non-wrapping) forward edge (t1, t2).
    // Wrap-around closing edges are implicitly covered when scanning from
    // other positions, so skipping i+1==n avoids spurious wrap-around gains.
    for i in 0..n - 1 {
        let t1 = tour[i];
        let t2 = tour[i + 1];
        let g0 = d(dm, t1, t2);
        for &t3 in &candidates[t2] {
            if d(dm, t2, t3) >= g0 {
                break; // LK gain criterion: no further candidate can improve
            }
            let pos_t3 = pos[t3];
            if pos_t3 == i || pos_t3 == i + 1 {
                continue; // same edge — skip
            }
            let t4 = tour[(pos_t3 + 1) % n];
            let gain = g0 - d(dm, t2, t3) + d(dm, t3, t4) - d(dm, t4, t1);
            if gain > 1e-6 {
                // Reverse the segment between t2's position and t3.
                let (lo, hi) = if pos_t3 > i {
                    (i + 1, pos_t3)
                } else {
                    (pos_t3 + 1, i)
                };
                if lo < hi {
                    return Some((lo, hi));
                }
            }
        }
    }
    None
}

fn lk_pass(
    tour: &mut Vec<usize>,
    pos: &mut Vec<usize>,
    candidates: &[Vec<usize>],
    dm: &DistanceMatrix,
) -> bool {
    let mut improved = false;
    loop {
        match find_2opt_lk(tour, pos, candidates, dm) {
            Some((i, j)) => {
                apply_2opt_at(tour, pos, i, j);
                improved = true;
            }
            None => break,
        }
    }
    improved
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

    #[test]
    fn find_2opt_lk_improves_bad_tour() {
        // 4 cities in a square: optimal tour cost = 4.0, crossed tour cost = 2+2*sqrt(2)
        // city 0:(0,0), 1:(1,0), 2:(1,1), 3:(0,1)
        // bad tour: 0->1->3->2->0 (crosses)
        let pts: Vec<KDPoint> = vec![
            KDPoint { id: 0, coords: [0.0, 0.0] },
            KDPoint { id: 1, coords: [1.0, 0.0] },
            KDPoint { id: 2, coords: [1.0, 1.0] },
            KDPoint { id: 3, coords: [0.0, 1.0] },
        ];
        let dm = distance_matrix::from_cities(&pts);
        let candidates = build_candidates(&pts, &dm, 3);
        let mut tour = vec![0usize, 1, 3, 2];
        let mut pos = make_pos(&tour);
        let before = tour_distance(&tour, &dm);
        let found = find_2opt_lk(&tour, &pos, &candidates, &dm);
        assert!(found.is_some(), "must find an improvement in a crossed tour");
        let (i, j) = found.unwrap();
        apply_2opt_at(&mut tour, &mut pos, i, j);
        let after = tour_distance(&tour, &dm);
        assert!(after < before, "tour must improve: before={before} after={after}");
    }

    #[test]
    fn find_2opt_lk_returns_none_for_optimal_tour() {
        // straight line 0-1-2-3: already optimal for this topology
        let pts: Vec<KDPoint> = (0..4)
            .map(|i| KDPoint { id: i, coords: [i as f32, 0.0] })
            .collect();
        let dm = distance_matrix::from_cities(&pts);
        let candidates = build_candidates(&pts, &dm, 3);
        let tour = vec![0usize, 1, 2, 3];
        let pos = make_pos(&tour);
        let found = find_2opt_lk(&tour, &pos, &candidates, &dm);
        assert!(found.is_none(), "optimal linear tour must not improve");
    }

    #[test]
    fn lk_pass_improves_crossed_tour() {
        let pts: Vec<KDPoint> = vec![
            KDPoint { id: 0, coords: [0.0, 0.0] },
            KDPoint { id: 1, coords: [1.0, 0.0] },
            KDPoint { id: 2, coords: [1.0, 1.0] },
            KDPoint { id: 3, coords: [0.0, 1.0] },
        ];
        let dm = distance_matrix::from_cities(&pts);
        let cands = build_candidates(&pts, &dm, 3);
        let mut tour = vec![0usize, 1, 3, 2]; // crossed
        let mut pos = make_pos(&tour);
        let before = tour_distance(&tour, &dm);
        let improved = lk_pass(&mut tour, &mut pos, &cands, &dm);
        let after = tour_distance(&tour, &dm);
        assert!(improved, "lk_pass must return true when it improved");
        assert!(after < before, "tour must be shorter: before={before} after={after}");
    }

    #[test]
    fn lk_pass_returns_false_for_optimal_tour() {
        let pts: Vec<KDPoint> = (0..4)
            .map(|i| KDPoint { id: i, coords: [i as f32, 0.0] })
            .collect();
        let dm = distance_matrix::from_cities(&pts);
        let cands = build_candidates(&pts, &dm, 3);
        let mut tour = vec![0usize, 1, 2, 3];
        let mut pos = make_pos(&tour);
        let improved = lk_pass(&mut tour, &mut pos, &cands, &dm);
        assert!(!improved, "lk_pass must return false when tour is already optimal");
    }
}

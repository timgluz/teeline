use crate::tsp::{
    distance_matrix::{self, DistanceMatrix},
    kdtree::KDPoint,
    nearest_neighbor,
    route::Route,
    HeuristicOptions, Solution, LKOptions, TspProblem,
};
use std::sync::mpsc;
use crate::tsp::progress::ProgressMessage;
use rand::RngExt;

pub(crate) fn build_candidates(cities: &[KDPoint], dm: &DistanceMatrix, k: usize) -> Vec<Vec<usize>> {
    let n = cities.len();
    let k = k.min(n.saturating_sub(1));
    let max_id = cities.iter().map(|c| c.id).max().unwrap_or(0);
    let mut candidates = vec![Vec::new(); max_id + 1];
    for city in cities {
        let mut others: Vec<(usize, f32)> = cities
            .iter()
            .filter(|c| c.id != city.id)
            .map(|c| {
                let dist = dm.distance_between(city.id, c.id).unwrap_or(f32::MAX);
                (c.id, dist)
            })
            .collect();
        others.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates[city.id] = others.into_iter().take(k).map(|(id, _)| id).collect();
    }
    candidates
}

fn send_progress(tx: Option<&mpsc::Sender<ProgressMessage>>, path: &[usize], dist: f32) {
    if let Some(tx) = tx {
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(path), dist));
    }
}

pub fn solve(
    problem: &TspProblem,
    opts: &LKOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    let dm = distance_matrix::from_cities(&problem.cities);
    let k = opts.heuristic.n_nearest;
    let candidates = build_candidates(&problem.cities, &dm, k);

    // seed initial tour
    let mut best_tour: Vec<usize> = if let Some(t) = init_tour {
        t.to_vec()
    } else {
        let nn_opts = HeuristicOptions {
            epochs: 1,
            ..HeuristicOptions::default()
        };
        nearest_neighbor::solve(problem, &nn_opts, None, None).route().to_vec()
    };

    if best_tour.len() < 4 {
        return Solution::new(&best_tour, problem);
    }

    let mut init_pos = make_pos(&best_tour);
    lk_pass(&mut best_tour, &mut init_pos, &candidates, &dm);
    drop(init_pos);
    let mut best_dist = tour_distance(&best_tour, &dm);
    send_progress(progress_tx, &best_tour, best_dist);

    let mut rng = rand::rng();
    let mut platoo = 0usize;
    for _ in 0..opts.heuristic.epochs {
        let mut candidate = double_bridge(&best_tour, &mut rng);
        let mut cand_pos = make_pos(&candidate);
        lk_pass(&mut candidate, &mut cand_pos, &candidates, &dm);
        let dist = tour_distance(&candidate, &dm);
        if dist < best_dist {
            best_tour = candidate;
            best_dist = dist;
            platoo = 0;
            send_progress(progress_tx, &best_tour, best_dist);
        } else {
            platoo += 1;
            if platoo >= opts.heuristic.platoo_epochs {
                break;
            }
        }
    }

    Solution::new(&best_tour, problem)
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

fn apply_2opt_at(tour: &mut [usize], pos: &mut [usize], i: usize, j: usize) {
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
    if n < 4 { return None; }
    // Iterate over each consecutive (non-wrapping) forward edge (t1, t2).
    // Wrap-around closing edges are implicitly covered when scanning from
    // other positions, so skipping i+1==n avoids spurious wrap-around gains.
    for i in 0..n - 1 {
        let t1 = tour[i];
        let t2 = tour[i + 1];
        let g0 = d(dm, t1, t2);
        for &t3 in &candidates[t2] {
            // LK bound: candidates are sorted by distance from t2.
            // If d(t2,t3) >= g0, the added edge (t2,t3) alone would cost more
            // than the removed edge (t1,t2), so no gain is possible.
            if d(dm, t2, t3) >= g0 {
                break; // no candidate further in the sorted list can help
            }
            let pos_t3 = pos[t3];
            if pos_t3 == i || pos_t3 == i + 1 {
                continue; // same edge — skip
            }
            // Reversal of [lo..=hi] removes edges (t1,t2) and (t3,t4)
            // and adds edges (t1,t3) and (t2,t4).
            let (lo, hi) = if pos_t3 > i {
                (i + 1, pos_t3)
            } else {
                (pos_t3 + 1, i)
            };
            if lo >= hi {
                continue;
            }
            // t4 is the city immediately after t3 in the original tour direction.
            // This edge (t3,t4) is removed by the reversal.
            let t4 = tour[(pos_t3 + 1) % n];
            // Correct 2-opt gain: remove (t1,t2)+(t3,t4), add (t1,t3)+(t2,t4)
            let gain = g0 + d(dm, t3, t4) - d(dm, t1, t3) - d(dm, t2, t4);
            if gain > 1e-6 {
                return Some((lo, hi));
            }
        }
    }
    None
}

fn lk_pass(
    tour: &mut [usize],
    pos: &mut [usize],
    candidates: &[Vec<usize>],
    dm: &DistanceMatrix,
) -> bool {
    let mut improved = false;
    while let Some((i, j)) = find_2opt_lk(tour, pos, candidates, dm) {
        apply_2opt_at(tour, pos, i, j);
        improved = true;
    }
    improved
}

pub(crate) fn double_bridge(tour: &[usize], rng: &mut impl RngExt) -> Vec<usize> {
    let n = tour.len();
    if n < 8 {
        return tour.to_vec();
    }
    let p1 = 1 + rng.random_range(0..n / 4);
    let p2 = p1 + 1 + rng.random_range(0..n / 4);
    let p3 = p2 + 1 + rng.random_range(0..n / 4);
    let mut result = Vec::with_capacity(n);
    result.extend_from_slice(&tour[0..p1]);
    result.extend_from_slice(&tour[p2..p3]);
    result.extend_from_slice(&tour[p1..p2]);
    result.extend_from_slice(&tour[p3..n]);
    result
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

    #[test]
    fn find_2opt_lk_returns_none_for_tiny_tour() {
        let pts: Vec<KDPoint> = (0..3)
            .map(|i| KDPoint { id: i, coords: [i as f32, 0.0] })
            .collect();
        let dm = distance_matrix::from_cities(&pts);
        let cands = build_candidates(&pts, &dm, 2);
        let tour = vec![0usize, 1, 2];
        let pos = make_pos(&tour);
        assert!(
            find_2opt_lk(&tour, &pos, &cands, &dm).is_none(),
            "tiny tour (n=3) must return None without panicking"
        );
    }

    #[test]
    fn double_bridge_produces_valid_permutation() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let tour: Vec<usize> = (0..10).collect();
        let result = double_bridge(&tour, &mut rng);
        assert_eq!(result.len(), 10, "length must be preserved");
        let mut sorted = result.clone();
        sorted.sort();
        assert_eq!(sorted, tour, "same cities, different order");
    }

    #[test]
    fn double_bridge_differs_from_original() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let tour: Vec<usize> = (0..10).collect();
        let result = double_bridge(&tour, &mut rng);
        assert_ne!(result, tour, "result must differ from original");
    }

    #[test]
    fn double_bridge_returns_original_for_small_tour() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let tour: Vec<usize> = (0..7).collect();
        let result = double_bridge(&tour, &mut rng);
        assert_eq!(result, tour, "tours with < 8 cities must be returned unchanged");
    }

    #[test]
    fn solve_reduces_distance_on_berlin52() {
        use std::path::Path;
        let data = crate::tsp::tsplib::read_from_file(Path::new("tests/fixtures/berlin52.tsp"))
            .expect("berlin52.tsp must be readable");
        let cities = data.cities().to_vec();
        let dm = distance_matrix::from_cities(&cities);
        let problem = crate::tsp::TspProblem::new(cities, dm);
        let mut opts = LKOptions::default();
        opts.heuristic.epochs = 5; // fast test
        let sol = solve(&problem, &opts, None, None);
        // NN on berlin52 gives ~8980; any reasonable LK should beat NN
        assert!(sol.total < 9000.0, "LK should beat NN: got {}", sol.total);
        assert_eq!(sol.route().len(), 52);
    }
}

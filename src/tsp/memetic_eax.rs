//! Memetic Edge Assembly Crossover (EAX) for the symmetric TSP.
//!
//! Two parent tours are recombined by exchanging alternating A/B edge cycles: edges unique to
//! one parent are applied as a symmetric difference, then any subtours the exchange creates are
//! patched back into one tour. Offspring are polished with candidate-restricted 2-opt and a
//! parent is replaced only by its own strictly cheaper child, which keeps population niches.
//!
//! All internal work is in dense positions; city IDs appear only at the boundary.

use super::DistanceSource;
use super::Position;
use super::candidate_list::CandidateList;
use super::lin_kernighan::double_bridge;
use super::probability::bernoulli;
use super::progress::ProgressMessage;
use super::route::Route;
use super::{Solution, TspProblem, kdtree::KDPoint};
use rand::seq::SliceRandom;
use rand::{RngExt, SeedableRng};
use std::collections::HashSet;
use std::sync::mpsc;

/// Cost improvement below this is treated as zero; without it a float-noise swap would loop.
const EPS: f32 = 1e-6;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// E-set selection strategy for the crossover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EaxStrategy {
    /// Apply one randomly chosen AB cycle per child.
    SingleAb,
    /// Include each AB cycle independently with probability 1/2.
    Rand,
    /// Start like [`EaxStrategy::Rand`] and switch to `SingleAb` once the search stalls.
    #[default]
    Hybrid,
}

impl EaxStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            EaxStrategy::SingleAb => "single_ab",
            EaxStrategy::Rand => "rand",
            EaxStrategy::Hybrid => "hybrid",
        }
    }
}

impl std::str::FromStr for EaxStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "single_ab" | "single" => Ok(EaxStrategy::SingleAb),
            "rand" | "random" => Ok(EaxStrategy::Rand),
            "hybrid" => Ok(EaxStrategy::Hybrid),
            other => Err(format!(
                "unsupported EAX strategy '{other}' (expected single_ab, rand, or hybrid)"
            )),
        }
    }
}

/// Configuration for the memetic EAX solver.
///
/// Deliberately not embedded in [`super::HeuristicOptions`]: `candidate_k` and `stall_epochs`
/// have their own semantics here (`stall_epochs` both switches the hybrid strategy and bounds
/// the stop), and `pop_size`/`children_per_pair` have no shared home.
#[derive(Clone, Debug, PartialEq)]
pub struct MemeticEaxOptions {
    /// Maximum generations; `0` returns the best polished initial member.
    pub epochs: usize,
    pub pop_size: usize,
    pub children_per_pair: usize,
    pub strategy: EaxStrategy,
    /// Stagnant generations before `hybrid` switches to `single_ab`; the solver stops at twice
    /// this count.
    pub stall_epochs: usize,
    pub candidate_k: usize,
    /// Fixed RNG seed for reproducible runs; `None` uses system entropy.
    pub seed: Option<u64>,
}

impl Default for MemeticEaxOptions {
    fn default() -> Self {
        MemeticEaxOptions {
            epochs: 500,
            pop_size: 100,
            children_per_pair: 15,
            strategy: EaxStrategy::Hybrid,
            stall_epochs: 50,
            candidate_k: 20,
            seed: None,
        }
    }
}

impl MemeticEaxOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.pop_size == 0 {
            return Err("pop_size must be >= 1".to_string());
        }
        if self.children_per_pair == 0 {
            return Err("children_per_pair must be >= 1".to_string());
        }
        if self.candidate_k == 0 {
            return Err("candidate_k must be >= 1".to_string());
        }
        if self.stall_epochs == 0 {
            return Err("stall_epochs must be >= 1".to_string());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tour graph
// ---------------------------------------------------------------------------

/// A fixed-degree undirected tour: `nbr[city]` holds exactly two neighbours.
#[derive(Clone, Debug)]
struct TourGraph {
    nbr: Vec<[usize; 2]>,
}

impl TourGraph {
    fn from_tour(tour: &[usize]) -> Self {
        let n = tour.len();
        let mut nbr = vec![[usize::MAX; 2]; n];
        for i in 0..n {
            let a = tour[i];
            let b = tour[(i + 1) % n];
            Self::push_neighbour(&mut nbr[a], b);
            Self::push_neighbour(&mut nbr[b], a);
        }
        debug_assert!(
            nbr.iter().all(|s| s[0] != usize::MAX && s[1] != usize::MAX),
            "every city must have two neighbours"
        );
        TourGraph { nbr }
    }

    fn push_neighbour(slots: &mut [usize; 2], value: usize) {
        if slots[0] == usize::MAX {
            slots[0] = value;
        } else {
            debug_assert_eq!(slots[1], usize::MAX, "degree must stay at two");
            slots[1] = value;
        }
    }

    fn len(&self) -> usize {
        self.nbr.len()
    }

    fn has_edge(&self, a: usize, b: usize) -> bool {
        self.nbr[a][0] == b || self.nbr[a][1] == b
    }

    /// Replaces the neighbour `old` of `city` with `new`.
    fn replace_neighbour(&mut self, city: usize, old: usize, new: usize) {
        let slots = &mut self.nbr[city];
        if slots[0] == old {
            slots[0] = new;
        } else if slots[1] == old {
            slots[1] = new;
        } else {
            panic!("replace_neighbour: {old} is not a neighbour of {city}");
        }
    }

    fn to_tour(&self, start: usize) -> Vec<usize> {
        let n = self.nbr.len();
        let mut tour = Vec::with_capacity(n);
        let mut prev = usize::MAX;
        let mut current = start;
        for _ in 0..n {
            tour.push(current);
            let [a, b] = self.nbr[current];
            let next = if a != prev { a } else { b };
            prev = current;
            current = next;
        }
        debug_assert_eq!(current, start, "tour graph must be one cycle");
        tour
    }

    fn is_single_cycle(&self) -> bool {
        let n = self.nbr.len();
        if n == 0 {
            return true;
        }
        let mut seen = vec![false; n];
        let mut prev = usize::MAX;
        let mut current = 0;
        for _ in 0..n {
            if seen[current] {
                return false;
            }
            seen[current] = true;
            let [a, b] = self.nbr[current];
            let next = if a != prev { a } else { b };
            prev = current;
            current = next;
        }
        current == 0 && seen.into_iter().all(|s| s)
    }

    fn edge_sum(&self, distances: &dyn DistanceSource) -> f32 {
        let n = self.nbr.len();
        if n < 2 {
            return 0.0;
        }
        let tour = self.to_tour(0);
        (0..n)
            .map(|i| distance(distances, tour[i], tour[(i + 1) % n]))
            .sum()
    }
}

fn distance(distances: &dyn DistanceSource, a: usize, b: usize) -> f32 {
    distances
        .distance_by_pos(Position::new(a), Position::new(b))
        .unwrap_or(f32::MAX)
}

fn is_permutation(positions: &[usize], n: usize) -> bool {
    if positions.len() != n {
        return false;
    }
    let mut seen = vec![false; n];
    for &p in positions {
        if p >= n || seen[p] {
            return false;
        }
        seen[p] = true;
    }
    true
}

// ---------------------------------------------------------------------------
// AB cycles
// ---------------------------------------------------------------------------

/// One AB cycle in traversal order. Edge `(v[i], v[i + 1])` is an A edge for even `i` and a
/// B edge for odd `i`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct AbCycle {
    vertices: Vec<usize>,
}

fn other_origin(origin: bool) -> bool {
    !origin
}

/// Marks `edge` visited in the adjacency set of origin `origin` (`true` = A, `false` = B).
fn mark_edge(
    origin_is_a: bool,
    a_only: &[Vec<usize>],
    b_only: &[Vec<usize>],
    used_a: &mut [[bool; 2]],
    used_b: &mut [[bool; 2]],
    a: usize,
    b: usize,
) {
    let (adj, used) = if origin_is_a {
        (a_only, used_a)
    } else {
        (b_only, used_b)
    };
    if let Some(slot) = adj[a].iter().position(|&x| x == b) {
        used[a][slot] = true;
    }
    if let Some(slot) = adj[b].iter().position(|&x| x == a) {
        used[b][slot] = true;
    }
}

fn unvisited_neighbour(adj: &[Vec<usize>], used: &[[bool; 2]], city: usize) -> Option<usize> {
    adj[city]
        .iter()
        .enumerate()
        .find(|(slot, _)| !used[city][*slot])
        .map(|(_, &neighbour)| neighbour)
}

/// Decomposes the symmetric difference of two tours into alternating AB cycles.
///
/// Shared edges are absent from the difference graph, so they can never appear in a cycle.
fn ab_cycles(a: &TourGraph, b: &TourGraph) -> Result<Vec<AbCycle>, &'static str> {
    let n = a.len();
    if b.len() != n {
        return Err("parent graphs differ in size");
    }

    let mut a_only: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut b_only: Vec<Vec<usize>> = vec![Vec::new(); n];
    for city in 0..n {
        for &nbr in &a.nbr[city] {
            if !b.has_edge(city, nbr) {
                a_only[city].push(nbr);
            }
        }
        for &nbr in &b.nbr[city] {
            if !a.has_edge(city, nbr) {
                b_only[city].push(nbr);
            }
        }
        if a_only[city].len() != b_only[city].len() {
            return Err("parent tours disagree on edge parity");
        }
    }

    let mut used_a = vec![[false; 2]; n];
    let mut used_b = vec![[false; 2]; n];
    let mut cycles = Vec::new();

    for start in 0..n {
        while let Some(first) = unvisited_neighbour(&a_only, &used_a, start) {
            let mut vertices = vec![start];
            let mut current = start;
            let mut next = first;
            let mut origin_is_a = true; // start on an A edge

            loop {
                mark_edge(
                    origin_is_a,
                    &a_only,
                    &b_only,
                    &mut used_a,
                    &mut used_b,
                    current,
                    next,
                );
                if next == start {
                    break;
                }
                vertices.push(next);
                origin_is_a = other_origin(origin_is_a);
                let (adj, used) = if origin_is_a {
                    (&a_only, &used_a)
                } else {
                    (&b_only, &used_b)
                };
                let Some(following) = unvisited_neighbour(adj, used, next) else {
                    return Err("AB traversal reached a dead end");
                };
                current = next;
                next = following;
            }

            if vertices.len() < 4 || vertices.len() % 2 != 0 {
                return Err("AB cycle must be even and at least length 4");
            }
            cycles.push(AbCycle { vertices });
        }
    }

    Ok(cycles)
}

/// Applies a cycle to parent A: every A edge is replaced by the adjacent B edge. Each vertex
/// loses exactly one neighbour and gains exactly one, so degree two is preserved by construction.
fn apply_cycle(graph: &mut TourGraph, vertices: &[usize]) {
    let len = vertices.len();
    debug_assert!(
        len >= 4 && len.is_multiple_of(2),
        "invalid AB cycle length {len}"
    );
    for i in 0..len {
        let city = vertices[i];
        let old = if i.is_multiple_of(2) {
            vertices[(i + 1) % len]
        } else {
            vertices[(i + len - 1) % len]
        };
        let new = if i.is_multiple_of(2) {
            vertices[(i + len - 1) % len]
        } else {
            vertices[(i + 1) % len]
        };
        graph.replace_neighbour(city, old, new);
    }
}

// ---------------------------------------------------------------------------
// Components and patching
// ---------------------------------------------------------------------------

struct Scratch {
    queue: Vec<usize>,
    in_queue: Vec<bool>,
    added: Vec<(usize, usize)>,
    comp_of: Vec<usize>,
    comp_sizes: Vec<usize>,
    next: Vec<usize>,
    prev: Vec<usize>,
}

impl Scratch {
    fn new(n: usize) -> Self {
        Scratch {
            queue: Vec::with_capacity(n),
            in_queue: vec![false; n],
            added: Vec::new(),
            comp_of: vec![usize::MAX; n],
            comp_sizes: Vec::new(),
            next: vec![usize::MAX; n],
            prev: vec![usize::MAX; n],
        }
    }
}

/// Fills `comp_of` with a component id per city and `sizes` with each component's vertex count.
fn component_map(graph: &TourGraph, comp_of: &mut [usize], sizes: &mut Vec<usize>) {
    let n = graph.len();
    comp_of.fill(usize::MAX);
    sizes.clear();
    for start in 0..n {
        if comp_of[start] != usize::MAX {
            continue;
        }
        let id = sizes.len();
        let mut count = 0usize;
        let mut prev = usize::MAX;
        let mut current = start;
        loop {
            comp_of[current] = id;
            count += 1;
            let [a, b] = graph.nbr[current];
            let next = if a != prev { a } else { b };
            prev = current;
            current = next;
            if current == start {
                break;
            }
        }
        sizes.push(count);
    }
}

/// Merges the smallest component into a neighbour component. Returns `false` when no candidate
/// edge crosses components, so the caller can widen the search.
fn merge_via_candidates(
    graph: &mut TourGraph,
    comp_of: &[usize],
    small: usize,
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
    added: &mut Vec<(usize, usize)>,
) -> bool {
    let n = graph.len();
    let mut best: Option<(f32, usize, usize, usize, usize)> = None;

    for a in 0..n {
        if comp_of[a] != small {
            continue;
        }
        for &b in &graph.nbr[a] {
            for &cand in candidates.neighbours_by_pos(Position::new(a)) {
                let c = cand.into_inner();
                if c >= n || comp_of[c] == small {
                    continue;
                }
                for &d in &graph.nbr[c] {
                    consider_bridge(distances, a, b, c, d, &mut best);
                }
            }
        }
    }

    apply_best_bridge(graph, best, added).is_some()
}

/// Full-scan fallback used only when no candidate edge crosses components.
fn merge_via_full_scan(
    graph: &mut TourGraph,
    comp_of: &[usize],
    small: usize,
    distances: &dyn DistanceSource,
    added: &mut Vec<(usize, usize)>,
) {
    let n = graph.len();
    let mut best: Option<(f32, usize, usize, usize, usize)> = None;

    for a in 0..n {
        if comp_of[a] != small {
            continue;
        }
        for &b in &graph.nbr[a] {
            for c in 0..n {
                if comp_of[c] == small {
                    continue;
                }
                for &d in &graph.nbr[c] {
                    consider_bridge(distances, a, b, c, d, &mut best);
                }
            }
        }
    }

    apply_best_bridge(graph, best, added)
        .expect("a complete graph always has a bridge between components");
}

fn consider_bridge(
    distances: &dyn DistanceSource,
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    best: &mut Option<(f32, usize, usize, usize, usize)>,
) {
    if a == c || a == d || b == c || b == d {
        return;
    }
    let delta = distance(distances, a, c) + distance(distances, b, d)
        - distance(distances, a, b)
        - distance(distances, c, d);
    let better = match best {
        None => true,
        Some((best_delta, _, _, _, _)) => delta < *best_delta,
    };
    if better {
        *best = Some((delta, a, b, c, d));
    }
}

/// Applies `(a,b)` + `(c,d)` → `(a,c)` + `(b,d)`, merging the two cycles.
fn apply_best_bridge(
    graph: &mut TourGraph,
    best: Option<(f32, usize, usize, usize, usize)>,
    added: &mut Vec<(usize, usize)>,
) -> Option<f32> {
    let (delta, a, b, c, d) = best?;
    graph.replace_neighbour(a, b, c);
    graph.replace_neighbour(b, a, d);
    graph.replace_neighbour(c, d, a);
    graph.replace_neighbour(d, c, b);
    added.push((a, c));
    added.push((b, d));
    Some(delta)
}

fn patch_subtours(
    graph: &mut TourGraph,
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
    scratch: &mut Scratch,
) {
    loop {
        component_map(graph, &mut scratch.comp_of, &mut scratch.comp_sizes);
        if scratch.comp_sizes.len() <= 1 {
            break;
        }
        let small = scratch
            .comp_sizes
            .iter()
            .enumerate()
            .min_by_key(|&(_, &size)| size)
            .map(|(id, _)| id)
            .expect("at least one component");
        scratch.added.clear();
        if !merge_via_candidates(
            graph,
            &scratch.comp_of,
            small,
            candidates,
            distances,
            &mut scratch.added,
        ) {
            merge_via_full_scan(
                graph,
                &scratch.comp_of,
                small,
                distances,
                &mut scratch.added,
            );
        }
        debug_assert!(
            !scratch.added.is_empty(),
            "every patch round must add at least one bridge"
        );
    }
}

// ---------------------------------------------------------------------------
// Dirty-edge 2-opt
// ---------------------------------------------------------------------------

/// Fills `next`/`prev` with one orientation of the tour, starting at position 0.
fn orient(graph: &TourGraph, next: &mut [usize], prev: &mut [usize]) {
    let n = graph.len();
    if n == 0 {
        return;
    }
    next.fill(usize::MAX);
    prev.fill(usize::MAX);
    let mut previous = usize::MAX;
    let mut current = 0usize;
    for _ in 0..n {
        let [a, b] = graph.nbr[current];
        let following = if a != previous { a } else { b };
        next[current] = following;
        prev[following] = current;
        previous = current;
        current = following;
    }
    debug_assert_eq!(current, 0, "orientation must close the cycle");
    debug_assert!(
        next.iter().all(|&x| x != usize::MAX),
        "orientation must cover every city"
    );
}

/// Candidate-restricted 2-opt, seeded from `dirty` vertices and propagated from accepted swaps.
fn polish(
    graph: &mut TourGraph,
    dirty: &[usize],
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
    scratch: &mut Scratch,
) -> bool {
    let n = graph.len();
    if n < 4 {
        return false;
    }
    orient(graph, &mut scratch.next, &mut scratch.prev);
    scratch.queue.clear();
    scratch.in_queue.iter_mut().for_each(|flag| *flag = false);
    for &city in dirty {
        if city < n && !scratch.in_queue[city] {
            scratch.in_queue[city] = true;
            scratch.queue.push(city);
        }
    }

    let mut improved = false;
    let mut head = 0usize;
    while head < scratch.queue.len() {
        let u = scratch.queue[head];
        head += 1;
        scratch.in_queue[u] = false;

        let Some((a, b, c, d)) = best_2opt_move(graph, u, &scratch.next, candidates, distances)
        else {
            continue;
        };
        apply_2opt(graph, a, b, c, d);
        orient(graph, &mut scratch.next, &mut scratch.prev);
        improved = true;
        for &city in &[a, b, c, d] {
            if !scratch.in_queue[city] {
                scratch.in_queue[city] = true;
                scratch.queue.push(city);
            }
        }
    }
    improved
}

/// Returns `(u, a, v, c)` for the best improving reconnection: the removed edges are
/// `(u, a)` and `(v, c)`, and the added edges are `(u, v)` and `(a, c)`.
///
/// The orientation check is what keeps the move on one cycle: reconnecting endpoints that
/// lie on the same side of the two cut edges would split the tour into two subtours.
#[allow(clippy::type_complexity)]
fn best_2opt_move(
    graph: &TourGraph,
    u: usize,
    next: &[usize],
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
) -> Option<(usize, usize, usize, usize)> {
    let [u0, u1] = graph.nbr[u];
    let mut best: Option<(f32, usize, usize, usize, usize)> = None;

    for &a in &[u0, u1] {
        let u_other = if a == u0 { u1 } else { u0 };
        let u_is_forward = next[u] == a;
        for &cand in candidates.neighbours_by_pos(Position::new(u)) {
            let v = cand.into_inner();
            if v == a || v == u_other {
                continue;
            }
            for &c in &graph.nbr[v] {
                if c == u || c == a {
                    continue;
                }
                let v_is_forward = next[v] == c;
                if u_is_forward == v_is_forward {
                    // Same direction: (u,v) and (a,c).
                    if graph.has_edge(u, v) || graph.has_edge(a, c) {
                        continue;
                    }
                    let delta = distance(distances, u, v) + distance(distances, a, c)
                        - distance(distances, u, a)
                        - distance(distances, v, c);
                    if delta < -EPS
                        && best
                            .as_ref()
                            .is_none_or(|&(best_delta, ..)| delta < best_delta)
                    {
                        best = Some((delta, u, a, v, c));
                    }
                } else {
                    // Opposite directions: (u,c) and (a,v).
                    if graph.has_edge(u, c) || graph.has_edge(a, v) {
                        continue;
                    }
                    let delta = distance(distances, u, c) + distance(distances, a, v)
                        - distance(distances, u, a)
                        - distance(distances, v, c);
                    if delta < -EPS
                        && best
                            .as_ref()
                            .is_none_or(|&(best_delta, ..)| delta < best_delta)
                    {
                        best = Some((delta, u, a, c, v));
                    }
                }
            }
        }
    }

    best.map(|(_, a, b, c, d)| (a, b, c, d))
}

fn apply_2opt(graph: &mut TourGraph, u: usize, a: usize, v: usize, c: usize) {
    graph.replace_neighbour(u, a, v);
    graph.replace_neighbour(a, u, c);
    graph.replace_neighbour(v, c, u);
    graph.replace_neighbour(c, v, a);
}

// ---------------------------------------------------------------------------
// Population
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Member {
    graph: TourGraph,
    cost: f32,
}

impl Member {
    fn new(graph: TourGraph, distances: &dyn DistanceSource) -> Self {
        let cost = graph.edge_sum(distances);
        Member { graph, cost }
    }
}

fn canonical_tour(tour: &[usize]) -> Vec<usize> {
    let n = tour.len();
    if n == 0 {
        return Vec::new();
    }
    let min_pos = tour
        .iter()
        .enumerate()
        .min_by_key(|&(_, &city)| city)
        .map(|(pos, _)| pos)
        .expect("non-empty tour");
    let forward: Vec<usize> = (0..n).map(|i| tour[(min_pos + i) % n]).collect();
    let backward: Vec<usize> = (0..n).map(|i| tour[(min_pos + n - i) % n]).collect();
    if backward < forward {
        backward
    } else {
        forward
    }
}

fn build_candidates(
    distances: &dyn DistanceSource,
    cities: &[KDPoint],
    k: usize,
) -> Result<CandidateList, &'static str> {
    if distances.is_coordinate_metric() {
        CandidateList::build(cities, k)
    } else {
        CandidateList::build_from_distances(distances, cities, k)
    }
}

fn randomized_nn(
    n: usize,
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
    rng: &mut impl RngExt,
    start: usize,
) -> Vec<usize> {
    let mut visited = vec![false; n];
    let mut tour = Vec::with_capacity(n);
    let mut current = start;
    visited[current] = true;
    tour.push(current);

    for _ in 1..n {
        let mut choices = [usize::MAX; 3];
        let mut count = 0usize;
        for &cand in candidates.neighbours_by_pos(Position::new(current)) {
            let c = cand.into_inner();
            if !visited[c] {
                choices[count] = c;
                count += 1;
                if count == 3 {
                    break;
                }
            }
        }
        current = if count == 0 {
            // Candidate list exhausted (k < n - 1); fall back to the nearest unvisited city.
            (0..n)
                .filter(|&city| !visited[city])
                .min_by(|&a, &b| {
                    distance(distances, current, a).total_cmp(&distance(distances, current, b))
                })
                .expect("at least one unvisited city remains")
        } else {
            choices[rng.random_range(0..count)]
        };
        visited[current] = true;
        tour.push(current);
    }
    tour
}

fn shuffled_tour(n: usize, rng: &mut impl RngExt) -> Vec<usize> {
    let mut tour: Vec<usize> = (0..n).collect();
    tour.shuffle(rng);
    tour
}

fn add_member(
    graph: TourGraph,
    distances: &dyn DistanceSource,
    seen: &mut HashSet<Vec<usize>>,
    members: &mut Vec<Member>,
    allow_duplicate: bool,
) -> bool {
    let key = canonical_tour(&graph.to_tour(0));
    if !allow_duplicate && !seen.insert(key) {
        return false;
    }
    members.push(Member::new(graph, distances));
    true
}

fn initial_population(
    n: usize,
    opts: &MemeticEaxOptions,
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
    init_tour: Option<&[usize]>,
    scratch: &mut Scratch,
    rng: &mut impl RngExt,
) -> Vec<Member> {
    let target = opts.pop_size.max(1);
    let mut members: Vec<Member> = Vec::with_capacity(target);
    let mut seen: HashSet<Vec<usize>> = HashSet::new();
    let all: Vec<usize> = (0..n).collect();

    if let Some(seed) = init_tour.filter(|tour| is_permutation(tour, n)) {
        // The raw seed is kept as one member so a pipeline's warm start is never lost; a
        // polished copy is added next so the search starts from a local optimum too.
        add_member(
            TourGraph::from_tour(seed),
            distances,
            &mut seen,
            &mut members,
            false,
        );
        let mut polished = TourGraph::from_tour(seed);
        polish(&mut polished, &all, candidates, distances, scratch);
        add_member(polished, distances, &mut seen, &mut members, false);
    }

    let mut attempts = 0usize;
    let max_attempts = target.saturating_mul(8).max(16);
    while members.len() < target && attempts < max_attempts {
        attempts += 1;
        let tour = if attempts.is_multiple_of(3) {
            shuffled_tour(n, rng)
        } else {
            let start = rng.random_range(0..n);
            randomized_nn(n, candidates, distances, rng, start)
        };
        let mut graph = TourGraph::from_tour(&tour);
        polish(&mut graph, &all, candidates, distances, scratch);
        add_member(graph, distances, &mut seen, &mut members, false);
    }

    // Instances with fewer distinct local optima than `pop_size` fall back to perturbed copies.
    while members.len() < target {
        let base = rng.random_range(0..members.len());
        let tour = members[base].graph.to_tour(0);
        let kicked = double_bridge(&tour, rng);
        let mut graph = TourGraph::from_tour(&kicked);
        polish(&mut graph, &all, candidates, distances, scratch);
        add_member(graph, distances, &mut seen, &mut members, true);
    }

    members
}

// ---------------------------------------------------------------------------
// Crossover
// ---------------------------------------------------------------------------

fn select_e_set<'a>(
    cycles: &'a [AbCycle],
    strategy: EaxStrategy,
    hybrid_as_single_ab: bool,
    rng: &mut impl RngExt,
) -> Vec<&'a AbCycle> {
    if cycles.is_empty() {
        return Vec::new();
    }
    let single_ab = matches!(strategy, EaxStrategy::SingleAb)
        || (matches!(strategy, EaxStrategy::Hybrid) && hybrid_as_single_ab);
    if single_ab {
        return vec![&cycles[rng.random_range(0..cycles.len())]];
    }

    let mut selected: Vec<&AbCycle> = cycles.iter().filter(|_| bernoulli(rng, 0.5)).collect();
    if selected.is_empty() {
        selected.push(&cycles[rng.random_range(0..cycles.len())]);
    }
    selected
}

fn perturb(
    member: &Member,
    n: usize,
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
    rng: &mut impl RngExt,
    scratch: &mut Scratch,
) -> Member {
    let tour = member.graph.to_tour(0);
    let kicked = double_bridge(&tour, rng);
    let mut graph = TourGraph::from_tour(&kicked);
    let all: Vec<usize> = (0..n).collect();
    polish(&mut graph, &all, candidates, distances, scratch);
    Member::new(graph, distances)
}

#[allow(clippy::too_many_arguments)]
fn make_child(
    parent_a: &Member,
    parent_b: &Member,
    opts: &MemeticEaxOptions,
    hybrid_as_single_ab: bool,
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
    rng: &mut impl RngExt,
    scratch: &mut Scratch,
) -> Option<Member> {
    let n = parent_a.graph.len();
    let cycles = match ab_cycles(&parent_a.graph, &parent_b.graph) {
        Ok(cycles) => cycles,
        Err(_) => return None,
    };
    if cycles.is_empty() {
        return Some(perturb(parent_a, n, candidates, distances, rng, scratch));
    }

    let selected = select_e_set(&cycles, opts.strategy, hybrid_as_single_ab, rng);
    let mut child = parent_a.graph.clone();
    for cycle in &selected {
        apply_cycle(&mut child, &cycle.vertices);
    }
    patch_subtours(&mut child, candidates, distances, scratch);

    // The bridge endpoints are the only vertices the crossover disturbed; propagating from
    // them avoids rescanning the parents' already locally optimal regions.
    let dirty: Vec<usize> = scratch.added.iter().flat_map(|&(a, b)| [a, b]).collect();
    if dirty.is_empty() {
        return None;
    }
    polish(&mut child, &dirty, candidates, distances, scratch);
    debug_assert!(
        child.is_single_cycle(),
        "an offspring must stay one Hamiltonian cycle"
    );
    Some(Member::new(child, distances))
}

/// Pairs a shuffled permutation of the population and reports each distinct pair once.
fn pair_order(population_len: usize, rng: &mut impl RngExt) -> Vec<(usize, usize)> {
    let mut order: Vec<usize> = (0..population_len).collect();
    order.shuffle(rng);
    let mut pairs = Vec::with_capacity(order.len() / 2);
    let mut index = 0;
    while index + 1 < order.len() {
        pairs.push((order[index], order[index + 1]));
        index += 2;
    }
    pairs
}

fn should_stop(epochs_done: usize, opts: &MemeticEaxOptions, stagnant: usize) -> bool {
    epochs_done >= opts.epochs || stagnant >= opts.stall_epochs.saturating_mul(2)
}

fn positions_to_ids(positions: &[usize], distances: &dyn DistanceSource) -> Vec<usize> {
    positions
        .iter()
        .map(|&pos| {
            distances
                .pos2city_id(Position::new(pos))
                .expect("position came from the dense index")
        })
        .collect()
}

fn send_progress(
    tx: Option<&mpsc::Sender<ProgressMessage>>,
    positions: &[usize],
    distances: &dyn DistanceSource,
    cost: f32,
) {
    if let Some(tx) = tx {
        let ids = positions_to_ids(positions, distances);
        let _ = tx.send(ProgressMessage::PathUpdate(Route::new(&ids), cost));
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn solve(
    problem: &TspProblem,
    opts: &MemeticEaxOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    init_tour: Option<&[usize]>,
) -> Solution {
    let distances = problem.distances.as_ref();
    let n = distances.num_cities();

    let init_positions: Option<Vec<usize>> = init_tour.and_then(|tour| {
        tour.iter()
            .map(|&id| distances.city_id2pos(id).map(Position::into_inner))
            .collect()
    });

    if n < 4 {
        let positions = init_positions.unwrap_or_else(|| (0..n).collect());
        let ids = positions_to_ids(&positions, distances);
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::Done);
        }
        return Solution::from_parts(&ids, distances);
    }

    let k = opts.candidate_k.min(n.saturating_sub(1)).max(1);
    let candidates = match build_candidates(distances, &problem.cities, k) {
        Ok(candidates) => candidates,
        Err(error) => {
            tracing::warn!(
                error,
                "memetic_eax: candidate list unavailable; returning the seed tour"
            );
            let positions = init_positions.unwrap_or_else(|| (0..n).collect());
            let ids = positions_to_ids(&positions, distances);
            return Solution::from_parts(&ids, distances);
        }
    };

    let mut rng = rand::rngs::StdRng::seed_from_u64(opts.seed.unwrap_or_else(rand::random::<u64>));

    tracing::info!(
        population = opts.pop_size,
        children_per_pair = opts.children_per_pair,
        epochs = opts.epochs,
        "memetic_eax: starting"
    );

    let mut scratch = Scratch::new(n);
    let mut population = initial_population(
        n,
        opts,
        &candidates,
        distances,
        init_positions.as_deref(),
        &mut scratch,
        &mut rng,
    );
    population.sort_by(|a, b| a.cost.total_cmp(&b.cost));

    let mut best = population[0].clone();
    let mut best_cost = best.cost;
    let mut stagnant = 0usize;
    let mut hybrid_as_single_ab = false;

    send_progress(progress_tx, &best.graph.to_tour(0), distances, best_cost);

    let mut epoch = 0usize;
    while !should_stop(epoch, opts, stagnant) {
        let pairs = pair_order(population.len(), &mut rng);
        for (a_idx, b_idx) in pairs {
            let mut best_child: Option<Member> = None;
            for _ in 0..opts.children_per_pair {
                if let Some(child) = make_child(
                    &population[a_idx],
                    &population[b_idx],
                    opts,
                    hybrid_as_single_ab,
                    &candidates,
                    distances,
                    &mut rng,
                    &mut scratch,
                ) && best_child
                    .as_ref()
                    .is_none_or(|current| child.cost < current.cost)
                {
                    best_child = Some(child);
                }
            }
            if let Some(child) = best_child
                && child.cost < population[a_idx].cost - EPS
            {
                population[a_idx] = child;
            }
        }

        let current_best = population
            .iter()
            .min_by(|a, b| a.cost.total_cmp(&b.cost))
            .expect("population is never empty");
        if current_best.cost < best_cost - EPS {
            best = current_best.clone();
            best_cost = best.cost;
            stagnant = 0;
            send_progress(progress_tx, &best.graph.to_tour(0), distances, best_cost);
        } else {
            stagnant += 1;
        }

        if matches!(opts.strategy, EaxStrategy::Hybrid)
            && !hybrid_as_single_ab
            && stagnant >= opts.stall_epochs
        {
            hybrid_as_single_ab = true;
            tracing::debug!(epoch, "memetic_eax: hybrid switched to single_ab");
        }

        tracing::debug!(epoch, best = best_cost, stagnant, "memetic_eax: generation");
        epoch += 1;
    }

    let tour = best.graph.to_tour(0);
    let ids = positions_to_ids(&tour, distances);
    tracing::info!(
        epochs = epoch,
        best = best_cost,
        stagnant,
        "memetic_eax: finished"
    );
    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(&ids, distances)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree};

    fn points(n: usize) -> Vec<KDPoint> {
        kdtree::build_points(
            &(0..n)
                .map(|i| vec![(i as f32) * 1.7 % 23.0, (i as f32) * 2.3 % 19.0])
                .collect::<Vec<_>>(),
        )
    }

    fn problem(cities: Vec<KDPoint>) -> TspProblem {
        let distances = distance_matrix::from_cities_unrounded(&cities);
        TspProblem::new(cities, distances)
    }

    fn options() -> MemeticEaxOptions {
        MemeticEaxOptions {
            epochs: 5,
            pop_size: 6,
            children_per_pair: 3,
            strategy: EaxStrategy::Hybrid,
            stall_epochs: 2,
            candidate_k: 3,
            seed: Some(7),
        }
    }

    #[test]
    fn graph_round_trips_edges() {
        let tour = vec![3usize, 1, 4, 0, 2];
        let graph = TourGraph::from_tour(&tour);
        let rebuilt = graph.to_tour(3);
        for i in 0..tour.len() {
            assert!(graph.has_edge(tour[i], tour[(i + 1) % tour.len()]));
        }
        assert_eq!(rebuilt, tour);
    }

    #[test]
    fn graph_has_degree_two_for_every_city() {
        let tour = vec![0usize, 1, 2, 3, 4, 5];
        let graph = TourGraph::from_tour(&tour);
        for slots in &graph.nbr {
            assert_ne!(slots[0], usize::MAX);
            assert_ne!(slots[1], usize::MAX);
            assert_ne!(slots[0], slots[1]);
        }
    }

    #[test]
    fn replace_neighbour_swaps_the_edge() {
        let mut graph = TourGraph::from_tour(&[0usize, 1, 2, 3]);
        graph.replace_neighbour(0, 1, 2);
        assert!(graph.has_edge(0, 2));
        assert!(!graph.has_edge(0, 1));
    }

    #[test]
    fn shared_edges_do_not_appear_in_ab_cycles() {
        let a = TourGraph::from_tour(&[0usize, 1, 2, 3]);
        let b = TourGraph::from_tour(&[0usize, 2, 1, 3]);
        let cycles = ab_cycles(&a, &b).expect("valid parents");
        assert_eq!(cycles.len(), 1);
        let vertices = &cycles[0].vertices;
        assert_eq!(vertices.len(), 4, "one 4-cycle is expected");
        for i in 0..vertices.len() {
            let from = vertices[i];
            let to = vertices[(i + 1) % vertices.len()];
            assert!(!(from == 1 && to == 2 || from == 2 && to == 1));
            assert!(!(from == 0 && to == 3 || from == 3 && to == 0));
        }
    }

    #[test]
    fn ab_cycles_alternate_origin_and_have_even_length() {
        let a = TourGraph::from_tour(&[0usize, 1, 2, 3, 4, 5]);
        let b = TourGraph::from_tour(&[0usize, 2, 4, 1, 3, 5]);
        let cycles = ab_cycles(&a, &b).expect("valid parents");
        assert!(!cycles.is_empty());
        for cycle in cycles {
            assert_eq!(cycle.vertices.len() % 2, 0);
            assert!(cycle.vertices.len() >= 4);
            for i in 0..cycle.vertices.len() {
                let from = cycle.vertices[i];
                let to = cycle.vertices[(i + 1) % cycle.vertices.len()];
                let is_a_edge = a.has_edge(from, to);
                let is_b_edge = b.has_edge(from, to);
                assert_ne!(is_a_edge, is_b_edge, "cycle edges must alternate origin");
            }
        }
    }

    #[test]
    fn identical_parents_have_no_cycles() {
        let a = TourGraph::from_tour(&[0usize, 1, 2, 3, 4]);
        let b = a.clone();
        assert!(ab_cycles(&a, &b).unwrap().is_empty());
    }

    #[test]
    fn applying_a_cycle_preserves_degree_and_shared_edges() {
        let a = TourGraph::from_tour(&[0usize, 1, 2, 3]);
        let b = TourGraph::from_tour(&[0usize, 2, 1, 3]);
        let cycles = ab_cycles(&a, &b).unwrap();
        let mut child = a.clone();
        apply_cycle(&mut child, &cycles[0].vertices);
        for slots in &child.nbr {
            assert_ne!(slots[0], usize::MAX);
            assert_ne!(slots[1], usize::MAX);
        }
        assert!(child.has_edge(1, 2), "shared edge must survive");
        assert!(child.has_edge(0, 3), "shared edge must survive");
        assert!(child.has_edge(0, 2), "B edge must be added");
        assert!(child.has_edge(1, 3), "B edge must be added");
    }

    #[test]
    fn patching_merges_two_components() {
        // Two disjoint triangles in one graph.
        let mut graph = TourGraph {
            nbr: vec![[1, 2], [0, 2], [0, 1], [4, 5], [3, 5], [3, 4]],
        };
        assert!(!graph.is_single_cycle());
        let cities = points(6);
        let distances = distance_matrix::from_cities_unrounded(&cities);
        let candidates = CandidateList::build(&cities, 5).unwrap();
        let mut scratch = Scratch::new(6);
        component_map(&graph, &mut scratch.comp_of, &mut scratch.comp_sizes);
        assert_eq!(scratch.comp_sizes.len(), 2);
        merge_via_candidates(
            &mut graph,
            &scratch.comp_of,
            0,
            &candidates,
            &distances,
            &mut scratch.added,
        );
        assert!(graph.is_single_cycle());
    }

    #[test]
    fn polish_improves_a_crossed_square() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
        ]);
        let distances = distance_matrix::from_cities_unrounded(&cities);
        let candidates = CandidateList::build(&cities, 3).unwrap();
        let mut graph = TourGraph::from_tour(&[0usize, 1, 3, 2]);
        let before = graph.edge_sum(&distances);
        let mut scratch = Scratch::new(4);
        let all: Vec<usize> = (0..4).collect();
        polish(&mut graph, &all, &candidates, &distances, &mut scratch);
        let after = graph.edge_sum(&distances);
        assert!(after < before, "polish must remove the crossing");
        assert!(graph.is_single_cycle());
    }

    #[test]
    fn polish_never_increases_cost() {
        let cities = points(12);
        let distances = distance_matrix::from_cities_unrounded(&cities);
        let candidates = CandidateList::build(&cities, 4).unwrap();
        let mut graph = TourGraph::from_tour(&(0..12).collect::<Vec<_>>());
        let before = graph.edge_sum(&distances);
        let mut scratch = Scratch::new(12);
        let all: Vec<usize> = (0..12).collect();
        polish(&mut graph, &all, &candidates, &distances, &mut scratch);
        assert!(graph.edge_sum(&distances) <= before + EPS);
        assert!(graph.is_single_cycle(), "polish must keep one cycle");
    }

    #[test]
    fn e_set_strategies_select_valid_cycles() {
        let a = TourGraph::from_tour(&[0usize, 1, 2, 3, 4, 5]);
        let b = TourGraph::from_tour(&[0usize, 2, 4, 1, 3, 5]);
        let cycles = ab_cycles(&a, &b).unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        assert_eq!(
            select_e_set(&cycles, EaxStrategy::SingleAb, false, &mut rng).len(),
            1
        );
        assert!(!select_e_set(&cycles, EaxStrategy::Rand, false, &mut rng).is_empty());
        assert!(!select_e_set(&cycles, EaxStrategy::Hybrid, false, &mut rng).is_empty());
        assert_eq!(
            select_e_set(&cycles, EaxStrategy::Hybrid, true, &mut rng).len(),
            1
        );
    }

    #[test]
    fn pairing_never_pairs_a_member_with_itself() {
        for n in 0..6 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(n as u64);
            for (a, b) in pair_order(n, &mut rng) {
                assert_ne!(a, b, "self-pair in population of {n}");
                assert!(a < n && b < n);
            }
        }
    }

    #[test]
    fn should_stop_honours_epochs_and_twice_the_stall() {
        let mut opts = options();
        opts.epochs = 100;
        opts.stall_epochs = 3;
        assert!(!should_stop(99, &opts, 0));
        assert!(should_stop(100, &opts, 0));
        assert!(!should_stop(0, &opts, 5));
        assert!(should_stop(0, &opts, 6));
    }

    #[test]
    fn initial_population_contains_the_seed_and_valid_members() {
        let cities = points(10);
        let distances = distance_matrix::from_cities_unrounded(&cities);
        let problem = problem(cities.clone());
        let candidates = CandidateList::build(&problem.cities, 4).unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        let seed: Vec<usize> = (0..10).collect();
        let members = initial_population(
            10,
            &options(),
            &candidates,
            &distances,
            Some(&seed),
            &mut Scratch::new(10),
            &mut rng,
        );
        assert_eq!(members.len(), options().pop_size);
        assert_eq!(members[0].graph.to_tour(0), seed);
        for member in &members {
            assert!(member.graph.is_single_cycle());
        }
    }

    #[test]
    fn solve_returns_a_valid_tour_with_matching_cost() {
        let problem = problem(points(14));
        let solution = solve(&problem, &options(), None, None);
        let mut ids = solution.route().to_vec();
        ids.sort_unstable();
        assert_eq!(ids, (0..14).collect::<Vec<_>>());
        assert!((solution.total - problem.distances.tour_length(solution.route())).abs() < EPS);
    }

    #[test]
    fn solve_handles_tiny_instances() {
        for n in 2..4 {
            let problem = problem(points(n));
            let solution = solve(&problem, &options(), None, None);
            assert_eq!(solution.route().len(), n);
        }
    }

    #[test]
    fn seeded_runs_are_reproducible() {
        let problem = problem(points(12));
        let mut opts = options();
        opts.seed = Some(99);
        let first = solve(&problem, &opts, None, None);
        let second = solve(&problem, &opts, None, None);
        assert_eq!(first.route(), second.route());
        assert!((first.total - second.total).abs() < EPS);
    }

    #[test]
    fn explicit_distances_are_ranked_by_the_matrix() {
        let cities: Vec<KDPoint> = (0..4)
            .map(|i| KDPoint::new_with_id(i + 1, &[i as f32, 0.0]))
            .collect();
        let matrix = distance_matrix::DistanceMatrix::new(
            4,
            vec![10.0, 20.0, 1.0, 1.0, 30.0, 30.0],
            cities.clone(),
        )
        .unwrap();
        let candidates = build_candidates(&matrix, &cities, 1).unwrap();
        assert_eq!(
            candidates.neighbours_by_pos(Position::new(0)),
            &[Position::new(3)],
            "matrix distance, not geometry, must rank the candidate"
        );
    }

    #[test]
    fn make_child_stays_a_single_cycle() {
        let problem = problem(points(10));
        let distances = problem.distances.as_ref();
        let candidates = CandidateList::build(&problem.cities, 4).unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(4);
        let a = Member::new(
            TourGraph::from_tour(&(0..10).collect::<Vec<_>>()),
            distances,
        );
        let b = Member::new(
            TourGraph::from_tour(&[0usize, 2, 4, 6, 8, 1, 3, 5, 7, 9]),
            distances,
        );
        let mut scratch = Scratch::new(10);
        let child = make_child(
            &a,
            &b,
            &options(),
            false,
            &candidates,
            distances,
            &mut rng,
            &mut scratch,
        );
        if let Some(child) = child {
            assert!(child.graph.is_single_cycle());
        }
    }

    #[test]
    #[ignore = "benchmark smoke test; run with --ignored --nocapture"]
    fn bench_quality() {
        let cases = [
            ("berlin52", 52usize, 7542.0f32),
            ("kroA100", 100, 21282.0),
            ("a280", 280, 2579.0),
            ("dsj1000", 1000, 18660188.0),
        ];
        for (name, n, optimum) in cases {
            let path = format!("data/tsplib/{name}.tsp");
            let data = crate::tsp::tsplib::read_from_file(std::path::Path::new(&path))
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(data.cities().len(), n, "{name} dimension");
            let problem = data
                .into_problem(crate::tsp::DistanceBackend::Matrix)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let mut opts = MemeticEaxOptions::default();
            opts.seed = Some(1);
            let start = std::time::Instant::now();
            let solution = solve(&problem, &opts, None, None);
            let elapsed = start.elapsed();
            let gap = (solution.total - optimum) / optimum * 100.0;
            println!("{name}: {} ({gap:.3}%) in {elapsed:?}", solution.total);
        }
    }
}

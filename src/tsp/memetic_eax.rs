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
use super::nearest_neighbor;
use super::probability::bernoulli;
use super::progress::ProgressMessage;
use super::random_shuffle;
use super::route::Route;
use super::tour_graph::{TourGraph, distance};
use super::two_opt_move;
use super::{Solution, TspProblem, kdtree::KDPoint};
use rand::seq::SliceRandom;
use rand::{RngExt, SeedableRng};
use std::collections::HashSet;
use std::sync::mpsc;

/// Cost improvement below this is treated as zero; without it a float-noise swap would loop.
const EPS: f32 = 1e-6;

/// Merge candidates are capped below the local-search list: patching only needs a few
/// boundary neighbours, and the smaller scan is a large win at n = 1,000.
const PATCH_CANDIDATE_K: usize = 15;

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
    /// Maximum generations; `0` means no epoch cap (the stagnation stop still applies).
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
            // Lower than the shared 500: at 500, dsj1000 runs regularly exceeded the
            // 300-second meta benchmark budget; 250 keeps the default inside it.
            epochs: 250,
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
        if self.pop_size < 2 {
            return Err("pop_size must be >= 2".to_string());
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

    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut eax = MemeticEaxOptions::default();
        for (key, value) in table.iter() {
            match key.as_str() {
                "epochs" => eax.epochs = super::parse_nonneg_usize(value, "epochs")?,
                "pop_size" => eax.pop_size = super::parse_nonneg_usize(value, "pop_size")?,
                "children_per_pair" => {
                    eax.children_per_pair = super::parse_nonneg_usize(value, "children_per_pair")?;
                }
                "strategy" => {
                    let raw = value.as_str().ok_or_else(|| {
                        format!("config: `strategy` must be a string, got {value}")
                    })?;
                    eax.strategy = raw.parse()?;
                }
                "stall_epochs" => {
                    eax.stall_epochs = super::parse_nonneg_usize(value, "stall_epochs")?;
                }
                "candidate_k" => {
                    eax.candidate_k = super::parse_nonneg_usize(value, "candidate_k")?;
                }
                "seed" => {
                    let raw = value
                        .as_integer()
                        .ok_or_else(|| format!("config: `seed` must be an integer, got {value}"))?;
                    eax.seed =
                        Some(u64::try_from(raw).map_err(|_| {
                            format!("config: `seed` must be non-negative, got {raw}")
                        })?);
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` in [memetic_eax] — valid: epochs, pop_size, children_per_pair, strategy, stall_epochs, candidate_k, seed"
                    ));
                }
            }
        }
        eax.validate()?;
        Ok(eax)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        let mut eax = MemeticEaxOptions::default();
        if let Some(v) = args.get_one::<String>("epochs") {
            eax.epochs = v
                .parse()
                .map_err(|_| format!("--epochs: invalid integer `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("pop_size") {
            eax.pop_size = v
                .parse()
                .map_err(|_| format!("--pop-size: invalid integer `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("children_per_pair") {
            eax.children_per_pair = v
                .parse()
                .map_err(|_| format!("--children-per-pair: invalid integer `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("strategy") {
            eax.strategy = v.parse()?;
        }
        if let Some(v) = args.get_one::<String>("stall_epochs") {
            eax.stall_epochs = v
                .parse()
                .map_err(|_| format!("--stall-epochs: invalid integer `{v}`"))?;
        } else if let Some(v) = args.get_one::<String>("stagnation_epochs") {
            let n: usize = v
                .parse()
                .map_err(|_| format!("--stagnation_epochs: invalid integer `{v}`"))?;
            // The shared flag counts the stop directly, while EAX stops at twice
            // `stall_epochs`; halving keeps the same effective run length. 0 means
            // "no override" here, so the default still bounds the run.
            if n > 0 {
                eax.stall_epochs = (n / 2).max(1);
            }
        }
        if let Some(v) = args.get_one::<String>("candidate_k") {
            eax.candidate_k = v
                .parse()
                .map_err(|_| format!("--candidate-k: invalid integer `{v}`"))?;
        } else if let Some(v) = args.get_one::<String>("n_nearest") {
            eax.candidate_k = v
                .parse()
                .map_err(|_| format!("--n_nearest: invalid integer `{v}`"))?;
        }
        if let Some(v) = args.get_one::<String>("seed") {
            eax.seed = Some(
                v.parse()
                    .map_err(|_| format!("--seed: invalid integer `{v}`"))?,
            );
        }
        eax.validate()?;
        Ok(eax)
    }
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

/// Decomposes the symmetric difference of two tours into alternating AB cycles.
///
/// Each city pairs its A-only edges with its B-only edges, which turns the difference graph
/// into disjoint even cycles. Shared edges never enter the difference graph.
fn ab_cycles(a: &TourGraph, b: &TourGraph) -> Result<Vec<AbCycle>, &'static str> {
    let n = a.len();
    if b.len() != n {
        return Err("parent graphs differ in size");
    }

    let mut a_edges: Vec<(usize, usize)> = Vec::new();
    let mut b_edges: Vec<(usize, usize)> = Vec::new();
    let mut incident_a: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut incident_b: Vec<Vec<usize>> = vec![Vec::new(); n];

    for city in 0..n {
        for neighbour in a.neighbours(city) {
            if city < neighbour && !b.has_edge(city, neighbour) {
                let edge = a_edges.len();
                a_edges.push((city, neighbour));
                incident_a[city].push(edge);
                incident_a[neighbour].push(edge);
            }
        }
        for neighbour in b.neighbours(city) {
            if city < neighbour && !a.has_edge(city, neighbour) {
                let edge = b_edges.len();
                b_edges.push((city, neighbour));
                incident_b[city].push(edge);
                incident_b[neighbour].push(edge);
            }
        }
    }

    for city in 0..n {
        if incident_a[city].len() != incident_b[city].len() {
            return Err("parent tours disagree on edge parity");
        }
    }

    let mut a_partner: Vec<[Option<usize>; 2]> = vec![[None, None]; a_edges.len()];
    let mut b_partner: Vec<[Option<usize>; 2]> = vec![[None, None]; b_edges.len()];
    for city in 0..n {
        for index in 0..incident_a[city].len() {
            let a_edge = incident_a[city][index];
            let b_edge = incident_b[city][index];
            let a_slot = usize::from(a_edges[a_edge].0 != city);
            let b_slot = usize::from(b_edges[b_edge].0 != city);
            a_partner[a_edge][a_slot] = Some(b_edge);
            b_partner[b_edge][b_slot] = Some(a_edge);
        }
    }

    let mut used_a = vec![false; a_edges.len()];
    let mut used_b = vec![false; b_edges.len()];
    let mut cycles = Vec::new();

    for start in 0..a_edges.len() {
        if used_a[start] {
            continue;
        }
        let entry_start = a_edges[start].0;
        let mut vertices = vec![entry_start];
        let mut entry = entry_start;
        let mut a_edge = start;

        loop {
            if used_a[a_edge] {
                return Err("AB traversal revisited an edge before closing");
            }
            used_a[a_edge] = true;

            let (u, v) = a_edges[a_edge];
            let exit = if entry == u { v } else { u };
            vertices.push(exit);
            let a_slot = usize::from(a_edges[a_edge].0 != exit);
            let b_edge = a_partner[a_edge][a_slot].ok_or("unpaired A endpoint")?;

            if used_b[b_edge] {
                return Err("AB traversal revisited a B edge before closing");
            }
            used_b[b_edge] = true;
            let (bu, bv) = b_edges[b_edge];
            let b_exit = if exit == bu { bv } else { bu };
            let b_slot = usize::from(b_edges[b_edge].0 != b_exit);
            let next_a = b_partner[b_edge][b_slot].ok_or("unpaired B endpoint")?;

            if next_a == start && b_exit == entry_start {
                break;
            }
            vertices.push(b_exit);
            entry = b_exit;
            a_edge = next_a;
        }

        if vertices.len() < 4 || !vertices.len().is_multiple_of(2) {
            return Err("AB cycle must be even and at least length 4");
        }
        cycles.push(AbCycle { vertices });
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
    comp_order: Vec<usize>,
    comp_start: Vec<usize>,
    pos: Vec<usize>,
}

impl Scratch {
    fn new(n: usize) -> Self {
        Scratch {
            queue: Vec::with_capacity(n),
            in_queue: vec![false; n],
            added: Vec::new(),
            comp_of: vec![usize::MAX; n],
            comp_order: Vec::with_capacity(n),
            comp_start: Vec::new(),
            pos: vec![0; n],
        }
    }
}

/// Fills `comp_of` with a component id per city, `comp_order` with the cities grouped by
/// component, and `comp_start` with each component's offset (length = components + 1).
///
/// The flat layout reuses two buffers across calls; a `Vec<Vec<_>>` would allocate once per
/// component per merge, which is exactly the hot path this patch is shortening.
fn component_map(
    graph: &TourGraph,
    comp_of: &mut [usize],
    comp_order: &mut Vec<usize>,
    comp_start: &mut Vec<usize>,
) {
    let n = graph.len();
    comp_of.fill(usize::MAX);
    comp_order.clear();
    comp_start.clear();
    comp_start.push(0);
    for start in 0..n {
        if comp_of[start] != usize::MAX {
            continue;
        }
        let id = comp_start.len() - 1;
        let mut prev = usize::MAX;
        let mut current = start;
        loop {
            comp_of[current] = id;
            comp_order.push(current);
            let [a, b] = graph.neighbours(current);
            let next = if a != prev { a } else { b };
            prev = current;
            current = next;
            if current == start {
                break;
            }
        }
        comp_start.push(comp_order.len());
    }
}

/// Merges the smallest component into a neighbour component. Returns `false` when no candidate
/// edge crosses components, so the caller can widen the search.
fn merge_via_candidates(
    graph: &mut TourGraph,
    comp_of: &[usize],
    small: usize,
    small_vertices: &[usize],
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
    added: &mut Vec<(usize, usize)>,
) -> bool {
    let n = graph.len();
    let mut best: Option<(f32, usize, usize, usize, usize, bool)> = None;

    for &a in small_vertices {
        for b in graph.neighbours(a) {
            for &cand in candidates
                .neighbours_by_pos(Position::new(a))
                .iter()
                .take(PATCH_CANDIDATE_K)
            {
                let c = cand.into_inner();
                if c >= n || comp_of[c] == small {
                    continue;
                }
                for d in graph.neighbours(c) {
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
    small_vertices: &[usize],
    distances: &dyn DistanceSource,
    added: &mut Vec<(usize, usize)>,
) {
    let mut best: Option<(f32, usize, usize, usize, usize, bool)> = None;

    for &a in small_vertices {
        for b in graph.neighbours(a) {
            for (c, &component) in comp_of.iter().enumerate() {
                if component == small {
                    continue;
                }
                for d in graph.neighbours(c) {
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
    best: &mut Option<(f32, usize, usize, usize, usize, bool)>,
) {
    if a == c || a == d || b == c || b == d {
        return;
    }
    let removed = distance(distances, a, b) + distance(distances, c, d);
    // The two ways to reconnect two removed edges; either merges the components.
    let cross = distance(distances, a, c) + distance(distances, b, d) - removed;
    if best
        .as_ref()
        .is_none_or(|&(best_delta, ..)| cross < best_delta)
    {
        *best = Some((cross, a, b, c, d, false));
    }
    let parallel = distance(distances, a, d) + distance(distances, b, c) - removed;
    if best
        .as_ref()
        .is_none_or(|&(best_delta, ..)| parallel < best_delta)
    {
        *best = Some((parallel, a, b, c, d, true));
    }
}

/// Applies `(a,b)` + `(c,d)` → `(a,c)` + `(b,d)`, merging the two cycles.
fn apply_best_bridge(
    graph: &mut TourGraph,
    best: Option<(f32, usize, usize, usize, usize, bool)>,
    added: &mut Vec<(usize, usize)>,
) -> Option<f32> {
    let (delta, a, b, c, d, parallel) = best?;
    if parallel {
        graph.replace_neighbour(a, b, d);
        graph.replace_neighbour(b, a, c);
        graph.replace_neighbour(c, d, b);
        graph.replace_neighbour(d, c, a);
        added.push((a, d));
        added.push((b, c));
    } else {
        graph.replace_neighbour(a, b, c);
        graph.replace_neighbour(b, a, d);
        graph.replace_neighbour(c, d, a);
        graph.replace_neighbour(d, c, b);
        added.push((a, c));
        added.push((b, d));
    }
    Some(delta)
}

fn patch_subtours(
    graph: &mut TourGraph,
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
    scratch: &mut Scratch,
) -> (usize, usize) {
    scratch.added.clear();
    component_map(
        graph,
        &mut scratch.comp_of,
        &mut scratch.comp_order,
        &mut scratch.comp_start,
    );
    let components_before = scratch.comp_start.len().saturating_sub(1);
    while scratch.comp_start.len() > 2 {
        let small = (0..scratch.comp_start.len() - 1)
            .min_by_key(|&id| scratch.comp_start[id + 1] - scratch.comp_start[id])
            .expect("at least one component");
        let range = scratch.comp_start[small]..scratch.comp_start[small + 1];
        if !merge_via_candidates(
            graph,
            &scratch.comp_of,
            small,
            &scratch.comp_order[range.clone()],
            candidates,
            distances,
            &mut scratch.added,
        ) {
            merge_via_full_scan(
                graph,
                &scratch.comp_of,
                small,
                &scratch.comp_order[range],
                distances,
                &mut scratch.added,
            );
        }
        component_map(
            graph,
            &mut scratch.comp_of,
            &mut scratch.comp_order,
            &mut scratch.comp_start,
        );
    }
    (components_before, scratch.added.len())
}

// ---------------------------------------------------------------------------
// Dirty-edge 2-opt
// ---------------------------------------------------------------------------

/// Candidate-restricted 2-opt on the flat tour, seeded by `dirty` cities and propagated from
/// accepted swaps. Reversing the affected segment keeps a move O(segment); the earlier version
/// re-derived the whole cycle orientation after every move, which made polish O(n · moves).
fn polish(
    tour: &mut [usize],
    dirty: &[usize],
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
    scratch: &mut Scratch,
) -> bool {
    let n = tour.len();
    if n < 4 {
        return false;
    }
    for (rank, &city) in tour.iter().enumerate() {
        scratch.pos[city] = rank;
    }
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

        let Some((i, j)) = best_2opt_move(tour, &scratch.pos, u, candidates, distances) else {
            continue;
        };
        let a = tour[i];
        let b = tour[(i + 1) % n];
        let c = tour[j];
        let d = tour[(j + 1) % n];
        two_opt_move::reverse_segment_with_pos(tour, &mut scratch.pos, i + 1, j);
        improved = true;
        for city in [a, b, c, d] {
            if !scratch.in_queue[city] {
                scratch.in_queue[city] = true;
                scratch.queue.push(city);
            }
        }
    }
    improved
}

/// Returns the best improving `(i, j)` for an edge adjacent to `u`, where `i < j` and the edges
/// `(i, i+1)` and `(j, j+1)` are replaced by `(i, j)` and `(i+1, j+1)`.
fn best_2opt_move(
    tour: &[usize],
    pos: &[usize],
    u: usize,
    candidates: &CandidateList,
    distances: &dyn DistanceSource,
) -> Option<(usize, usize)> {
    let n = tour.len();
    let p = pos[u];
    let u_edges = [p, (p + n - 1) % n];
    let mut best: Option<(f64, usize, usize)> = None;

    for &u_edge in &u_edges {
        for &cand in candidates.neighbours_by_pos(Position::new(u)) {
            let v = cand.into_inner();
            let q = pos[v];
            for &v_edge in &[q, (q + n - 1) % n] {
                let (i, j) = if u_edge < v_edge {
                    (u_edge, v_edge)
                } else {
                    (v_edge, u_edge)
                };
                if j <= i + 1 || (i == 0 && j == n - 1) {
                    continue;
                }
                let delta = two_opt_move::delta(tour, i, j, |a, b| distance(distances, a, b));
                if delta < -f64::from(EPS)
                    && best
                        .as_ref()
                        .is_none_or(|&(best_delta, ..)| delta < best_delta)
                {
                    best = Some((delta, i, j));
                }
            }
        }
    }

    best.map(|(_, i, j)| (i, j))
}

// ---------------------------------------------------------------------------
// Population
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
struct ChildStats {
    cycles: usize,
    components: usize,
    bridges: usize,
}

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
    let target = opts.pop_size.max(2);
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
        let mut polished = seed.to_vec();
        polish(&mut polished, &all, candidates, distances, scratch);
        add_member(
            TourGraph::from_tour(&polished),
            distances,
            &mut seen,
            &mut members,
            false,
        );
    }

    let mut attempts = 0usize;
    let max_attempts = target.saturating_mul(8).max(16);
    while members.len() < target && attempts < max_attempts {
        attempts += 1;
        let tour = if attempts.is_multiple_of(3) {
            let mut tour: Vec<usize> = (0..n).collect();
            random_shuffle::shuffle_slice(&mut tour, rng);
            tour
        } else {
            let start = rng.random_range(0..n);
            nearest_neighbor::randomized_tour(distances, candidates, start, 3, rng)
        };
        let mut tour = tour;
        polish(&mut tour, &all, candidates, distances, scratch);
        add_member(
            TourGraph::from_tour(&tour),
            distances,
            &mut seen,
            &mut members,
            false,
        );
    }

    // Instances with fewer distinct local optima than `pop_size` fall back to perturbed copies.
    while members.len() < target {
        let base = rng.random_range(0..members.len());
        let tour = members[base].graph.to_tour(0);
        let mut kicked = double_bridge(&tour, rng);
        polish(&mut kicked, &all, candidates, distances, scratch);
        add_member(
            TourGraph::from_tour(&kicked),
            distances,
            &mut seen,
            &mut members,
            true,
        );
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
    let mut kicked = double_bridge(&tour, rng);
    let all: Vec<usize> = (0..n).collect();
    polish(&mut kicked, &all, candidates, distances, scratch);
    Member::new(TourGraph::from_tour(&kicked), distances)
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
) -> Option<(Member, ChildStats)> {
    let n = parent_a.graph.len();
    let cycles = match ab_cycles(&parent_a.graph, &parent_b.graph) {
        Ok(cycles) => cycles,
        Err(error) => {
            tracing::debug!(error, "memetic_eax: AB-cycle decomposition failed");
            return None;
        }
    };
    if cycles.is_empty() {
        return Some((
            perturb(parent_a, n, candidates, distances, rng, scratch),
            ChildStats::default(),
        ));
    }

    let selected = select_e_set(&cycles, opts.strategy, hybrid_as_single_ab, rng);
    let mut child = parent_a.graph.clone();
    for cycle in &selected {
        apply_cycle(&mut child, &cycle.vertices);
    }
    let (components, bridges) = patch_subtours(&mut child, candidates, distances, scratch);

    // Every vertex of an applied AB cycle got a new neighbour; patching disturbs the bridge
    // endpoints again. Seed polish from both: with `single_ab` the crossover often already
    // yields one tour and therefore no bridge edges.
    let mut dirty: Vec<usize> = scratch.added.iter().flat_map(|&(a, b)| [a, b]).collect();
    dirty.extend(
        selected
            .iter()
            .flat_map(|cycle| cycle.vertices.iter().copied()),
    );
    let mut tour = child.to_tour(0);
    polish(&mut tour, &dirty, candidates, distances, scratch);
    let child = TourGraph::from_tour(&tour);
    debug_assert!(
        child.is_single_cycle(),
        "an offspring must stay one Hamiltonian cycle"
    );
    Some((
        Member::new(child, distances),
        ChildStats {
            cycles: selected.len(),
            components,
            bridges,
        },
    ))
}

/// Pairs a shuffled permutation of the population and reports each distinct pair once.
fn pair_order(population_len: usize, rng: &mut impl RngExt) -> Vec<(usize, usize)> {
    if population_len < 2 {
        return Vec::new();
    }
    let mut order: Vec<usize> = (0..population_len).collect();
    order.shuffle(rng);
    // A ring over the shuffled order: every member is parent A exactly once, and the last
    // member is paired with the first, so no member is left out.
    (0..population_len)
        .map(|i| (order[i], order[(i + 1) % population_len]))
        .collect()
}

fn should_replace(parent_cost: f32, child_cost: f32) -> bool {
    child_cost < parent_cost - EPS
}

fn should_stop(epochs_done: usize, opts: &MemeticEaxOptions, stagnant: usize) -> bool {
    (opts.epochs > 0 && epochs_done >= opts.epochs)
        || stagnant >= opts.stall_epochs.saturating_mul(2)
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

    // `solve` is public and the option fields are public, so a direct caller can bypass the
    // `validate()` that the dispatch path applies. Clamp instead of silently turning the run
    // into a no-op: `stall_epochs = 0` would stop immediately, `children_per_pair = 0` would
    // produce no offspring, and `pop_size = 1` would leave nothing to pair.
    let mut sanitized = opts.clone();
    if sanitized.pop_size < 2 {
        tracing::warn!("memetic_eax: pop_size below 2 clamped to 2");
        sanitized.pop_size = 2;
    }
    if sanitized.children_per_pair == 0 {
        tracing::warn!("memetic_eax: children_per_pair 0 clamped to 1");
        sanitized.children_per_pair = 1;
    }
    if sanitized.candidate_k == 0 {
        tracing::warn!("memetic_eax: candidate_k 0 clamped to 1");
        sanitized.candidate_k = 1;
    }
    if sanitized.stall_epochs == 0 {
        tracing::warn!("memetic_eax: stall_epochs 0 clamped to 1");
        sanitized.stall_epochs = 1;
    }
    let opts = &sanitized;

    let init_positions: Option<Vec<usize>> = init_tour.and_then(|tour| {
        let positions: Option<Vec<usize>> = tour
            .iter()
            .map(|&id| distances.city_id2pos(id).map(Position::into_inner))
            .collect();
        positions.filter(|positions| is_permutation(positions, n))
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
            if let Some(tx) = progress_tx {
                let _ = tx.send(ProgressMessage::Done);
            }
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
        let mut generation_cycles = 0usize;
        let mut generation_components = 0usize;
        let mut generation_bridges = 0usize;
        for (a_idx, b_idx) in pairs {
            let mut best_child: Option<Member> = None;
            for _ in 0..opts.children_per_pair {
                if let Some((child, stats)) = make_child(
                    &population[a_idx],
                    &population[b_idx],
                    opts,
                    hybrid_as_single_ab,
                    &candidates,
                    distances,
                    &mut rng,
                    &mut scratch,
                ) {
                    generation_cycles += stats.cycles;
                    generation_components = generation_components.max(stats.components);
                    generation_bridges += stats.bridges;
                    if best_child
                        .as_ref()
                        .is_none_or(|current| child.cost < current.cost)
                    {
                        best_child = Some(child);
                    }
                }
            }
            if let Some(child) = best_child
                && should_replace(population[a_idx].cost, child.cost)
            {
                population[a_idx] = child;
            }
        }

        let current_best = population
            .iter()
            .min_by(|a, b| a.cost.total_cmp(&b.cost))
            .expect("population is never empty");
        if should_replace(best_cost, current_best.cost) {
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

        tracing::debug!(
            epoch,
            best = best_cost,
            stagnant,
            cycles = generation_cycles,
            components = generation_components,
            bridges = generation_bridges,
            "memetic_eax: generation"
        );
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

    fn tour_cost(tour: &[usize], distances: &dyn DistanceSource) -> f32 {
        let n = tour.len();
        (0..n)
            .map(|i| distance(distances, tour[i], tour[(i + 1) % n]))
            .sum()
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
        for city in 0..child.len() {
            let slots = child.neighbours(city);
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
        let mut graph =
            TourGraph::from_neighbours(vec![[1, 2], [0, 2], [0, 1], [4, 5], [3, 5], [3, 4]]);
        assert!(!graph.is_single_cycle());
        let cities = points(6);
        let distances = distance_matrix::from_cities_unrounded(&cities);
        let candidates = CandidateList::build(&cities, 5).unwrap();
        let mut scratch = Scratch::new(6);
        component_map(
            &graph,
            &mut scratch.comp_of,
            &mut scratch.comp_order,
            &mut scratch.comp_start,
        );
        assert_eq!(scratch.comp_start.len() - 1, 2);
        let small_vertices =
            scratch.comp_order[scratch.comp_start[0]..scratch.comp_start[1]].to_vec();
        merge_via_candidates(
            &mut graph,
            &scratch.comp_of,
            0,
            &small_vertices,
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
        let mut tour = vec![0usize, 1, 3, 2];
        let before = tour_cost(&tour, &distances);
        let mut scratch = Scratch::new(4);
        let all: Vec<usize> = (0..4).collect();
        polish(&mut tour, &all, &candidates, &distances, &mut scratch);
        let after = tour_cost(&tour, &distances);
        assert!(after < before, "polish must remove the crossing");
        assert!(TourGraph::from_tour(&tour).is_single_cycle());
    }

    #[test]
    fn polish_never_increases_cost() {
        let cities = points(12);
        let distances = distance_matrix::from_cities_unrounded(&cities);
        let candidates = CandidateList::build(&cities, 4).unwrap();
        let mut tour: Vec<usize> = (0..12).collect();
        let before = tour_cost(&tour, &distances);
        let mut scratch = Scratch::new(12);
        let all: Vec<usize> = (0..12).collect();
        polish(&mut tour, &all, &candidates, &distances, &mut scratch);
        assert!(tour_cost(&tour, &distances) <= before + EPS);
        assert!(
            TourGraph::from_tour(&tour).is_single_cycle(),
            "polish must keep one cycle"
        );
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
            let pairs = pair_order(n, &mut rng);
            assert_eq!(pairs.len(), if n < 2 { 0 } else { n });
            for &(a, b) in &pairs {
                assert_ne!(a, b, "self-pair in population of {n}");
                assert!(a < n && b < n);
            }
            if n >= 2 {
                let mut parents: Vec<usize> = pairs.iter().map(|&(a, _)| a).collect();
                parents.sort_unstable();
                assert_eq!(
                    parents,
                    (0..n).collect::<Vec<_>>(),
                    "every member must be parent A exactly once in a population of {n}"
                );
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
        opts.epochs = 0;
        assert!(!should_stop(1_000_000, &opts, 5));
        assert!(should_stop(1_000_000, &opts, 6));
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
        if let Some((child, _)) = child {
            assert!(child.graph.is_single_cycle());
        }
    }

    #[test]
    fn ab_cycles_random_parent_pairs_are_even_and_alternating() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(11);
        for n in 6..=12 {
            for _ in 0..20 {
                let mut a: Vec<usize> = (0..n).collect();
                a.shuffle(&mut rng);
                let mut b: Vec<usize> = (0..n).collect();
                b.shuffle(&mut rng);
                let ga = TourGraph::from_tour(&a);
                let gb = TourGraph::from_tour(&b);
                let cycles = ab_cycles(&ga, &gb).expect("random parents must decompose");
                for cycle in &cycles {
                    assert!(cycle.vertices.len() >= 4);
                    assert!(cycle.vertices.len().is_multiple_of(2));
                    for i in 0..cycle.vertices.len() {
                        let from = cycle.vertices[i];
                        let to = cycle.vertices[(i + 1) % cycle.vertices.len()];
                        assert_ne!(
                            ga.has_edge(from, to),
                            gb.has_edge(from, to),
                            "cycle edge must be A-only or B-only, never shared"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn make_child_with_identical_parents_returns_some() {
        let problem = problem(points(10));
        let distances = problem.distances.as_ref();
        let candidates = CandidateList::build(&problem.cities, 4).unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(5);
        let a = Member::new(
            TourGraph::from_tour(&(0..10).collect::<Vec<_>>()),
            distances,
        );
        let b = a.clone();
        let mut scratch = Scratch::new(10);
        let (child, stats) = make_child(
            &a,
            &b,
            &options(),
            false,
            &candidates,
            distances,
            &mut rng,
            &mut scratch,
        )
        .expect("identical parents must perturb, not skip");
        assert_eq!(stats.cycles, 0);
        assert!(child.graph.is_single_cycle());
    }

    #[test]
    fn make_child_single_ab_returns_some_and_single_cycle() {
        let problem = problem(points(10));
        let distances = problem.distances.as_ref();
        let candidates = CandidateList::build(&problem.cities, 5).unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(6);
        let a_tour: Vec<usize> = (0..10).collect();
        let mut b_tour = a_tour.clone();
        b_tour[1..=4].reverse(); // one 2-opt move: a single AB cycle
        let a = Member::new(TourGraph::from_tour(&a_tour), distances);
        let b = Member::new(TourGraph::from_tour(&b_tour), distances);
        let mut opts = options();
        opts.strategy = EaxStrategy::SingleAb;
        let mut scratch = Scratch::new(10);
        let (child, stats) = make_child(
            &a,
            &b,
            &opts,
            false,
            &candidates,
            distances,
            &mut rng,
            &mut scratch,
        )
        .expect("a single AB cycle must produce a child, not None");
        assert_eq!(stats.cycles, 1);
        assert!(child.graph.is_single_cycle());
    }

    #[test]
    fn population_fills_when_fewer_optima_than_pop_size() {
        let problem = problem(points(4));
        let distances = problem.distances.as_ref();
        let candidates = CandidateList::build(&problem.cities, 3).unwrap();
        let mut opts = options();
        opts.pop_size = 20;
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let members = initial_population(
            4,
            &opts,
            &candidates,
            distances,
            None,
            &mut Scratch::new(4),
            &mut rng,
        );
        assert_eq!(members.len(), 20);
        for member in &members {
            assert!(member.graph.is_single_cycle());
        }
    }

    #[test]
    fn initial_population_clamps_zero_and_one() {
        let problem = problem(points(6));
        let distances = problem.distances.as_ref();
        let candidates = CandidateList::build(&problem.cities, 3).unwrap();
        for pop_size in [0usize, 1] {
            let mut opts = options();
            opts.pop_size = pop_size;
            let mut rng = rand::rngs::StdRng::seed_from_u64(8);
            let members = initial_population(
                6,
                &opts,
                &candidates,
                distances,
                None,
                &mut Scratch::new(6),
                &mut rng,
            );
            assert_eq!(members.len(), 2);
        }
    }

    #[test]
    fn candidate_list_clamps_to_n_minus_one() {
        let cities = points(6);
        let distances = distance_matrix::from_cities_unrounded(&cities);
        let candidates = build_candidates(&distances, &cities, 100).unwrap();
        for city in 0..6 {
            assert!(
                candidates.neighbours_by_pos(Position::new(city)).len() <= 5,
                "candidate row must be capped at n - 1"
            );
        }
    }

    #[test]
    fn full_scan_fallback_merges_components_when_candidates_do_not_cross() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![100.0, 100.0],
            vec![101.0, 100.0],
            vec![100.0, 101.0],
        ]);
        let distances = distance_matrix::from_cities_unrounded(&cities);
        let candidates = CandidateList::build(&cities, 1).unwrap();
        let mut graph =
            TourGraph::from_neighbours(vec![[1, 2], [0, 2], [0, 1], [4, 5], [3, 5], [3, 4]]);
        let mut scratch = Scratch::new(6);
        component_map(
            &graph,
            &mut scratch.comp_of,
            &mut scratch.comp_order,
            &mut scratch.comp_start,
        );
        assert_eq!(scratch.comp_start.len() - 1, 2);
        let small_vertices =
            scratch.comp_order[scratch.comp_start[0]..scratch.comp_start[1]].to_vec();
        assert!(
            !merge_via_candidates(
                &mut graph,
                &scratch.comp_of,
                0,
                &small_vertices,
                &candidates,
                &distances,
                &mut scratch.added,
            ),
            "each city's only candidate is inside its own component"
        );
        merge_via_full_scan(
            &mut graph,
            &scratch.comp_of,
            0,
            &small_vertices,
            &distances,
            &mut scratch.added,
        );
        assert!(graph.is_single_cycle());
    }

    #[test]
    fn polish_improves_from_dirty_vertices_only() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
        ]);
        let distances = distance_matrix::from_cities_unrounded(&cities);
        let candidates = CandidateList::build(&cities, 3).unwrap();
        let mut tour = vec![0usize, 1, 3, 2];
        let before = tour_cost(&tour, &distances);
        let mut scratch = Scratch::new(4);
        let improved = polish(&mut tour, &[0, 3], &candidates, &distances, &mut scratch);
        assert!(improved, "dirty vertices must be enough to find the move");
        assert!(tour_cost(&tour, &distances) < before);
        assert!(TourGraph::from_tour(&tour).is_single_cycle());
    }

    #[test]
    fn should_replace_is_strict() {
        assert!(should_replace(10.0, 9.0));
        assert!(!should_replace(10.0, 10.0));
        assert!(!should_replace(10.0, 10.0 - EPS / 2.0));
    }

    #[test]
    fn solve_clamps_zero_options() {
        let problem = problem(points(8));
        let opts = MemeticEaxOptions {
            epochs: 2,
            pop_size: 0,
            children_per_pair: 0,
            strategy: EaxStrategy::Hybrid,
            stall_epochs: 0,
            candidate_k: 0,
            seed: Some(12),
        };
        let solution = solve(&problem, &opts, None, None);
        let mut ids = solution.route().to_vec();
        ids.sort_unstable();
        assert_eq!(ids, (0..8).collect::<Vec<_>>());
    }

    #[test]
    fn solve_filters_invalid_tiny_seed() {
        let problem = problem(points(3));
        let invalid = vec![0usize, 0, 1]; // duplicate 0, missing 2
        let solution = solve(&problem, &options(), None, Some(&invalid));
        assert_eq!(solution.route(), &[0, 1, 2]);
    }

    #[test]
    fn generation_events_are_emitted() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tracing::span::Id;

        struct Counter(Arc<AtomicUsize>);
        impl tracing::Subscriber for Counter {
            fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
                true
            }
            fn new_span(&self, _: &tracing::span::Attributes<'_>) -> Id {
                Id::from_u64(1)
            }
            fn record(&self, _: &Id, _: &tracing::span::Record<'_>) {}
            fn record_follows_from(&self, _: &Id, _: &Id) {}
            fn event(&self, _: &tracing::Event<'_>) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
            fn enter(&self, _: &Id) {}
            fn exit(&self, _: &Id) {}
        }

        let count = Arc::new(AtomicUsize::new(0));
        tracing::subscriber::with_default(Counter(count.clone()), || {
            let problem = problem(points(8));
            let opts = MemeticEaxOptions {
                epochs: 2,
                pop_size: 4,
                children_per_pair: 2,
                stall_epochs: 1,
                seed: Some(1),
                ..MemeticEaxOptions::default()
            };
            let _ = solve(&problem, &opts, None, None);
        });
        assert!(count.load(Ordering::SeqCst) > 0, "no tracing event emitted");
    }

    #[test]
    fn options_validate_requires_two_parents() {
        let mut opts = MemeticEaxOptions::default();
        assert!(opts.validate().is_ok());
        opts.pop_size = 1;
        assert!(opts.validate().is_err());
        opts.pop_size = 2;
        assert!(opts.validate().is_ok());
    }

    #[test]
    fn options_from_toml_parses_every_field() {
        let table: toml::Table = toml::from_str(
            "epochs = 42\npop_size = 12\nchildren_per_pair = 4\nstrategy = \"rand\"\nstall_epochs = 7\ncandidate_k = 9\nseed = 99",
        )
        .unwrap();
        let opts = MemeticEaxOptions::from_toml(&table).unwrap();
        assert_eq!(opts.epochs, 42);
        assert_eq!(opts.pop_size, 12);
        assert_eq!(opts.children_per_pair, 4);
        assert_eq!(opts.strategy, EaxStrategy::Rand);
        assert_eq!(opts.stall_epochs, 7);
        assert_eq!(opts.candidate_k, 9);
        assert_eq!(opts.seed, Some(99));
    }

    #[test]
    fn options_from_toml_rejects_unknown_field_and_bad_strategy() {
        let unknown: toml::Table = toml::from_str("bogus = 1").unwrap();
        assert!(MemeticEaxOptions::from_toml(&unknown).is_err());
        let bad: toml::Table = toml::from_str("strategy = \"turbo\"").unwrap();
        assert!(MemeticEaxOptions::from_toml(&bad).is_err());
    }

    fn cli_command() -> clap::Command {
        use clap::{Arg, ArgAction, Command};
        Command::new("test").args([
            Arg::new("epochs").long("epochs").action(ArgAction::Set),
            Arg::new("pop_size").long("pop-size").action(ArgAction::Set),
            Arg::new("children_per_pair")
                .long("children-per-pair")
                .action(ArgAction::Set),
            Arg::new("strategy").long("strategy").action(ArgAction::Set),
            Arg::new("stall_epochs")
                .long("stall-epochs")
                .action(ArgAction::Set),
            Arg::new("candidate_k")
                .long("candidate-k")
                .action(ArgAction::Set),
            Arg::new("seed").long("seed").action(ArgAction::Set),
            Arg::new("stagnation_epochs")
                .long("stagnation_epochs")
                .action(ArgAction::Set),
            Arg::new("n_nearest")
                .long("n_nearest")
                .action(ArgAction::Set),
        ])
    }

    #[test]
    fn options_from_cli_falls_back_to_the_shared_flags() {
        let args = cli_command().get_matches_from([
            "test",
            "--stagnation_epochs",
            "20",
            "--n_nearest",
            "7",
        ]);
        let opts = MemeticEaxOptions::from_cli(&args).unwrap();
        assert_eq!(
            opts.stall_epochs, 10,
            "20 shared epochs map to EAX's 2× stop"
        );
        assert_eq!(opts.candidate_k, 7);
    }

    #[test]
    fn options_from_cli_zero_stagnation_keeps_the_default() {
        let args = cli_command().get_matches_from(["test", "--stagnation_epochs", "0"]);
        let opts = MemeticEaxOptions::from_cli(&args).unwrap();
        assert_eq!(
            opts.stall_epochs,
            MemeticEaxOptions::default().stall_epochs,
            "0 means no override for EAX"
        );
    }

    #[test]
    fn options_from_cli_specific_flags_win_over_the_shared_ones() {
        let args = cli_command().get_matches_from([
            "test",
            "--stall-epochs",
            "9",
            "--stagnation_epochs",
            "20",
            "--candidate-k",
            "11",
            "--n_nearest",
            "7",
        ]);
        let opts = MemeticEaxOptions::from_cli(&args).unwrap();
        assert_eq!(opts.stall_epochs, 9);
        assert_eq!(opts.candidate_k, 11);
    }

    #[test]
    fn options_from_cli_parses_every_field() {
        let args = cli_command().get_matches_from([
            "test",
            "--epochs",
            "42",
            "--pop-size",
            "12",
            "--children-per-pair",
            "4",
            "--strategy",
            "single_ab",
            "--stall-epochs",
            "7",
            "--candidate-k",
            "9",
            "--seed",
            "99",
        ]);
        let opts = MemeticEaxOptions::from_cli(&args).unwrap();
        assert_eq!(opts.epochs, 42);
        assert_eq!(opts.pop_size, 12);
        assert_eq!(opts.children_per_pair, 4);
        assert_eq!(opts.strategy, EaxStrategy::SingleAb);
        assert_eq!(opts.stall_epochs, 7);
        assert_eq!(opts.candidate_k, 9);
        assert_eq!(opts.seed, Some(99));
    }

    #[test]
    fn options_from_cli_rejects_invalid_values() {
        let args = cli_command().get_matches_from(["test", "--pop-size", "zero"]);
        assert!(MemeticEaxOptions::from_cli(&args).is_err());
        let args = cli_command().get_matches_from(["test", "--strategy", "turbo"]);
        assert!(MemeticEaxOptions::from_cli(&args).is_err());
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

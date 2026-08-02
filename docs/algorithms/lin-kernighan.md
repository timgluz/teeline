---
id: "lk"
name: "Lin-Kernighan ILS"
typeBadge: "Heuristic — iterated local search"
description: "Iterated Local Search (ILS) built around a candidate-list 2-opt move with the Lin-Kernighan gain criterion."
hasExplainer: true
---

# Lin-Kernighan ILS

| | |
| --- | --- |
| **Alias** | `lk`, `lin_kernighan` |
| **Type** | Heuristic — iterated local search |
| **Auto-seeds from** | `nn` (nearest neighbor) |

## Description

Iterated Local Search (ILS) built around a candidate-list 2-opt move with the Lin-Kernighan gain criterion. Each iteration of the inner loop scans every edge of the tour and tests replacements restricted to a pre-built candidate list of the `k` nearest neighbours. The LK gain bound short-circuits the search: when the cheapest candidate edge already costs more than the edge being removed, no profitable swap exists further down the sorted list, so the scan stops early. The inner loop repeats until no improving move remains (a local optimum is reached).

Once the inner optimizer stalls, a **double-bridge** perturbation kicks the tour out of the current basin of attraction. Double-bridge is a non-sequential 4-opt move that splits the tour at four random cut points and reconnects the four segments in a different order — the resulting tour cannot be reached by any 2-opt or 3-opt move, so it provides a genuinely different starting point for the next optimization pass. The best tour seen across all restarts is kept; a configurable plateau counter terminates early if no improvement is found for `platoo_epochs` consecutive restarts.

Auto-expands to `pipeline(nn, lk)`: the nearest-neighbour tour provides a low-cost starting point, avoiding the wasted restarts that a random seed would require.

```text
procedure LinKernighan(cities, epochs):
    tour ← nearest_neighbor(cities)
    best ← tour
    for epoch in 1..epochs:
        tour ← two_opt_with_candidate_list(tour)
        if length(tour) < length(best):
            best ← tour
        tour ← double_bridge_kick(best)   // escape local optimum
    return best
```

## Options

| Field | CLI flag | Default | Range | Description |
| ------- | ------- | --------- | ------- | ------------- |
| `epochs` | `--epochs` | 100 | ≥ 0 (unvalidated) | Number of ILS restarts |
| `platoo_epochs` | `--platoo_epochs` | 10 | ≥ 0 (unvalidated) | Stop after this many consecutive non-improving restarts |
| `n_nearest` | `--n_nearest` | 5 | ≥ 1 | Candidate list size (k nearest neighbours per city) |
| `max_depth` | `--max-depth` | 5 | ≥ 1 | LK chain-search depth; depth-1 ≈ 2-opt, depth-5 enables the full k-opt move space |

`epochs` and `platoo_epochs` aren't rejected by validation at any value, but unlike the
"0 = run forever" convention some other solvers use, LK's loop doesn't implement that
convention: `epochs=0` runs zero ILS restarts (only the initial pass), and
`platoo_epochs=0` stops after the first non-improving restart — the opposite of
"unlimited". All four fields are also reachable via the REST API's `configs.lk` or a
`[lk]`/`[stage.lk]` TOML table (field names match the table above).

## Usage

```bash
# auto-expands to pipeline(nn, lk)
teeline solve lk -i ./data/tsplib/berlin52.tsp

# verbose output (prints tour distance each improvement)
teeline solve lk -i ./data/tsplib/berlin52.tsp --verbose

# skip NN seeding — start from input city order
teeline solve lk --no-seed -i ./data/tsplib/berlin52.tsp

# wider candidate list and longer run
teeline solve lk -i ./data/tsplib/berlin52.tsp --n_nearest=5 --epochs=50000
```

Per-stage TOML config (via `pipeline --config`):

```toml
[[stage]]
solver = "lk"

[stage.lk]
max_depth = 3
n_nearest = 8
```

## Benchmark

| Instance | Optimal | This solver (default) | Gap |
| --- | --- | --- | --- |
| berlin52 | 7 544.37 | 7 544.37 | 0.0% |

With the default `max_depth=5` (full LK chain-search depth), this solver finds the
optimal berlin52 tour. Depth matters: over 10 runs, depth-1 (≈ 2-opt with ILS restarts)
hits optimal 6/10 times (mean 7595), depth-2 hits 7/10 (mean 7580), and depth-5 hits
10/10 (mean 7544) — see `docs/benchmarks.md` for the full breakdown. Full sequential
LK chain search (issue #184) is implemented and shipped, not a future improvement.

## Notes

- **Candidate-list construction uses a KD-tree**: `build_candidates()` builds the
  per-city `k`-nearest-neighbour list with a `KDTree::nearest` query per city
  (`src/tsp/kdtree.rs`, same module used by `branch_bound.rs` and `fourier.rs`) instead
  of a brute-force all-pairs distance scan + sort. This changes the construction cost
  from O(n² log n) to O(n log n): a one-time tree build (O(n log n)) followed by `n`
  queries, each an O(log n) tree descent plus O(k) buffer maintenance (the buffer is
  a `k`-capacity `Vec` that's re-sorted on every qualifying insert, not a bounded
  insert/heap, so the constant factor is larger than the O(k) term alone suggests —
  negligible at the small `k` this solver uses by default). Unlike `fourier.rs`'s
  KD-tree usage — which rebuilds the
  tree on every gradient step and so only realizes a fraction of the theoretical speedup
  (see `docs/algorithms/fourier.md`) — LK builds the tree once per `solve()` call, before
  the ILS loop, so the full asymptotic win is realized *for that step*. Both the KD-tree
  query and the prior brute-force scan route through the same `KDPoint::distance`
  (`f32` Euclidean) function, so candidate selection is numerically identical except for
  genuine equidistant ties, where the two algorithms may pick a different (equally
  valid) member of the tied set — checked by a one-off manual diff of candidate lists
  computed for berlin52 (exact match), a280, and pr1002 against the pre-swap
  brute-force implementation before it was deleted; every differing entry corresponded
  to cities at provably equal exact distance (e.g. pr1002 cities 11 and 17, both at
  distance 403.1129 from city 13). This comparison isn't a committed regression test —
  the brute-force implementation no longer exists in the tree to compare against.
  Isolated timing for the current (KD-tree) implementation, measured with
  `cargo test --release -- --ignored bench_build_candidates` (seeded random 2D points,
  k=5): n=52 → 0.14ms, n=280 → 0.69ms, n=1002 → 2.71ms, n=5000 → 14.3ms — run it
  yourself to reproduce, exact figures vary by machine. These are consistent with the
  pre-swap brute-force numbers quoted in the commit message (178.5ms at n=1002, 4.75s
  at n=5000, a 34x/217x speedup at those sizes) despite not being bit-identical to
  them: that earlier comparison used a different (unseeded) random point generator and
  an implementation that no longer exists in the tree to re-run head-to-head. In the
  context of a full `solve()`
  call, this step is a small fraction of total wall time (100 ILS epochs of candidate-
  restricted 2-opt dominate), so the end-to-end wall-clock difference is within
  run-to-run noise at the instance sizes and epoch counts this solver is typically run
  at (LK's ILS loop uses an unseeded RNG and a plateau-based early exit, both of which
  add their own run-to-run variance independent of this change) — the win is real and
  compounds for larger instances or lower epoch counts, where the one-time candidate-
  list cost is a larger share of total work.

## References

- Lin, S. & Kernighan, B. W. (1973) — "An Effective Heuristic Algorithm for the Traveling-Salesman Problem", *Operations Research*, **21**(2), 498–516. DOI: 10.1287/opre.21.2.498
  (source of the gain criterion and candidate-list restriction used in the inner optimizer)
- [4-opt — Double bridge move (Wikipedia)](https://en.wikipedia.org/wiki/4-opt#Double-bridge_move)
- [Iterated Local Search (Wikipedia)](https://en.wikipedia.org/wiki/Iterated_local_search)

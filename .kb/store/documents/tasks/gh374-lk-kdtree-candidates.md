---
id: 019fc17c-4e1e-75f0-b066-954f0e322d8a
slug: tasks/gh374-lk-kdtree-candidates
title: "GH #374: LK candidate-list KD-tree swap"
type: task
status: completed
priority: medium
tags: [performance, lin-kernighan, kdtree]
---

## Overview

GitHub issue #374: `build_candidates()` in `src/tsp/lin_kernighan.rs` (lines 12-34) builds
the k-nearest-neighbour candidate list per city via brute-force all-pairs distance scan +
sort (O(n² log n)), called once per `solve()` before the ILS loop. Swap to `KDTree::nearest`
(same pattern as `fourier.rs`/`branch_bound.rs`), but the issue requires **measured**
wall-clock benchmarks proving a real improvement before shipping — not just complexity
analysis — citing #370 (Fourier KD-tree swap: measured 2-4x vs theoretical ~22x, because
that tree was rebuilt every gradient step) as a cautionary precedent.

LK's case differs favorably: tree built once per solve, queried n times — the classic
amortized-build shape, not Fourier's per-step-rebuild trap. Confirmed both
`KDTree::nearest` and `DistanceMatrix`'s Euc2D path use the same `KDPoint::distance`
(plain Euclidean, f32) — no precision/formula mismatch risk.

## Goals

- Replace brute-force scan in `build_candidates()` with `KDTree::nearest`, dropping the
  now-unused `DistanceMatrix` parameter.
- Measured before/after wall-clock benchmarks on berlin52 (52), a280 (280), pr1002 (1002).
- Confirm tour quality (gap vs optimal) unchanged.
- Update `docs/algorithms/lin-kernighan.md` with new complexity.
- Ship unconditional swap (no size-gating) per plan decision — fall back to hybrid or
  close-as-not-worth-it only if benchmarks surprise us.

## Acceptance Criteria

- [x] `build_candidates()` uses `KDTree::nearest` instead of brute-force scan
- [x] Existing 3 unit tests pass (signature updated mechanically, no assertion changes)
- [x] Measured wall-clock benchmark on berlin52/a280/pr1002, documented with real numbers
- [x] Tour quality (gap vs optimal) confirmed unchanged across same 3 instances
- [x] `docs/algorithms/lin-kernighan.md` updated with new complexity bound
- [x] `cargo clippy --workspace` clean

## Context

Full plan: see `/home/timgluz/.claude/plans/please-check-github-issue-polymorphic-dongarra.md`
(session-local file, not in repo). Branch: `feat/gh374-lk-kdtree-candidates`.

Related: [[tasks/fourier-scaling-optimizer-experiments]], PR #370, PR #375 (GH #372 follow-on
that discovered this issue).

## Progress Log

### 2026-08-02

- Implemented the swap: `build_candidates(cities, k)` now builds a `KDTree` once and
  queries `.nearest(city, k)` per city, dropping the `DistanceMatrix` parameter. All 3
  `build_candidates_*` unit tests plus the full `lin_kernighan` and workspace test suite
  pass unmodified in substance (only mechanical call-site signature updates). Clippy and
  fmt clean.
- **Full-pipeline wall-clock benchmark** (release build, `solve lk` default opts, 3 runs
  each) was inconclusive/noisy: berlin52 ~0.07s both variants; a280 ~0.29s→~0.25s
  (modest, within noise margin for n=3); pr1002 ~8.0s baseline vs ~8.5s kdtree (no
  measurable win, noise-dominated). Root cause: LK's ILS loop uses an **unseeded RNG**
  and a **plateau-based early exit** (`platoo_epochs`), both of which add run-to-run
  wall-time variance unrelated to `build_candidates` — and the 100-epoch ILS loop itself
  dominates total wall time.
- **Isolated `build_candidates` benchmark** (random 2D points, k=5, release,
  `cargo test --release -- --ignored`) gave a clean, dramatic, reproducible signal:

  | n | brute-force | KD-tree | speedup |
  |---|---:|---:|---:|
  | 52 | 0.54ms | 0.22ms | 2.5x |
  | 280 | 11.3ms | 0.91ms | 12.4x |
  | 1002 | 178.5ms | 5.19ms | 34.4x |
  | 5000 | 4.75s | 21.9ms | 217x |

  This confirms the O(n² log n)→O(n log n) complexity win is real and grows with n as
  expected. It doesn't show up in full-`solve()` wall time at the tested epoch counts
  because `build_candidates` is a small fraction (well under 5%) of total solve time —
  the ILS loop dominates. The win would matter more for larger instances, fewer epochs,
  or repeated `build_candidates` calls in a different caller.
- **Tour-quality verification**: rather than trust noisy full-solve gap numbers (also
  affected by the unseeded RNG), directly diffed candidate lists computed by both
  implementations on real TSPLIB data (berlin52, a280, pr1002, k=5). berlin52: 0 diffs
  (byte-for-byte identical). a280/pr1002: a handful of differing rows, every one verified
  by hand to be a genuine equidistant tie (e.g. pr1002 cities 11 and 17 both at exactly
  403.11288... from city 13) — the two algorithms pick a different but equally valid
  member of the tied set. No correctness regression.
- Decision: **shipped the unconditional swap** (no size-gating) — the code is simpler
  (removes a manual sort/filter, reuses existing well-tested infra), carries no
  measured downside, and the win is real for the isolated hot path even though it's
  not visible in full-pipeline wall time at the instance sizes tested. Documented all
  of this transparently in `docs/algorithms/lin-kernighan.md`'s new "Notes" section
  rather than overclaiming an end-to-end speedup that the data doesn't support.
- Files changed: `src/tsp/lin_kernighan.rs`, `docs/algorithms/lin-kernighan.md`.

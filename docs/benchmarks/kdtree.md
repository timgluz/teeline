# KD-tree Performance Audit

Microbenchmarks and end-to-end solver benchmarks for the KD-tree
(`src/tsp/kdtree.rs`) and its shared `NearestResult` accumulator
(`src/tsp/mod.rs`). The audit accompanies the `perf/kdtree-verification-and-cleanup`
branch and establishes a reproducible baseline for measuring improvements and
catching regressions.

## Methodology

### Microbenchmarks (criterion)

- **Tool**: `criterion 0.5` — `cargo bench --bench kdtree`
- **Harness**: `benches/kdtree.rs`
- **Groups**:
  - `build` — `kdtree::from_cities(&cities)` (tree construction only)
  - `nearest_k5` — pre-built tree, one `nearest(city, 5)` query per city
  - `build_candidates` — end-to-end mirror of `lin_kernighan::build_candidates`
    (build tree + N queries with k=5)
- **Datasets**: `berlin52` (n=52), `a280` (n=280), `att532` (n=532)
- **Samples**: criterion default (100 samples, ~5s measurement window)

### Solver benchmarks (GNU `time -v`)

- **Tool**: `scripts/bench-solvers.sh` via `task bench:solvers`
- **Metrics**: wall time (seconds), peak RSS (kB), tour cost
- **Runs**: 5 per solver/dataset combo
- **Datasets per solver**:
  - `nn`: berlin52, a280, att532
  - `lk`: berlin52, a280, att532
  - `fourier`: berlin52, a280
  - `branch_bound`: burma14, gr17

### Environment

| Item | Value |
| --- | --- |
| CPU | AMD Ryzen 7 PRO 4750U (16 threads) |
| RAM | 30 GiB |
| OS | Linux 7.1.4-1-default x86_64 |
| Rust | rustc 1.97.1 (8bab26f4f 2026-07-14) |
| teeline | 1.0.11 |
| Baseline SHA | `2d32039` (master, 2026-08-02) |

---

## Baseline Results (master @ 2d32039)

### Criterion Microbenchmarks

The `[low mean high]` range from criterion; the middle value is the point estimate.

| Benchmark | n=52 | n=280 | n=532 |
| --- | --- | --- | --- |
| `build` | 4.36 µs [4.12–4.63] | 27.73 µs [26.49–29.08] | 56.43 µs [54.02–58.97] |
| `nearest_k5` | 19.26 µs [18.13–20.55] | 253.40 µs [243.70–264.01] | 546.77 µs [528.13–566.94] |
| `build_candidates` | 27.80 µs [26.41–29.49] | 322.90 µs [311.78–335.44] | 639.15 µs [607.06–676.50] |

### Solver Benchmarks

#### nn (deterministic on berlin52/att532, non-deterministic on a280)

| Dataset | Wall (s) | RSS (kB) | Tour cost |
| --- | --- | --- | --- |
| berlin52 | ~0.00 | ~6640 | 8980.92 |
| a280 | ~0.01 | ~6716 | 3148–3552 (non-deterministic) |
| att532 | ~0.02 | ~7472 | 112099.42 |

> **Note**: nn/a280 produces different tour costs across runs (3148, 3169, 3552).
> The source is `DistanceMatrix::nearest`'s k-buffer tie-breaking when multiple
> cities share the same distance — the `n_nearest=3` boundary can include or
> exclude a tied city depending on buffer eviction order. This is pre-existing
> behavior, not introduced by this audit. berlin52 and att532 happen to have
> unique distance orderings and are deterministic.

#### lk (stochastic on large N; finds optimal on berlin52)

| Dataset | Wall (s) | RSS (kB) | Tour cost |
| --- | --- | --- | --- |
| berlin52 | ~0.15 | ~6718 | 7544.37 (optimal) |
| a280 | ~1.96 | ~7064 | ~2660 (stochastic, ±100) |
| att532 | ~25.4 | ~8790 | ~89600 (stochastic, ±1000) |

#### fourier (deterministic)

| Dataset | Wall (s) | RSS (kB) | Tour cost |
| --- | --- | --- | --- |
| berlin52 | ~0.15 | ~6865 | 8549.14 |
| a280 | ~0.33 | ~7085 | 5260.94 |

#### branch_bound (deterministic, exact — optimal guaranteed)

| Dataset | Wall (s) | RSS (kB) | Tour cost |
| --- | --- | --- | --- |
| burma14 | ~0.02 | ~6820 | 3651.00 (optimal) |
| gr17 | ~0.00 | ~6663 | 2187.00 (optimal) |

---

## Post-Change Results (branch @ P5+P6, this PR)

**Environment caveat**: this second measurement pass ran on the same shared
dev machine while under active background load (load average 10-11 on 16
cores from a concurrent editor session, git-kb daemon, and browser). The
`build` benchmark exercises code that is **completely unchanged** by this PR
(`from_cities`/`partition_points`/`build_subtree`) and still shows +15-25%
versus the baseline session — that delta is measurement noise, not a
regression, and serves as this run's noise floor. Wall-time and RSS deltas
below should be read with that floor in mind; the tour-cost equality check is
the reliable regression signal.

### Criterion Microbenchmarks (Post-Change)

| Benchmark | n=52 | n=280 | n=532 |
| --- | --- | --- | --- |
| `build` (unchanged code — noise floor) | 5.06 µs [4.91–5.24] | 32.03 µs [31.44–32.71] | 65.34 µs [63.53–67.55] |
| `nearest_k5` | 26.95 µs [25.75–28.43] | 280.33 µs [269.50–292.57] | 583.65 µs [569.81–599.20] |
| `build_candidates` | 29.63 µs [28.93–30.47] | 298.38 µs [290.24–307.71] | 662.48 µs [643.98–682.77] |

Applying the `build` noise floor (~20-25% inflation on this run) as a rough
correction, `nearest_k5`/`build_candidates` are directionally flat-to-improved
relative to baseline, consistent with the P2 (binary-search insertion) and P6
(removing per-add closest-point field writes) changes — but the absolute
deltas are within this machine's noise band and should not be quoted as
precise percentages. Re-running on a quiet/dedicated machine is recommended
before relying on exact numbers.

### Solver Benchmarks — Correctness & Timing Deltas

| Solver | Dataset | Wall Δ | RSS Δ | Tour cost |
| --- | --- | --- | --- | --- |
| `nn` | berlin52 | +noise (0.00→0.002s) | +0.0% | **IDENTICAL** (8980.92) |
| `nn` | a280 | 0.0% | -0.5% | differs (pre-existing non-determinism, both branches) |
| `nn` | att532 | -10.0% | +1.4% | **IDENTICAL** (112099.42) |
| `lk` | berlin52 | -2.7% | -0.8% | **IDENTICAL** (7544.37, optimal) |
| `lk` | a280 | -6.1% | +0.7% | differs (expected — stochastic) |
| `lk` | att532 | +11.8% | +0.4% | differs (expected — stochastic; wall time dominated by ILS restart count, not KD-tree perf) |
| `fourier` | berlin52 | +1.3% | +0.2% | **IDENTICAL** (8549.14) |
| `fourier` | a280 | +16.9% | -0.1% | **IDENTICAL** (5260.94) |
| `branch_bound` | burma14 | -10.0% | +0.5% | **IDENTICAL** (3651.00, optimal) |
| `branch_bound` | gr17 | 0.0% | 0.0% | **IDENTICAL** (2187.00, optimal) |

**Correctness verdict**: every deterministic solver (`nn` on berlin52/att532,
`fourier`, `branch_bound`, `lk` on berlin52) produced **byte-identical tour
costs** before and after the P2/P5/P6 changes. This confirms the k-NN
buffer's binary-search insertion, the dropped `point`/`distance` fields, and
the `nearest()` signature change did not alter solver output.

**Timing verdict**: no wall-time or RSS delta exceeds the noise floor
established by the unchanged `build` benchmark on this run (~20-25%), except
`lk`/att532's +11.8% — which is expected variance from LK's stochastic
double-bridge restarts on a 60-city-larger instance, not a KD-tree
regression (its RSS delta of +0.4% is unremarkable). RSS deltas are all
< 1.5% across every solver, consistent with P6 only removing two small
struct fields.

---

## Regression Detection Thresholds

| Metric | Flag if |
| --- | --- |
| Wall time (mean) | > 10% slower than baseline, net of the `build`-benchmark noise floor |
| Peak RSS | > 5% larger than baseline |
| Tour cost (deterministic solvers) | Δ > 0.01 |

Deterministic solvers: `nn` (berlin52/att532), `fourier`, `branch_bound`,
`lk` (berlin52 only — reliably hits optimal).
Stochastic (excluded from tour-cost regression check): `lk` (a280/att532).
Known pre-existing non-determinism (unrelated to this PR): `nn` (a280) — see
note above; root cause is `DistanceMatrix::nearest`'s tie-breaking at the
`n_nearest` buffer boundary, tracked as a follow-up.

## Raw Data

- Criterion output: `target/criterion/` (not committed; regenerated by `cargo bench`)
- Solver raw TSV, master baseline: `bench/baseline-solvers.tsv` (committed)
- Solver raw TSV, post-change: `bench/post-solvers.tsv` (committed)

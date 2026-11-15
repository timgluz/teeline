# Distance backend benchmark

**Status:** Phase 0 matrix baseline captured. The matrix-vs-on-the-fly comparison is added in
Phase 2; this document is the pre-refactor reference it will be judged against.

## Method

- **Solver comparison:** `scripts/bench-matrix.sh bench/matrix/tier-distance.toml`, five runs
  per cell, medians reported (constitution §2.5). Each cell runs in a fresh process under GNU
  `time -v`; `peak_rss_kb` is the child's maximum resident set size.
- **Memory scaling:** `scripts/bench-distance-scaling.sh`, one fresh process per size. Sizes
  are synthetic deterministic TSPLIB EUC_2D instances (`scripts/gen-synthetic-tsp.py`,
  seed 42); `sfc` is the measured solver.
- **Machine:** AMD Ryzen 7 PRO 4750U, 8 cores / 16 threads, 30 GiB RAM, Linux 7.3.3
  (`x86_64`), rustc 1.100.0, release profile.
- **Binary:** commit `8e562872` (solver code identical to `master` `ae8e5ba2`; the working
  tree was dirty with the Phase 0 benchmark tooling).
- **Fixtures:** `burma14` (GEO), `ulysses16` (GEO), `berlin52` (EUC_2D), `a280` (EUC_2D),
  `att532` (ATT). The large tier uses downloaded `dsj1000`, `pla7397`, `pla33810`.

Raw rows: `bench/baseline/distance/matrix-tier.jsonl` and
`bench/baseline/distance/scaling-matrix.jsonl`. The git-ignored `bench/runs/` holds the same
data plus future runs.

## Solver baseline — matrix backend

All 180 runs were `status: ok`; there were no timeouts or errors. `3opt` is scoped to `a280`
and smaller because it exceeds the local budget on `att532` (measured >60 s, versus `2opt`
0.2 s and `or_opt` 14.5 s). `lk` and `tabu` are non-deterministic; their tour costs vary
across runs, so their medians and distributions are the comparison basis.

| Solver | Fixture | Median wall (s) | Median peak RSS (MB) | Tour cost |
|---|---|---:|---:|---:|
| 2opt | a280 | 0.0663 | 7.1 | 2838 |
| 2opt | att532 | 0.1537 | 8.2 | 31606 |
| 2opt | berlin52 | 0.0213 | 6.8 | 8385 |
| 2opt | burma14 | 0.0147 | 7.1 | 4040 |
| 2opt | ulysses16 | 0.0178 | 7.2 | 7658 |
| 3opt | a280 | 24.9358 | 7.0 | 2695 |
| 3opt | berlin52 | 0.0866 | 6.8 | 7741 |
| 3opt | burma14 | 0.0137 | 7.2 | 3323 |
| 3opt | ulysses16 | 0.0185 | 7.2 | 6859 |
| bhk | burma14 | 0.0258 | 7.6 | 3323 |
| bhk | ulysses16 | 0.0482 | 9.0 | 6859 |
| lk | a280 | 0.9767 | 7.3 | varies |
| lk | att532 | 8.2994 | 8.9 | varies |
| lk | berlin52 | 0.0926 | 6.8 | varies |
| lk | burma14 | 0.0366 | 7.1 | 3543 |
| lk | ulysses16 | 0.0401 | 7.0 | 6983 |
| nn | a280 | 0.0214 | 6.8 | 3157 |
| nn | att532 | 0.0400 | 7.7 | 35516 |
| nn | berlin52 | 0.0144 | 6.9 | 8980 |
| nn | burma14 | 0.0150 | 7.0 | 4048 |
| nn | ulysses16 | 0.0136 | 7.1 | 9988 |
| or_opt | a280 | 1.3729 | 7.1 | 2821 |
| or_opt | att532 | 14.3526 | 8.2 | 31159 |
| or_opt | berlin52 | 0.0277 | 6.8 | 8098 |
| or_opt | burma14 | 0.0136 | 7.0 | 3323 |
| or_opt | ulysses16 | 0.0158 | 7.1 | 6859 |
| sfc | a280 | 0.0139 | 6.8 | 3555 |
| sfc | att532 | 0.0152 | 7.9 | 39498 |
| sfc | berlin52 | 0.0113 | 7.0 | 9095 |
| sfc | burma14 | 0.0146 | 7.3 | 3743 |
| sfc | ulysses16 | 0.0145 | 7.2 | 7580 |
| tabu | a280 | 0.5062 | 7.7 | varies |
| tabu | att532 | 1.5857 | 10.4 | varies |
| tabu | berlin52 | 0.0409 | 6.8 | varies |
| tabu | burma14 | 0.0256 | 7.2 | varies |
| tabu | ulysses16 | 0.0188 | 7.1 | varies |

The per-solver peak RSS is dominated by process and HashMap overhead at these sizes; the
matrix itself is small. The scaling table below is the memory evidence that matters.

## Matrix scaling baseline

| Cities | Status | Wall (s) | Peak RSS | Packed-item model (4 B/pair) |
|---:|---|---:|---:|---:|
| 1,000 | ok | 0.0324 | 10.7 MB | 1.9 MB |
| 5,000 | ok | 0.0967 | 103.4 MB | 47.7 MB |
| 10,000 | ok | 0.3064 | 390.6 MB | 190.7 MB |
| 25,000 | ok | 1.7375 | 2.34 GiB | 1.16 GiB |
| 50,000 | ok | 6.7888 | 9.33 GiB | 4.66 GiB |
| 100,000 | **oom** | 39.7809 | 25.76 GiB | 18.6 GiB |

Two findings:

1. **The 100k matrix does not fit this machine.** The run was killed at ~27 GB; the script
   recorded `oom`, not an extrapolated number.
2. **Measured RSS is roughly twice the packed-item model** at 50k, because the peak includes
   parser and allocator transients, not only the final `Vec<f32>`. Reducing those transients
   is as important as changing the O(n²) layout; the Phase 4 work should measure them
   separately.

## On-the-fly scaling result

`sfc` with `--distance-backend on-the-fly`, same synthetic generator:

| Cities | Status | Wall | Peak RSS |
|---:|---|---:|---:|
| 1,000 | ok | 0.0153s | 7.1 MB |
| 10,000 | ok | 0.0383s | 8.5 MB |
| 100,000 | ok | 0.2220s | 23.3 MB |

The 100k run stays under 1 GiB with three orders of magnitude of headroom; the matrix run at
the same size was killed at 25.76 GiB. This is the acceptance path for #647.

## Backend selection

- `--distance-backend auto|matrix|on-the-fly` (default `auto`); the same value is accepted as a
  top-level `distance_backend` in a pipeline TOML. The CLI flag wins over the file.
- `Auto` uses the matrix at or below 20,000 cities and on-the-fly above it.
- EXPLICIT input always uses the matrix, regardless of `--distance-type`, because the parser
  supplies only placeholder coordinates for those files. Coordinate input with
  `--distance-type explicit` is a descriptive error, not a panic.
- Trade-off: the matrix is O(n²) memory and a lookup is a `Vec` index; on-the-fly is O(n)
  memory and computes the declared TSPLIB primitive per lookup.

## Gate and publication

- The comparison is a manual per-release sweep on a documented machine; a nightly
  small-instance smoke is a follow-up once the thresholds stabilize. There is no per-PR CI job.
- The report is internal: `publish-benchmarks.sh` does not consume
  `bench/runs/tier-distance-compare.jsonl`, and the public leaderboard continues to measure the
  default backend.
- `scripts/check-distance-benchmarks.py` fails when a paired cell exceeds the documented
  runtime or memory tolerance.

## LK metric-fix re-baseline

LK used to build a private EUC_2D matrix, so it optimized the wrong objective on GEO, ATT,
CEIL_2D, and EXPLICIT instances while reporting the correct metric. After the fix, seeded LK
(`--seed 7`) is deterministic: five runs per instance return the same tour.

| Instance | Type | Tour cost | Published optimum | Gap |
| --- | --- | ---: | ---: | ---: |
| ulysses16 | GEO | 6909 | 6859 | 0.73% |
| att48 | ATT | 10684 | 10628 | 0.53% |
| gr120 | EXPLICIT | 6971 | 6942 | 0.42% |
| dsj1000 | CEIL_2D | 20,085,164 | 18,660,188 | 7.64% |
| a280 | EUC_2D | 2727 | 2579 | 5.74% |

Raw rows: `bench/baseline/distance/lk-rebaseline.jsonl`. The published LK numbers for these
instances should be re-run from this tier before the next release; the solver's behavior on
the affected metrics changed by design.

## Phase 1 refactor result

The matrix backend was re-run after the `Arc<dyn DistanceSource>` switch. All 180 cells
returned `ok`, every deterministic tour cost was identical, and peak RSS stayed within 5%.

| Check | Result |
|---|---|
| Tour cost, deterministic solvers | identical |
| `2opt att532` median | 0.1537s → 0.1537s |
| `3opt a280` median | 24.94s → 25.42s (+2%) |
| `or_opt a280` / `att532` | +1% / +2% |
| `nn`, `sfc` | sub-10 ms differences; noise |
| Peak RSS | within 5% |

The first refactor run showed a 20% regression on `2opt att532`. The trait default resolves two
`city_id2pos` calls and one `distance_by_pos` through the vtable per lookup; the matrix backend
now overrides `distance_between` with the direct-index body, restoring the baseline. The
conformance suite (Phase 1b) is the equivalence gate for that override.

## Phase 1 no-regression bound

The behaviour-preserving trait refactor is accepted only if, against this baseline:

- **Tour cost is identical** for every deterministic solver cell (`2opt`, `3opt`, `bhk`,
  `nn`, `sfc`). `lk` and `tabu` are compared by median and distribution, not exact tours.
- **Median wall-clock does not exceed the baseline by more than 15% or 10 ms, whichever is
  larger**, on any deterministic solver/fixture cell. The absolute floor recognizes that
  cells under ~50 ms are dominated by process startup and timer resolution; the percentage
  bound is meaningful above that.
- **Peak RSS does not increase by more than 5%** on any cell.
- A violation is investigated before proceeding. Relaxing the bound requires recording why
  in the PR that relaxes it.

## Reproduction

```bash
# Solver comparison (matrix family; Phase 2 adds the on-the-fly family)
task bench:distance

# Memory scaling, one fresh process per size
task bench:distance-scaling

# Large downloaded-instance tier (after download_data.sh)
scripts/bench-matrix.sh bench/matrix/tier-distance-large.toml
```

Criterion construction-time microbenchmarks remain separate:
`cargo bench --bench distance_matrix`.

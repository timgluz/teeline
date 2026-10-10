# Benchmark Data Contract (v1)

How Teeline benchmark results are produced, published and consumed. This is the
single source of truth for the JSON shape served from R2 and for the mirror into
criteriondb. **Bump `schema_version` and the URL prefix together** — the version
is in the path (`/bench/v1/`) precisely so a breaking change ships as `/bench/v2/`
without breaking deployed pages.

## Where the data lives

| Artifact | Location | Written by |
| --- | --- | --- |
| Raw measurements | `bench/runs/<tier>.tsv` (git-ignored) | `scripts/bench-matrix.sh` |
| Published JSON | R2 bucket, prefix `bench/v1/` | `scripts/publish-benchmarks.sh` |
| Public URL | `https://static.tspsolver.com/bench/v1/` | R2 custom domain |
| Archived observations | criteriondb, `tspsolver` profile | `scripts/publish-benchmarks.sh` |
| Solver identity map | `bench/solvers.json` | committed |

`static.tspsolver.com` is an R2 custom domain over the bucket that also serves
`tsplib/`. Read is public; there is no bucket listing, so every object must be
addressed by its full key.

## Object layout

```text
bench/v1/index.json                     manifest: what exists, from which commit
bench/v1/algorithms/{solver_id}.json    one shard per algorithm page
bench/v1/problems/{dataset_id}.json     one shard per problem page
```

One shard per page keeps each page to a single request. `{solver_id}` and
`{dataset_id}` are the ids used by the docs collection and the `/algorithms/{id}/`
and `/problems/{id}/` routes — see `bench/solvers.json` for the solver ids, and
note that the CLI name is not always the doc id (`som` → `kohonen_som`,
`tabu` → `tabu_search`, `bhk` → `bellman_karp`, `2opt` → `two_opt`).

## `index.json`

```json
{
  "schema_version": 1,
  "generated_at": "2026-10-12T09:00:00Z",
  "git_commit": "abc1234",
  "dirty": false,
  "teeline_version": "1.0.1",
  "tier": "A",
  "default_timeout_s": 180,
  "environment": {
    "os": "linux",
    "cpu_name": "AMD Ryzen 7 PRO 4750U",
    "n_threads": 16,
    "ram_gb": 32,
    "build": "release",
    "rust_version": "1.90.0"
  },
  "solvers": [{ "id": "lk", "name": "Lin-Kernighan", "configs": ["default"] }],
  "problems": [{ "id": "berlin52", "cities": 52, "optimal_cost": 7544.37 }]
}
```

Rules:

- `git_commit`/`dirty` describe the **binary that was measured**, not the commit
  that published the data. A dirty working tree at publish time is recorded as
  `dirty: true` rather than treated as fatal — the tree state at publish time is
  not what makes a number reproducible, the measured binary is. CI can demand the
  stricter behaviour with `--require-clean`.
- `git_commit_source` says where `git_commit` came from. `"tsv"` means the
  benchmark file recorded the commit its measurements came from. `"publish-time"`
  means the file carried no provenance, so `git_commit` is only HEAD when the
  publish ran and does **not** necessarily identify the built binary — a consumer
  must not present such a commit as the source of those numbers.
- `environment` is required. Wall-clock numbers are meaningless without it, and
  the UI shows it next to every table.
- `optimal_cost` is `null` — never `0` — when the instance has no known optimal tour.

### TSV provenance header

`bench-matrix.sh` prefixes each output TSV with `#`-comment provenance lines, which
`publish-benchmarks.sh` reads in preference to publish-time git state. The data rows
that follow are tab-separated:

```text
# git_commit=899de9d
# dirty=0
# teeline_version=1.0.1
# rust_version=1.99.0
# tier=a
# timeout_s=180
solver <TAB> dataset <TAB> config <TAB> run <TAB> wall_s <TAB> peak_rss_kb <TAB> tour_cost <TAB> status
nn <TAB> berlin52 <TAB> default <TAB> 1 <TAB> 0.0000 <TAB> 6720 <TAB> 8980.91797 <TAB> ok
```

A TSV without these lines still publishes, but is marked
`git_commit_source: "publish-time"` so the weaker provenance is visible rather
than implied.

## `algorithms/{solver_id}.json`

```json
{
  "schema_version": 1,
  "solver": "sa",
  "configs": ["default", "no-seed"],
  "results": {
    "berlin52": {
      "cities": 52,
      "optimal_cost": 7544.37,
      "by_config": {
        "default": {
          "runs": 5,
          "timeouts": 0,
          "errors": 0,
          "gap_pct": { "median": 6.8, "best": 5.9, "worst": 7.4, "mean": 6.8 },
          "wall_s": { "median": 0.34, "best": 0.31, "worst": 0.38 },
          "peak_rss_kb": { "median": 8100 }
        }
      }
    }
  }
}
```

`results` is keyed by dataset (not a flat list) so a page can render
"this algorithm on this instance" in O(1) without scanning, and `by_config`
nests variants of the same solver.

## `problems/{dataset_id}.json`

The per-instance solver leaderboard, sorted by `gap_pct.median` ascending — or by
`tour_cost.median` ascending when the optimal is unknown.

```json
{
  "schema_version": 1,
  "dataset": "berlin52",
  "cities": 52,
  "optimal_cost": 7544.37,
  "leaderboard": [
    {
      "solver": "lk",
      "config": "default",
      "runs": 5,
      "timeouts": 0,
      "errors": 0,
      "gap_pct": { "median": 0.0, "best": 0.0, "worst": 0.0 },
      "tour_cost": { "median": 7544.37, "best": 7544.37 },
      "wall_s": { "median": 0.15 },
      "peak_rss_kb": { "median": 6500 }
    }
  ]
}
```

## Semantics that consumers must honour

1. **An unknown optimal yields `null` gaps, never `0`.** A `0` gap means "reached
   the optimum", which is a strong claim. Instances without an `.opt.tour` are
   reported by `tour_cost` only.
2. **`gap_pct` is computed over successful runs only.** `runs` counts completed
   runs; `timeouts` and `errors` are separate counters. A solver that timed out
   three times and succeeded twice over a 3-minute limit reports `runs: 2,
   timeouts: 3` — it is not ranked as if its gap were the timeout value.
3. **Non-deterministic solvers are always summarised, never single-valued.**
   `bench/solvers.json` records per-solver `deterministic`, measured by running
   the solver repeatedly and comparing tours. For any solver with
   `deterministic: false`, a page must present median/best/worst (or a spread)
   and must not present a single number as *the* result. Measured so far:
   `nn`, `aco`, `stochastic_hill` and `lk` vary run to run; `2opt`, `3opt`,
   `or_opt`, `christofides`, `greedy_edge`, `savings` are stable.
4. **Every summary is recomputable from the raw rows.** The summaries are a cache
   over `bench/runs/*.tsv`, not an independent source of truth. If a summary and
   the raw rows disagree, the raw rows win.
5. **A timeout is not a bad result — it is missing data.** Pages render it in a
   `timeouts` column, and the absence of a gap cell for that solver on that
   instance is expected, not an error to paper over.

## Mirror into criteriondb

Each `(solver, config, dataset)` cell is uploaded as one `tspsolver`-profile run,
with one `SolverResult` per run number. Metrics sent per run:

| Metric | Unit | Notes |
| --- | --- | --- |
| `tour_cost` | *(empty)* | Total tour length as reported by the solver |
| `wall_s` | `s` | Wall-clock seconds |
| `peak_rss_kb` | `kb` | Peak RSS from GNU `time -v` |
| `optimal_cost` | *(empty)* | Known optimal, or omitted when unknown |
| `gap_pct` | `%` | `(tour_cost - optimal_cost) / optimal_cost * 100`, omitted when unknown |

The `config_label` field on the `tspsolver_observations` dimension carries the
config (for example `default`, `no-seed`, `epochs=100k`), so
`(solver, dataset, config_label, run_number)` is the unique case identity.

This makes the leaderboard a single SQL query — the reason optimal and gap are
stored rather than computed at read time:

```sql
SELECT t.solver, t.dataset, t.config_label,
       median(o.metric_value) FILTER (WHERE o.metric_name = 'gap_pct') AS median_gap
FROM tspsolver_observations t
JOIN observations o
  ON o.run_id = t.run_id AND o.observation_id = t.observation_id
GROUP BY 1, 2, 3
ORDER BY 4;
```

## Publishing

Reads are public; writes need a Cloudflare credential with R2 edit access.

```bash
# 1. Measure (resumable; safe to interrupt and re-run)
task bench:tierA                      # or: scripts/bench-matrix.sh bench/matrix/tier-a.toml

# 2. Build the JSON and inspect it without uploading
scripts/publish-benchmarks.sh --tier A --dry-run

# 3. Publish to R2, then mirror the raw runs into criteriondb
scripts/publish-benchmarks.sh --tier A --only r2
scripts/publish-benchmarks.sh --tier A --only criteriondb

# 4. Verify what is actually live
scripts/read-benchmarks.sh index
scripts/read-benchmarks.sh algorithm lk
scripts/read-benchmarks.sh problem berlin52
```

`scripts/publish-benchmarks.sh` always uses the `cf` CLI (the agentic Cloudflare
CLI) when it is on `PATH`; `wrangler` is only a fallback for machines with no `cf`
binary. Both are driven by the `CF_R2_BUCKET` environment variable. Reading does
not need a client at all — `scripts/read-benchmarks.sh` fetches over HTTPS by
default, and only uses `cf` when you ask for `--via r2`.

Authenticate once before publishing:

```bash
cf auth login          # or: export CLOUDFLARE_API_TOKEN=...
cf r2 buckets list     # find the real bucket name (CF_R2_BUCKET)
```

## Adding a solver or an instance

1. Add the solver to `bench/solvers.json` **and**
   `teeline-web/src/lib/solver-index.ts` — `solver-index.test.ts` fails if the two
   disagree, and also fails if a solver with a docs page is missing a `family`
   that matches its `SOLVER_GROUPS` entry.
2. Re-run the tier the solver belongs to, then republish. Pages pick up new shards
   with no code change; a solver that has not been measured simply renders no
   benchmark section.

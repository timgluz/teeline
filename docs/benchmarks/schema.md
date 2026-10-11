# Benchmark Data Contract (v2)

How Teeline benchmark results are produced, published and consumed. This is the
single source of truth for the JSON shape served from R2 and for the mirror into
criteriondb. **Bump `schema_version` and the URL prefix together** — the version
is in the path (`/bench/v2/`) precisely so a breaking change ships as `/bench/v3/`
without breaking deployed pages.

## What v2 changed, and why

Provenance moved from the dataset to **each solver individually**.

v1 recorded one `git_commit`, `dirty` and `environment` in `index.json` and stated
that the commit identified the measured binary. That holds only while every solver
is re-measured together. Under the per-release differential cadence — re-run the
algorithms that changed, keep the rest — it becomes false: after a release touching
two algorithms the other twenty are still measured at an older commit, yet a
dataset-level commit presents all of them as current.

That is the same class of defect as the distance-metric bugs previously fixed: a
plausible number carrying a false claim. Unlike a miscalculated value it **cannot be
reconstructed after publication**, so it was established before the first
differential publish.

Concretely, v2:

- adds a `measurement` block to every `algorithms/{id}.json`
- removes `git_commit`, `git_commit_source`, `dirty`, `teeline_version` and
  `environment` from `index.json`, which now makes no claim about when anything was
  measured
- adds `budgets` (per family) to `index.json`
- adds immutable per-release `snapshots/{tag}/`

`/bench/v1/` remains served and unchanged. It is superseded, not rewritten.

## Where the data lives

| Artifact | Location | Written by |
| --- | --- | --- |
| Raw measurements | `bench/runs/<tier>.tsv` (git-ignored) | `scripts/bench-matrix.sh` |
| Published JSON | R2 bucket, prefix `bench/v2/` | `scripts/publish-benchmarks.sh` |
| Public URL | `https://static.tspsolver.com/bench/v2/` | R2 custom domain |
| Release snapshots | R2 bucket, prefix `bench/v2/snapshots/{tag}/` | `scripts/publish-benchmarks.sh --snapshot` |
| Per-family budgets | `bench/budgets.json` | committed |
| Archived observations | criteriondb, `tspsolver` profile | `scripts/publish-benchmarks.sh` |
| Solver identity map | `bench/solvers.json` | committed |

`static.tspsolver.com` is an R2 custom domain over the bucket that also serves
`tsplib/`. Read is public; there is no bucket listing, so every object must be
addressed by its full key.

## Object layout

```text
bench/v2/index.json                     manifest: what exists (no provenance claim)
bench/v2/algorithms/{solver_id}.json    one shard per algorithm page, WITH provenance
bench/v2/problems/{dataset_id}.json     one shard per problem page
bench/v2/snapshots/{tag}/...            immutable copy of the whole tree at a release
```

One shard per page keeps each page to a single request. `{solver_id}` and
`{dataset_id}` are the ids used by the docs collection and the `/algorithms/{id}/`
and `/problems/{id}/` routes — see `bench/solvers.json` for the solver ids, and
note that the CLI name is not always the doc id (`som` → `kohonen_som`,
`tabu` → `tabu_search`, `bhk` → `bellman_karp`, `2opt` → `two_opt`).

## `index.json`

```json
{
  "schema_version": 2,
  "generated_at": "2026-10-12T09:00:00Z",
  "tier": "A",
  "default_timeout_s": 180,
  "budgets": {
    "meta": { "seconds": 300, "agreed": true },
    "local": { "seconds": 120, "agreed": false }
  },
  "budgets_note": "…why some figures are provisional…",
  "solvers": [
    { "id": "lk", "name": "Lin-Kernighan", "family": "local", "configs": ["default"] }
  ],
  "problems": [{ "id": "eil51", "cities": 51, "optimal_cost": 426 }]
}
```

Rules:

- **`index.json` carries no commit, dirty flag, version or environment.** It must
  not: under the differential cadence its solvers were measured at different
  commits, and any single value would present them as sharing one. Those fields
  live in each `algorithms/{id}.json` `measurement` block. A consumer that wants
  provenance for a solver reads that solver's shard; the algorithm page does
  exactly this.
- `generated_at` is when this manifest was written, which is **not** when the
  results were measured. It is not a provenance substitute.
- `default_timeout_s` is the run's overall timeout as recorded by the harness; the
  per-family `budgets` are what a result must be read against.
- `budgets` maps a solver family to its wall-clock allowance. `agreed: true` means
  the figure is fixed by a decision; `agreed: false` marks a provisional value that
  has not been settled, so a page must not present it as an agreed like-for-like
  basis. A gap is never shown without its budget.
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
# measured_at=2026-10-12T09:00:00Z
solver <TAB> dataset <TAB> config <TAB> run <TAB> wall_s <TAB> peak_rss_kb <TAB> tour_cost <TAB> status
nn <TAB> berlin52 <TAB> default <TAB> 1 <TAB> 0.0000 <TAB> 6720 <TAB> 8980.91797 <TAB> ok
```

A TSV without these lines still publishes, but is marked
`git_commit_source: "publish-time"` so the weaker provenance is visible rather
than implied.

## `algorithms/{solver_id}.json`

```json
{
  "schema_version": 2,
  "solver": "sa",
  "measurement": {
    "git_commit": "899de9d",
    "git_commit_source": "tsv",
    "dirty": false,
    "teeline_version": "1.0.1",
    "measured_at": "2026-10-12T09:00:00Z",
    "environment": { "os": "linux", "build": "release", "rust_version": "1.90.0" }
  },
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

### The `measurement` block

| Field | Meaning |
| --- | --- |
| `git_commit` | The commit the **measured binary** came from |
| `git_commit_source` | `tsv` if the benchmark file recorded that commit; `publish-time` if it did not |
| `dirty` | Whether the tree was dirty when the measurement was taken; `null` if the TSV did not record it |
| `teeline_version` | Version of the measured binary |
| `measured_at` | When the measurement was taken (not when it was published); `null` if the TSV did not record it |
| `environment` | `os`, `build`, `rust_version` |

Rules:

- This block describes **only this solver**. A solver that was not re-measured
  keeps the block it already had, byte for byte, so its page keeps stating the
  version it was actually measured at.
- `git_commit_source: "publish-time"` is a warning to the consumer: the commit is
  merely HEAD when the publish ran and does **not** identify the built binary, so
  it must not be presented as making those numbers reproducible. A dirty tree is
  recorded rather than treated as fatal — the tree state at publish time is not
  what makes a number reproducible. CI can demand the stricter behaviour with
  `--require-clean`.
- `environment` is required. Wall-clock numbers are meaningless without it, and
  the page shows it beside the table.
- Every field the TSV did not record is `null`, never the publisher's value. A header carrying
  `git_commit` but no `dirty` must not pair the measured commit with the publisher's dirty flag,
  version or rustc — that is the same false claim in miniature, and once written into a shard under
  the differential cadence it is permanent.
- `measured_at` is `null` when the TSV recorded no `# measured_at=` line. It is
  deliberately **not** filled in from the publish time: a file carried forward or
  republished would then be stamped with today's date, which is the misdating this
  contract exists to avoid. The harness records it; a missing value stays missing.
- The provenance footer on an algorithm page is this block. Staleness is
  communicated this way rather than by a badge: being able to see which version
  produced a number is the requirement (design section 3.8).

## `problems/{dataset_id}.json`

The per-instance solver leaderboard, sorted by `gap_pct.median` ascending — or by
`tour_cost.median` ascending when the optimal is unknown.

```json
{
  "schema_version": 2,
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
6. **Provenance is per solver and must be presented that way.** A page states the
   version from *that solver's* `measurement` block. Two solvers on the same page
   may legitimately have been measured at different commits, and presenting one
   version for both would be a false claim about at least one of them. In
   particular, `index.json` has no version to show — its absence is deliberate, not
   an omission to fill in from `generated_at`.

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

The archived run records the commit **the TSV says the measurements came from**,
with `git_commit_source` alongside it — not whatever HEAD was when the publish
ran. Stamping archived runs with the publish-time commit would attach the same
false claim that per-solver provenance exists to prevent.

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

## Snapshots

Every release writes an immutable copy of the whole tree under
`snapshots/{tag}/`, so a later regression can be evaluated against what an
algorithm actually produced in that release:

```text
bench/v2/snapshots/v1.0.0/index.json
bench/v2/snapshots/v1.0.0/algorithms/{solver_id}.json
bench/v2/snapshots/v1.0.0/problems/{dataset_id}.json
```

- The snapshot is **complete**, not a diff: it must stand alone, because it is the
  only record of that release's numbers once the current tree moves on.
- It is **immutable**. Publishing refuses to overwrite an existing tag, since
  rewriting one would silently redefine what that release measured; step past it
  deliberately with `--force-snapshot`.
- Each snapshot carries its own `index.json`, so every reader command works against
  a snapshot with `--snapshot <tag>`.

```bash
scripts/read-benchmarks.sh snapshot v1.0.0
scripts/read-benchmarks.sh --snapshot v1.0.0 algorithm lk
```

## Differential publishing

Under the per-release cadence only the algorithms that changed are re-measured, so
the published tree is **completed from what is already live**: the solvers this run
did not measure keep their shards (and therefore their `measurement` blocks), and
the problem leaderboards keep the entries that rank them. A run that replaced the
leaderboard from a one-solver TSV would silently drop every other solver from it.

There is no bucket listing, so the previous `index.json` is read first — it names
the shards to fetch — into `build/bench-prev/`. Override with `--previous <dir>`, or
`--no-fetch-previous` to work offline (correct for a first publish or a full
re-run, wrong for a differential one).

Only objects whose bytes changed are uploaded, which is what makes re-publishing a
single solver touch only that solver's shard. `index.json` and any affected
`problems/*.json` legitimately change too.

The merge is strict about the tree it is given. Only shards the previous manifest
names are merged — a stray file was never live — and a manifest that names a shard
which is missing or unreadable **aborts the publish** rather than dropping that
solver from the republished tree. For the same reason a partial run needs an
explicit `--first-publish` when there is no previous tree at all, and only a
confirmed `404` is read as "nothing to merge with": a network failure is not a
licence to publish a tree holding only this TSV's solvers.

**A solver's shard is replaced wholesale when it is re-measured.** So a differential
run must cover that solver's whole tier — every instance, every config — or the
instances it omits are lost. The cadence is differential per *algorithm*, not per
*instance*.

## Publishing

Reads are public; writes need a Cloudflare credential with R2 edit access.

```bash
# 1. Measure (resumable; safe to interrupt and re-run)
task bench:tierA                      # or: scripts/bench-matrix.sh bench/matrix/tier-a.toml

# 2. Build the JSON and inspect it without uploading
#    (merges build/bench-prev/, fetching it from R2 when absent)
scripts/publish-benchmarks.sh --tier A --dry-run

# 3. Publish to R2, then mirror the raw runs into criteriondb
scripts/publish-benchmarks.sh --tier A --only r2
scripts/publish-benchmarks.sh --tier A --only criteriondb

# 3b. At a release, also write an immutable snapshot of the whole tree
scripts/publish-benchmarks.sh --tier A --snapshot v1.0.0

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

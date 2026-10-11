---
id: 01a12651-5646-7e10-ba40-b75b1a4fc3b2
slug: tasks/bench-algorithm-pages
title: "feat(web): render per-algorithm benchmark tables and graphs from published data"
type: task
status: draft
priority: high
tags: [benchmarks, teeline-web, frontend]
---

## Overview

Algorithm pages render no benchmarks. The only benchmark surface is the landing chart, fed by a
hand-maintained array in `teeline-web/src/lib/benchmark-data.ts` that is stale: 20 of 22 solvers,
missing `aco`, `bhk` and `shuffle`, and carrying an `hk` entry that corresponds to no solver.

`/bench/v2/` is now live, so this task builds the shared read layer and the algorithm page on top
of it. `tasks/bench-problem-leaderboard` then reuses the layer for the problem pages; the split is
because the two pages need different shapes (one solver across instances; one instance across
solvers) and the shared layer is the only thing worth landing first.

**Status 2026-10-11:** the prelim tier has since published all 22 solvers (7 instances; `bhk` and
`branch_bound` on 3, the latter with two configs). The "two of 22" below is drafting-time context,
not current state. See the Plan section for what that changes.

## What exists to build on

- `algorithms/[id]/index.astro` (42 lines) renders the docs entry through `DocsLayout`. The
  benchmark section is added here, not in the markdown, so no doc has to change.
- `LandingBenchmarks.astro` already draws an inline `<svg>` scatter by hand. **There is no charting
  dependency** and this task does not add one: same approach, one component.
- `upload.ts` fetches TSPLIB at runtime and switches base URL on `import.meta.env.DEV`;
  `astro.config.mjs` proxies `/tsplib` to `https://static.tspsolver.com`. Benchmarks need the same
  two things: a `benchBaseUrl(dev)` helper and a `/bench` proxy entry.
- The site is a static build. Runtime fetch is what lets data be republished without a redeploy
  (decision 9), so the section is a client island, not build-time.

## The shapes being rendered

`algorithms/{id}.json`:

```
solver, configs[], measurement{git_commit, git_commit_source, dirty, teeline_version,
  measured_at, environment{os,build,rust_version}},
results{dataset:{cities, optimal_cost, by_config{config:{runs, timeouts, errors,
  [gap_pct{median,best,worst,mean}], tour_cost{...}, wall_s{...}, [peak_rss_kb{...}],
  [optimal_cost]}}}}
```

Note `gap_pct` is **absent** when the instance has no known optimum — `berlin52`, `att48`,
`burma14`, `ch150`, `a280` and `dsj1000` are all null today. The table must therefore render
without a gap column rather than showing 0 or a blank that reads as one.

## Content

- **Summary line** — median gap and median wall time, with the budget and config stated inline, so
  a cross-family row is never read as like-for-like
- **Per-instance table** — instance, cities, distance type, optimal, gap (median, best), wall time
  (median), runs, timeouts
- **Graph** — gap against instance size, one point per instance, drawn as inline SVG. Same numbers
  as the table, chosen for shape: the table answers "what did it score here", the graph answers
  "how does it scale". Where an optimum is unknown the point is plotted on tour cost instead and
  the axis is labelled as such; it is never silently mixed with gaps.
- **Provenance footer** — the solver's own `measurement`: commit, version, environment, measured_at

## Honest rendering rules

Each of these is a way a benchmark page lies, and each is cheap to get right once:

- **Timeouts render as timeouts.** No gap and no tour cost for a killed run — there is no result to
  report (decision 5). Not a zero, not a blank cell that reads as one.
- **Non-deterministic solvers show their spread.** `tour_cost` carries median/best/worst; a page
  that prints only the median presents a distribution as a single fact.
- **A budget is never omitted.** A gap without the allowance it was measured under is
  uninterpretable, and cross-family gaps are not like-for-like (decision 4).
- **Provenance is per solver.** One solver may be measured at an older commit than another, and the
  footer says so per section rather than implying one build for the page.
- **Partial data is normal, not an error.** Coverage is uneven and moves with every publish:
  `bhk` and `branch_bound` are measured on 3 instances (their capability tier) where the other
  solvers have 7. The page renders whatever its own shard holds.

## Read path

`fetch` `algorithms/{id}.json` at runtime, mirroring `upload.ts`. A **404 is a normal state**, not
a failure: an unpublished solver, or a solver with no measurements yet, renders no benchmark
section and no error. Any other status is a real error and should be visible rather than swallowed.
Nothing may be required at build time (acceptance criterion below), because the build must not fail
when the data is unreachable.

## Plan

Drafted 2026-10-11 against repo HEAD `0eca5ef8` and live `index.json` `generated_at`
`2026-10-11T11:00:18Z`. The tree is being republished by a concurrent session, so every count here
is a snapshot to re-verify, not a premise to build on.

### Phase 0 — re-verify the data before writing code

```bash
scripts/read-benchmarks.sh index           # tier, families, budgets, solver list
scripts/read-benchmarks.sh algorithm 2opt  # one shard, human-readable
scripts/read-benchmarks.sh check           # every advertised shard parses
```

The read layer derives everything from the shard it fetched. Two live properties make that
non-negotiable:

- **`index.json` is not an inventory.** It currently lists 7 problems while
  `problems/{a280,ch150,dsj1000,gr120,kroA100}.json` still return `200`. A page fetches its own
  shard; 404 is the only "absent" signal.
- **Coverage is uneven.** 22 solvers, but 3 instances for `bhk`/`branch_bound` and 7 for the rest;
  `branch_bound` publishes two configs (`budget`, `exact`). "One row per instance" and "one config
  per solver" are both wrong assumptions — the UI renders the rows the shard holds.

### Phase 1 — the read layer (`teeline-web/src/lib/benchmark.ts`)

Pure, DOM-free and node-testable, so the rendering rules live in one place:

- `benchmarkBaseUrl(dev)` → `/bench/v2` in dev (through the new Vite proxy),
  `https://static.tspsolver.com/bench/v2` in production — mirroring `datasetBaseUrl` in `upload.ts`.
- `fetchAlgorithmShard(id)` → `{kind:'ok', shard}` | `{kind:'absent'}` (HTTP 404 only) |
  `{kind:'error', status}` (network failure, any other status, or an unrecognised `schema_version`).
- `parseAlgorithmShard(raw)` — runtime validation. `schema_version !== 2` is an error, not a silent
  fallback: a v3 shard rendered by v2 rules is the "plausible number carrying a false claim" the
  contract exists to prevent.
- `chooseColumns(rows, budget)` → `{gap, optimal}`, each true only when a row carries it; `gap` is
  additionally false when no budget resolved (Phase 2).
- `summarise(shard, budget)`, `cell(row, config)`, `gapPoints(...)`, `formatGap`, `formatSeconds`,
  `formatTimeout`.

Rules encoded here rather than in the component:

1. A gap exists only where `optimal_cost` is non-null; `null` never becomes `0`.
2. A cell with `runs: 0` renders from `timeouts`/`errors` and contributes no gap, tour or wall time.
3. Spread follows `solver-index.isDeterministic(id)`: a non-deterministic solver always shows median
   **and** best/worst. Today's shards are all deterministic, so that path can only be tested from a
   fixture.
4. Provenance is read per shard; `dirty: null` renders "dirty state not recorded", never "clean".

### Phase 2 — the algorithm page section

- `src/components/AlgorithmBenchmarks.tsx` — a Preact island mounted `client:visible` from
  `src/pages/algorithms/[id]/index.astro` after `<Content />`, so no markdown doc changes.
- Build-time props (no network): the solver `id`, and an `instanceMeta` map
  (`id → {name, edgeWeightType, cities}`) from the `problems` collection. The shard carries neither a
  display name nor a distance type, and the planned table has both columns.
- Runtime: two small cached fetches — `index.json` (the `budgets` map, the only published statement
  of the allowance the numbers were measured under) and `algorithms/{id}.json`.
- **Gap cells are gated on a resolved budget.** If `index.json` is unavailable the table renders tour
  cost, wall time, runs and timeouts and says why the gap column is hidden. This makes decision 4
  mechanical: no gap is ever shown without the allowance it was measured under.
- Summary line: instances covered, config, family budget (marking `agreed: false` provisional),
  median gap and median wall time, or "no published optimum" when no row has one.
- Table: instance, cities, distance type, optimal, gap median, gap best, wall median, runs, timeouts.
- Graph: inline SVG in the `LandingBenchmarks` style, x = instance size, one point per instance.
  **Axis rule:** gaps when the shard has any — instances without an optimum are then omitted and the
  caption says how many and why; when no instance has a gap, plot tour length and label the axis so.
  Two y meanings are never mixed on one axis.
- Provenance footer: commit and its `git_commit_source`, the `dirty` tri-state, `measured_at`,
  `teeline_version` and the environment block.
- 404 → no section, no error. Non-404 failure or unsupported schema → a visible inline message.

### Phase 3 — landing chart migration

- Delete `BERLIN52_BENCHMARKS` and `LABELED_IDS` from `src/lib/benchmark-data.ts`; keep
  `FAMILY_COLORS`/`FAMILY_LABELS` (add `utility`, which `shuffle` now exercises) in the shared module.
- `LandingBenchmarks.astro` becomes `LandingBenchmarks.tsx`, `client:visible`, reading
  `problems/{INSTANCE}.json` — the cross-solver shape, which is the problem shard, not the algorithm
  shard.
- `INSTANCE` stays `berlin52` and the same axis rule picks y: berlin52's `optimal_cost` is
  deliberately `null` pending the EUC_2D rounding fix ([[tasks/fix-euc-2d-rounding]]), so today the
  chart plots **tour length** with a labelled axis and ~20 real points, and upgrades itself to gap
  once berlin52 publishes an optimum. Switching to `att48`, `eil76` or `ulysses22` — each with ~20
  rows *and* a known optimum today — is a one-constant change if the gap axis is wanted immediately.
- The "Twenty algorithms, one chart." headline becomes count-derived so it cannot go stale; the
  family legend stays. An absent or empty shard renders no section.

### Phase 4 — tests, e2e, docs

- `src/lib/benchmark.test.ts` with fixtures under `src/lib/__fixtures__/`, imported with `?raw` as
  `solver-index.test.ts` already does (keeps Node types out of the browser tsconfig). Pinned to a
  recorded publish: a trimmed `algorithm-2opt.json` (mixed gap/no-gap), `algorithm-branch-bound.json`
  (two configs), `problem-berlin52.json`, plus a synthetic timeout cell and a synthetic
  non-deterministic spread — neither exists in the live tree, so only a fixture can test them.
- Cases: schema-version mismatch; 404 → absent and 500 → error with a stubbed fetch; gap column
  dropped when no row has one *and* when no budget resolved; timeout renders as a timeout; per-shard
  provenance with two different commits; determinism spread; axis never mixes gap and tour; a
  `utility`-family colour exists for `shuffle`.
- `tests/algorithm-benchmarks.spec.ts` (Playwright, routes fulfilled from the fixtures so it needs no
  network): the section renders and hydrates for a published solver, and vanishes for a 404.
  Playwright is not in CI today, so this is a local guard — worth having because it is the only check
  that the island actually hydrates in a page.
- `docs/benchmarks/schema.md`: its `problems/{dataset_id}.json` example shows
  `berlin52 optimal_cost: 7544.37`, contradicting both `docs/problems/berlin52.md` and the live shard
  (both `null`). One-line fix here; the rest of that doc is [[tasks/bench-docs-spec-update]]'s remit.

### Verification

```bash
cd teeline-web
npm run format:check && npx tsc --noEmit && npm test && npm run build:check
npm run dev   # /algorithms/2opt/, /algorithms/3opt/, /algorithms/bhk/, /
```

`scripts/read-benchmarks.sh check` before and after, and re-read `index.json` `generated_at`: the
tree is being republished concurrently, so "the page is wrong" and "the data moved" must be told
apart.

### Planning findings

- The task body's "Two of 22 solvers are published today" is stale: the prelim tier published all 22
  at `2026-10-11T11:00:18Z`. The partial-data rule still has live cases (`bhk`/`branch_bound` on 3
  instances, `shuffle` with a shard but no page), so the criteria stand.
- That same publish dropped 5 problems from `index.json` while their shards stayed served. It is a
  publishing concern, not this task's, but the read layer is designed around it.
- No timeout cell is published today, so "timeouts render as timeouts" cannot be verified against
  live data — hence the fixture.

## Acceptance Criteria

- [ ] Algorithm page renders summary, per-instance table, graph and provenance footer
- [ ] An algorithm with no published shard renders no benchmark section and does not error
- [ ] A solver measured at an older version displays that version honestly
- [ ] Cross-family comparison is not presented without its budget
- [ ] Timeouts render as timeouts; no gap cell is fabricated for them
- [ ] Non-deterministic solvers show a spread, never a single value presented as *the* result
- [ ] An instance with no known optimum renders without a gap column, and the graph says what it
      is plotting instead
- [ ] `BERLIN52_BENCHMARKS` deleted; the landing chart reads the published source
- [ ] The phantom `hk` entry and the missing `aco`/`bhk`/`shuffle` solvers resolved by the migration
- [ ] `tsc`, vitest and the web build pass
- [ ] No new dependency on network access at build time
- [ ] Unit tests cover the pure parts: parsing a shard, choosing table columns when `gap_pct` is
      absent, and formatting a timeout — the rendering rules above are what regress silently
- [ ] No gap cell renders unless the published budget for that solver's family resolved
- [ ] Absence means a 404 for the page's own shard; not being listed in `index.json` is never read
      as absence (the last publish left 5 problem shards served but unlisted)

## Notes

- `src/lib/solver-index.test.ts` already enforces that `bench/solvers.json` and `nav-data.ts`
  agree; keep that invariant intact.
- The dev proxy needs `/bench` alongside `/tsplib`, or local development silently renders no
  benchmarks and looks like a bug in the component.

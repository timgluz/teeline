---
id: 01a12b16-2fe4-7a20-b2f2-1c24f8526891
slug: tasks/blog-benchmarks-duckdb
title: "docs(blog): analysing the benchmark data with DuckDB"
type: task
status: draft
priority: medium
tags: [blog, benchmarks, duckdb, data]
---

## Overview

A post showing how to check Teeline's published numbers with DuckDB — the same data the site renders,
queried directly, so a reader can verify a claim, compare solvers their own way, or trace how a number
was produced instead of trusting a chart.

## Verified before writing (DuckDB 1.5.5, against the live tree)

- A **single shard reads straight over HTTPS**:
  `read_json_auto('https://static.tspsolver.com/bench/v2/algorithms/lk.json')`.
- **Globs over HTTP do not work**: DuckDB answers "Globs (`*`) for generic HTTP file are not
  supported". A cross-solver query therefore needs an explicit file list or a local fetch first
  (`scripts/read-benchmarks.sh list` enumerates the keys). The post must show that step rather than a
  query that cannot run.
- The two shapes that matter, both tested against the live shards:
  - problem shard (one instance, many solvers): `UNNEST(leaderboard)`;
  - algorithm shard (one solver, many instances):
    `UNNEST(map_entries(CAST(results AS MAP(VARCHAR, JSON))))`, then the same on `by_config`.
  Example output: `lk` on `gr17` 97.84%, on `burma14` 6.62% — matching what the site shows.

## Outline

1. What is published and where: per-solver shards, per-instance leaderboards, the manifest, release
   snapshots.
2. Reading one shard, then joining it into a table.
3. The caveats that make the numbers honest, each with a query where one fits: budgets are per family
   and cross-family gaps are not like-for-like; a timeout is missing data, not a bad result; a
   non-deterministic solver is a distribution, so median with best and worst; `optimal_cost: null`
   means no gap exists.
4. A worked comparison (for example `lk` against `2opt`) as the payoff.
5. The criteriondb mirror, for readers who would rather query SQL than files.

## Acceptance Criteria

- [ ] Every query in the post runs against the live tree and its output is pasted
- [ ] The fetch step for multi-file queries is shown, not assumed
- [ ] The budget, timeout and spread caveats all appear
- [ ] Frontmatter matches the existing posts, and prettier and the site build pass
- [ ] Links to the contract (`docs/benchmarks/schema.md`) and to the download task

## Notes

- Posts live in `teeline-web/src/content/blog/`; `fourier-algorithm-how-it-came-to-be.md` is the
  closest existing example of a data-heavy post.
- Sandbox quirk, not for publication: DuckDB could not write its extension directory outside the
  workspace, so `httpfs` had to be installed with `SET extension_directory=<workspace path>`. On a
  reader's machine it installs normally.

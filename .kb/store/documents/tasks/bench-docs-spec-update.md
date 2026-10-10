---
id: 01a12651-565c-7183-9d74-562a67e27df0
slug: tasks/bench-docs-spec-update
title: "docs(bench): retire benchmarks.md, add methodology, update schema contract"
type: task
status: draft
priority: medium
tags: [benchmarks, docs]
blocked_by: [tasks/bench-publish-provenance]
---

## Overview

Benchmark documentation is inconsistent with both the contract and the code, and the page that
held the old numbers is being retired.

- `docs/benchmarks.md` holds pre-fix numbers and contradicts current behaviour (for example
  ATT instances were measured as Euclidean before the distance-type fix). Decision: retire it.
- `docs/benchmarks/schema.md` documents a single dataset-level `git_commit`, which the
  per-solver provenance change invalidates.
- `docs/algorithms/simulated-annealing.md` and others may state termination behaviour the
  plateau work changes.
- The `--platoo_epochs` help/docs defect is tracked separately in
  `tasks/bench-platoo-epochs-help-name`.

## Goals

- Remove the stale benchmarks page without breaking inbound links
- Bring the schema contract in line with per-solver provenance and per-family budgets
- Add a methodology page explaining the tier, budgets, and why cross-family comparisons are not
  like-for-like
- Document plateau/convergence semantics once implemented

## Acceptance Criteria

- [ ] `docs/benchmarks.md` retired; inbound references updated (repo docs, web, blog)
- [ ] No dangling links to the retired page (check links)
- [ ] `docs/benchmarks/schema.md` describes per-solver `measurement` and drops the single-commit
      claim
- [ ] Per-family budgets documented as part of the contract, not just the data
- [ ] Methodology page exists: tier selection, budgets, timeout semantics, non-determinism
      handling, and the comparability rule
- [ ] Converged-run definition documented once the plateau work lands
- [ ] `docs/CLAUDE.md`/`AGENTS.md` solver table still accurate
- [ ] `markdownlint` clean

## Notes

The spec `decisions/benchmarking-design` holds the stable rationale; this task owns the
user-facing docs, and should not duplicate the spec.
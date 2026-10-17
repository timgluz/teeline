---
id: 01a12651-5689-7883-ab53-7ff7bb70901e
slug: tasks/bench-regression-history
title: "feat(bench): regression history across releases"
type: task
status: draft
priority: low
tags: [benchmarks, regression, follow-up]
blocked_by: [tasks/bench-publish-provenance]
---

## Overview

Per-release snapshots and the criteriondb mirror are retained so a regression can be evaluated
against what an algorithm produced in an earlier release. Nothing consumes that history yet.

Regression *display* was deliberately deferred during planning; this task captures the deferred
work so the retained data has a stated purpose rather than being an unexamined cost.

## Goals

- Decide where historical comparison is presented: a page, a per-algorithm history view, or
  queryable-only
- If displayed, show per-algorithm trend without implying cross-family comparability
- Ensure the retained data is sufficient for the chosen presentation

## Options to settle

1. Per-algorithm history on the algorithm page (gap over releases, against the same instance set)
2. A dedicated regressions page across all algorithms
3. Queryable-only via criteriondb; no UI

## Acceptance Criteria

- [ ] Presentation decision recorded
- [ ] If a UI: it states the instance set and budget for each historical point, and does not plot
      incomparable budgets as one line
- [ ] If a UI: it handles instances that were added or removed between releases
- [ ] If queryable-only: the intended queries are documented so the data is actually usable
- [ ] Snapshots verified sufficient for the chosen presentation

## Notes

Deliberately last and optional. The snapshots written by the publish task are the enabler; this
task may remain unstarted without blocking anything else.
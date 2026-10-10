---
id: 01a1264b-f469-78c0-b4f8-e4dfbd39a729
slug: decisions/benchmarking-design
title: "Solver Benchmarking: Design & Specification"
type: spec
status: active
priority: high
tags: [benchmarks, spec, web, ci]
---

# Solver Benchmarking: Specification

**Status:** Agreed, not started
**Date:** 2026-10-12

What Teeline's published solver benchmarks are, what they must guarantee, and the decisions
behind them. The JSON shape served to consumers is defined in `docs/benchmarks/schema.md`;
this document says *why* that shape is what it is and what claims the published numbers are
allowed to make. Implementation sequencing and mechanics stay out of the tracked record.

---

## 0. Locked-in decisions

| # | Decision | Choice |
| --- | --- | --- |
| 1 | Comparison basis | **Two runs per algorithm** — budget-limited and to-convergence |
| 2 | "Convergence" | **Plateau stop added to all iterative solvers** |
| 3 | Instance coverage | **Capability tiers**, not exhaustive |
| 4 | Budgets | **Per family**, recorded in the data and stated on the page |
| 5 | Timeout handling | **Blank** — missing data, not a bad result |
| 6 | Prerequisites | Three solver fixes, each its own PR, before measurement |
| 7 | Page content | Summary line + per-instance table + provenance footer |
| 8 | Harness | New `bench-matrix.sh` + tier config; retire `bench-solvers.sh` |
| 9 | Web read path | **Runtime fetch** from R2; retire the hand-maintained array |
| 10 | Campaign scale | Runs=5 per cell; metaheuristic budget 300s |
| 11 | `docs/benchmarks.md` | Retire |
| 12 | Cadence | **Per-release, differential** — re-run only changed algorithms |
| 13 | Config names | `budget300`, `converge` |
| 14 | Staleness signal | Provenance footer only; no separate badge |
| 15 | Snapshot trigger | Per release tag |
| 16 | Regression display | Deferred follow-up; this work only produces the enabling data |

---

## 1. Problem

Every algorithm page is currently unmeasured. The only benchmark surface is the landing page
chart, fed by a hand-maintained array that is stale: 20 of 22 solvers, missing `aco`, `bhk`
and `shuffle`, and carrying an `hk` entry that corresponds to no solver.

Separately, the measurement harness does not exist. The data contract names
`scripts/bench-matrix.sh` and `bench/runs/<tier>.tsv`, but what exists covers 4 of 22
solvers, omits the `config` and `status` columns the contract requires, enforces no timeout,
and cannot resume an interrupted run.

So no published number describes the current solvers, and there is no mechanism to keep one
accurate.

---

## 2. Invariants

A benchmark run that violates any of these is not publishable.

1. **A number is comparable, or it is labelled as not.** Ranking is meaningful only within
   a solver family; a gap figure is never presented as a like-for-like cross-family
   comparison.
2. **A timeout is missing data, not a bad result.** It is recorded in its own column and
   contributes no tour cost or gap.
3. **Every published number names the binary that produced it.** Provenance is per-solver,
   not per-dataset.
4. **Non-deterministic solvers are summarised, never single-valued.**
5. **An unknown optimal yields a null gap, never 0.** A 0 gap asserts optimality.
6. **Every summary is recomputable from raw measurements.** Where a summary and the raw
   rows disagree, the raw rows win.
7. **Averages never merge incomparable configs.** Results stay grouped by configuration.

---

## 3. Decisions and rationale

### 3.1 Two runs per algorithm

Each algorithm is measured at a shared budget *and* run to convergence. The budget-limited
run answers "how good per unit time" and is the only fair basis for ranking; the convergence
run answers "how good given unlimited time". Pages present both rather than choosing.

Required because natural solver costs differ by orders of magnitude — `nn` finishes
berlin52 in milliseconds while metaheuristics converge over minutes — so any single number
would measure the budget rather than the algorithm.

### 3.2 Per-family budgets, always recorded

Exact, local-search and metaheuristic families receive different wall-clock allowances.
Every result carries its budget and pages state it, because a gap figure without its budget
is uninterpretable.

A uniform budget was rejected: one-shot constructors finish in milliseconds, so a uniform
cap wastes nearly all of it while still being far too short for metaheuristics to converge.

### 3.3 "Convergence" must be implementable before it can be claimed

A run counts as converged when it stops improving for a sustained period. Today only two
iterative solvers have any convergence criterion, and one solver does not honour its
iteration budget at all — so "converged" is not currently a measurable event. Giving the
iterative solvers a common stopping rule is a prerequisite to publishing a convergence
result, not an optimisation of it.

### 3.4 Capability-tiered coverage

Exact solvers are exponential and cannot run the corpus; large instances exceed what several
metaheuristics can finish. Coverage therefore spans every distance type and a range of sizes
rather than attempting every solver on every instance.

Exhaustive coverage was rejected as unbounded in cost and dominated by timeouts.

### 3.5 Provenance is per-solver — the critical correctness requirement

This follows directly from decision 12.

The contract currently records one commit for the whole dataset. That holds only while every
solver is re-measured together. Under differential re-runs it becomes false: after a release
changing two algorithms, the other twenty are still measured at an older commit, yet a
dataset-level commit presents all of them as current.

Provenance therefore attaches to each solver individually. A solver that has not been
re-measured keeps the provenance of its last real measurement, and its page states the
version it was actually measured at. This is a correctness requirement of the same kind as
the distance-metric defects previously fixed: it is about not attaching a plausible but
false claim to a number.

**It must be established before the first differential publish**, because unlike a
miscalculated value it cannot be reconstructed afterwards — once published, the link between
a solver's numbers and the commit that produced them is gone unless it was recorded.

### 3.6 Historical versions retained

Each release retains an immutable snapshot, and raw per-run observations are mirrored to
criteriondb, so a regression can be evaluated against what an algorithm produced in an
earlier release. Regression *display* is deliberately out of scope; this specification only
requires that the data to do it exists.

### 3.7 Web reads at runtime

Algorithm pages fetch their results at runtime rather than inlining them at build time. This
keeps benchmark data republishable without a site redeploy — the site is a static build — and
degrades to an absent section rather than a build failure when data is missing.

### 3.8 Staleness is communicated by provenance, not a badge

Each page shows the measurement version in a provenance footer. Being able to see which
version produced a number is the requirement; no separate indicator is needed.

---

## 4. Non-goals

- Regression visualisation or a history UI — only the enabling data is in scope.
- Automated re-measurement in CI. Re-runs are manual per release.
- Measuring `shuffle`. It is a `utility` baseline, not an algorithm, and has no docs page.
- Automatic re-run triggers based on changed files. Manual selection for now, with the
  operator responsible for not mixing incomparable measurements.

---

## 5. Success criteria

- Every algorithm with a docs page renders measured results, or explicitly states it is
  unmeasured.
- Every displayed number carries the version and environment that produced it.
- Re-running one algorithm and republishing updates only that algorithm's results and
  provenance, leaving the others intact and truthful.
- A timeout is visible as a timeout and contributes nothing to any aggregate.
- No page presents a cross-family gap comparison without its budget.
- Exactly one source of benchmark truth remains.

---

## 6. Operational note

A convenience skill exists for measuring a single algorithm and uploading it, because the
differential workflow is manual. It carries a safety requirement: it must refuse to publish
a single-solver re-run when shared code has changed since the other solvers' recorded
measurements, since that mixes incomparable numbers under one version label.
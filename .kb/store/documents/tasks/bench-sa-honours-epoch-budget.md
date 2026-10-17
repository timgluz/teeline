---
id: 01a12650-9cbe-7021-8707-f061aaa974e4
slug: tasks/bench-sa-honours-epoch-budget
title: "fix(tsp): make simulated annealing honour its iteration budget"
type: task
status: completed
priority: high
tags: [solver, benchmarks, correctness, sa]
---

## Overview

`simulated_annealing` combined its two stopping rules with `||` instead of `&&`:

```rust
while epoch < opts.heuristic.epochs || temperature > opts.min_temperature
```

With `||` the loop runs until **both** bounds expire, i.e. `max(epochs, schedule)`, so
`epochs` acted as a *floor* rather than a cap. `cooling()` is geometric
(`t * (1 - cooling_rate)`), so at the defaults (rate 0.0001, 1000 → 0.001) the temperature
schedule needs **~138,149 iterations**, well past the default 10,000 epochs. The consequences:

- Raising `--epochs` from 10k to 50k to 138k changed nothing — the schedule always outlasted it.
- A *small* `--epochs` did truncate the run, but left the temperature near its maximum
  (10k iterations leaves T ≈ 368 of 1000), so almost every move was accepted and the search
  was a random walk rather than annealing.

This is a user-facing correctness bug, not only a benchmarking obstacle: a user asking for
fewer iterations to save time got degraded quality, and one asking for more got identical work.

### Correction to the original diagnosis

This task was filed claiming "`--epochs` never acts as an upper bound" and citing a flat
berlin52 timing (0.203 / 0.201 / 0.205 / 0.190s across 10 → 10,000 epochs). That measurement was
confounded: berlin52 is small enough that ~10k iterations cost only ~17ms, which fixed process
startup swamps. Re-measured on a280 the flag demonstrably did change work (0.78s at 100 epochs
vs 0.93s at 10,000). The real defect is the `||` semantics described above, and the flat-timing
evidence should not be reused.

Note `epochs` could not exceed `epochs`, so the bound was never *violated* — it was simply not
the binding constraint, which is why this looked like a budget bug but is really a
bounds-composition bug.

## Goals

- Make the iteration count a real budget, without discarding the temperature schedule
- Keep the scheduling behaviour intentional and documented, whichever semantics are chosen
- Add a regression test that would have caught the flat runtime

## Decision (settled)

**Both bounds are strict caps: the run ends at whichever expires first.** Implemented by
combining them with `&&`.

Rationale: `docs/algorithms/simulated-annealing.md` documents the temperature schedule as the
primary stopping rule (pseudocode `until T ≤ T_end`; `--min_temperature` described as "Stopping
temperature") with `--epochs` as "Maximum iterations". Treating `epochs` as a true cap matches
that documented contract, and makes the flag behave as its name promises.

Consequence addressed: capping at the generic 10,000-epoch default would have truncated the
schedule early and regressed default quality badly (measured on a280: ~32k tours at a 10k cap
versus ~3.4k when the schedule finishes). The SA-specific default is therefore **150,000**,
above the ~138k the schedule needs, so the temperature schedule still stops a default run while
`--epochs` remains an effective cap. It is applied in `SAOptions::from_cli` because
`HeuristicOptions::from_cli` always yields the generic 10k default and would overwrite a value
placed in `SAOptions::default()`.

## Acceptance Criteria

- [x] Semantics decided and recorded in this task (see Decision above)
- [x] `--epochs` measurably changes work performed — asserted by iteration count, not wall time
- [x] Unit test fails before the fix and passes after (verified: 138,149 vs 1,000 iterations)
- [x] Existing SA quality tests still pass; default quality preserved (~3.4k on a280, as before)
- [x] `cargo test` (15 suites), `cargo clippy --workspace -- -D warnings` clean
- [x] `docs/algorithms/simulated-annealing.md` updated: `||`→`&&` semantics, epoch default

## Notes

Found while planning the benchmarking effort (`decisions/benchmarking-design`), which needs a
working iteration budget before a convergence run means anything.

## Progress Log

### 2026-10-12

- **Corrected the filed diagnosis.** The claim "--epochs never acts as an upper bound" was
  wrong; the loop ran `max(epochs, schedule)`, so `epochs` was a floor. The original flat-timing
  evidence was an artifact of berlin52 being too small to discriminate — 10k iterations cost
  ~17ms against a fixed startup cost of ~180ms.
- **Measured the schedule length** by hand and confirmed in code: geometric cooling at rate
  0.0001 takes ~138,149 iterations to move 1000 → 0.001.
- **Decided semantics with the user**: both bounds are strict caps (`&&`).
- **Implemented**: `||` → `&&` in `solve()`.
- **Made the bound observable.** Added a test-only, thread-local iteration counter
  (`#[cfg(test)]`), because the budget was previously unobservable except by timing the process,
  which is exactly why the defect survived. Thread-local matters: `cargo test` runs tests in
  parallel and a shared `static` mixed counts across tests (observed as a flaky failure).
- **Regression test** `test_sa_epochs_is_a_strict_cap` asserts an exact iteration count.
  Verified it fails on the old code: `expected exactly 1000 iterations, ran 138149` — matching
  the hand-computed schedule length exactly, which independently confirms both the diagnosis and
  the fix.
- **Preserved default quality.** Capping at the generic 10k default regressed a280 from ~3.4k to
  ~32k tours. Raised the SA-specific default to 150,000 in `SAOptions::from_cli`; default quality
  is back to ~3.4k while `--epochs` now caps correctly (0.007s / 0.106s / 0.861s at 100 / 10k /
  150k).
- Files touched: `src/tsp/simulated_annealing.rs`, `src/tsp/mod.rs`,
  `docs/algorithms/simulated-annealing.md`.

## Completion Evidence

- **Commit**: `72007e0` — `fix(tsp): make simulated annealing honour its iteration budget`
- **Merged**: PR [#560](https://github.com/timgluz/teeline/pull/560) → merge commit `ea7f7cb`
  on `master` (2026-10-10)
- **Review**: five `ocr` passes (findings 5 / 4 / 5 / 4 / 1), all addressed. The substantive ones:
  `epochs == 0` silently became zero iterations under `&&` and returned the initial tour; the cap was
  hardcoded and applied only on the CLI path, so `from_toml`/api/qt/wasm regressed; `--min_temperature 0`
  produced a non-terminating loop; and a stray duplicated `#[test]` attribute double-registered a test.
- **Comments**: trimmed from 99 to 47 added lines once the why-not-what rule was recorded — they had
  narrated the bug history, which belongs in this task and the commit messages instead.
- **Files**: `src/tsp/simulated_annealing.rs` (condition + counter + 2 tests),
  `src/tsp/mod.rs` (SA-specific `epochs` default in `from_cli`),
  `docs/algorithms/simulated-annealing.md` (semantics + defaults)
- **Regression test proven to catch the bug**: with `||` restored, `test_sa_epochs_is_a_strict_cap`
  fails with `expected exactly 1000 iterations, ran 138149`; with `&&` it passes. The 138,149
  figure was derived independently by hand from the cooling schedule, so the test and the
  analysis corroborate each other.
- **Default quality preserved** (the main risk of making `epochs` binding): a280 tours at
  default settings ~3.3–3.5k, matching the pre-fix schedule-driven behaviour (~3.4k). A naive
  fix at the generic 10k default measured ~32k, a ~9x regression, which is why the SA default was
  raised to 150,000.
- **`--epochs` now caps**, confirmed end to end on a280: 0.007s / 0.106s / 0.861s at 100 / 10,000
  / 150,000 epochs, with quality improving 34966 → 31436 → 3585.
- **Gates**: `cargo test` 15 suites green; `cargo clippy --workspace -- -D warnings` clean;
  `cargo fmt --all --check` clean; web suite 486 tests green (algorithm docs are consumed by the
  web build); markdownlint/prettier pass via the pre-commit hook.

### 2026-10-12

- Task completed. `epochs` is now a strict cap rather than a floor, and the SA-specific default
  was raised so the temperature schedule still governs a default run.

## Unblocks

Closing this removes the blocker from:

- `tasks/bench-plateau-stop-all-iterative` (also blocked by nothing else now — **ready to start**)
- `tasks/bench-platoo-epochs-help-name` (**ready to start**)
- `tasks/bench-measurement-harness` (still blocked by the two above)

### 2026-10-10 — closed

- Merged as `ea7f7cb`. Task marked completed.
- Note for future work: the epoch budget's residual limitation (it can bound a run at the schedule
  length or leave it uncapped via `0`, but cannot *shorten* a run) is documented in `usable_epochs`.
  Lifting it needs `Option<usize>` to distinguish "unset" from "explicitly small".

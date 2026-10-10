---
id: 01a1273a-ef08-7462-b2da-bdef1659b8ba
slug: tasks/bench-lk-use-shared-plateau-option
title: "refactor(lk): use the shared plateau option instead of platoo_epochs"
type: task
status: draft
priority: medium
tags: [tsp, refactor, plateau, breaking-change]
---

## Overview

`lin_kernighan` is the last solver not using the shared plateau option. It has its own
`HeuristicOptions::platoo_epochs` (note the historical misspelling), default **10** via
`LKOptions::default`, used as a *restart* threshold: it stops after that many consecutive
non-improving restarts.

`stagnation_epochs` was built for exactly this, and every other solver now uses it. Migrating LK would
retire the duplicate option, the misspelled field, and the `--plateau_epochs` flag with its hidden
`platoo_epochs` alias.

## Why it is not a mechanical change

The two options have **different defaults**, so a naive swap silently changes LK's default path:

| | default | meaning |
|---|---|---|
| `platoo_epochs` | 10 (via `LKOptions::default`) | stop after 10 non-improving restarts |
| `stagnation_epochs` | 0 | never stop early |

`LKOptions::default` carries `platoo_epochs: 10` as a deliberate override of the generic
`HeuristicOptions` default of 500, so LK stops early out of the box while other solvers do not. Under
`stagnation_epochs`'s "0 = disabled" convention, moving LK over without preserving that would leave it
running its full restart budget by default.

So the compatibility call must be made explicitly, and it has a user-visible consequence either way:

1. **Preserve the behaviour** — set `LKOptions::default().stagnation_epochs = 10`. LK's default run is
   unchanged, but the flag now means the same thing it means everywhere else, and `--plateau_epochs`
   disappears.
2. **Adopt the convention** — leave it 0, so LK only stops early when asked. Consistent, but changes
   default results and run times for existing users.

Option 1 is probably right, but it is a judgement about a delivered solver's behaviour, not a
refactor.

## Goals

- LK stops through `Budget` like every other solver
- One option expresses "stop when progress stalls" across the whole crate
- No behaviour change that a caller did not ask for

## Acceptance Criteria

- [ ] `lin_kernighan` uses `Budget`, with its restarts counted the way the other solvers count epochs
- [ ] The `stagnation_epochs` vs `LKOptions::default` decision is made explicitly and recorded
- [ ] `HeuristicOptions::platoo_epochs` and the `--plateau_epochs` flag are removed, along with the
      hidden `platoo_epochs` alias, once nothing reads them
- [ ] `docs/algorithms/lin-kernighan.md` is updated: it currently documents `platoo_epochs`, a default
      of 10, and that the value is "≥ 0 (unvalidated)"; the plateau sentence and the options table both
      refer to it
- [ ] A test pins LK's convergence behaviour and its default
- [ ] `cargo test --workspace`, clippy, and the wasm and Qt crates all build

## Notes

- Found completing `tasks/bench-plateau-stop-all-iterative`. All ten solvers now use `Budget`; this is
  the one loose end, and it was left out of #568 because it changes a delivered default rather than
  wiring a new one.
- The CLI flag is currently `--plateau_epochs` with `platoo_epochs` as a hidden alias, so scripts
  passing either spelling need considering before removal.
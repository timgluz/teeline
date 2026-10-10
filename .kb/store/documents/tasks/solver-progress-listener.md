---
id: 01a126de-0734-7a40-b552-369ed5f2960b
slug: tasks/solver-progress-listener
title: "refactor(tsp): replace progress_tx channels with a listener seam"
type: task
status: draft
priority: low
tags: [tsp, refactor, events, observability]
---

## Overview

Every solver takes `progress_tx: Option<&mpsc::Sender<ProgressMessage>>` — 24 files, one identical
signature — and reports progress by sending on a concrete channel. Proposed: replace that with an
observer/listener seam so a solver publishes events without knowing who consumes them.

## Why it is worth doing

- **The concrete channel is threaded through 25 signatures.** `Option<&mpsc::Sender<..>>` means every
  solver, the pipeline, and every test carries a transport detail. A solver calling `tx.send(..)`
  cannot be exercised without constructing a channel, which is why the only real consumer is threaded
  all the way down.
- **Exactly two consumers exist.** `teeline-qt/src/solver_engine.rs` (drains on a thread) and
  `tests/som_test.rs` (collects into a `Vec`). Twenty-four producers, two consumers — the seam is
  carrying a lot of surface for very few listeners, and adding a second consumer today means another
  channel per solver.
- **Multiple listeners are the point.** Today a second observer (a logger, a metrics sink, a CLI
  progress bar) requires a second channel parameter and a second send at every emission site.
- **Test-only instrumentation exists *because* there is no seam.** `simulated_annealing.rs` still
  carries a `#[cfg(test)] thread_local! ITERATIONS` counter to observe how many iterations ran. The
  equivalent for GA was removed in favour of a testable `Budget`, but SA's remains — a listener could
  observe that directly instead of a global.

## Goals

- Solvers publish progress events without depending on `mpsc` or on a concrete receiver type
- One emission site reaches zero, one or many listeners, with a no-op implementation when nobody is
  listening (rather than `Option` plumbing at every call site)
- A test can capture events with a plain in-memory collector, with no channel and no thread
- Retire the test-only counters that exist only to make solver internals observable

## Acceptance Criteria

- [ ] A listener/observer abstraction exists, and solvers depend only on it
- [ ] `progress_tx: Option<&mpsc::Sender<ProgressMessage>>` is gone from solver signatures
- [ ] The Qt engine and the SOM test work through the new seam
- [ ] A no-op listener removes the need for `Option` checks at emission sites
- [ ] `simulated_annealing`'s test-only `ITERATIONS` counter is removed in favour of observing events
- [ ] An emission that no listener wants costs nothing measurable (no allocation on the hot path)
- [ ] `cargo test --workspace`, clippy and the wasm and Qt crates all build
- [ ] Existing progress semantics are preserved: `EpochUpdate`, `PathUpdate`, `Done`, `Restart`,
      `OptimalTour`, `CityChange` all still reach consumers

## Design questions to settle first

1. **Trait object vs generic parameter.** `&dyn ProgressListener` is simpler to thread and store;
   a generic `L: ProgressListener` can be monomorphised but infects every signature and every
   `Option`. Probably a trait object with a no-op implementation.
2. **Method per event vs one `emit(ProgressMessage)`.** Per-event methods avoid allocating a message
   for listeners that ignore it; a single `emit` keeps the surface small and reuses the existing enum.
3. **Hot-path cost.** Emission sites are inside solver loops, so the listener must be cheap to call
   when it ignores the event. Worth measuring on `nn`/`2opt` rather than assuming.
4. **Whether `ProgressMessage` stays an enum.** It is a transport-agnostic description already, so it
   can likely be reused as the event payload.
5. **Threading.** The Qt consumer drains on a separate thread; a listener called synchronously from
   the solver changes that model. Decide whether the Qt side keeps its channel behind an adapter
   listener, or the solver becomes responsible for delivery.

## Notes

- Found while adding the plateau stop (`tasks/bench-plateau-stop-all-iterative`), where the absence of
  an observation seam forced a test-only global. Related in kind to that work but independent of it —
  this touches every solver's signature, so it wants its own PR and its own review.
- Not urgent and not a blocker for benchmarking. Registered so the idea is not lost.
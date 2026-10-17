---
id: 019fbdbc-55d2-7072-9057-89ab286fddfb
slug: tasks/fourier-scaling-optimizer-experiments
title: "Fourier solver: scaling and optimizer experiments"
type: task
status: completed
priority: medium
parent: tasks/gh-185-fourier-solver
tags: [solver, fourier, performance, research]
---

## Overview

Post-implementation investigation into why the Fourier-basis solver's quality degrades
badly on larger instances, discovered while benchmarking for the
["Fourier algorithm: how it came to be" blog post](../../teeline-web/src/content/blog/fourier-algorithm-how-it-came-to-be.md)
(PR #369). On a280 (280 cities), Fourier standalone lands at +103.4% gap, more than double
the optimal tour length, versus +13.3% on berlin52 (52 cities). This task tracks the
follow-up experiments into why, and what (if anything) is worth changing.

## Goals

- Characterize whether the a280 blowup is a capacity problem (too few coefficients / curve
  samples) or an optimization problem (the gradient descent doesn't converge well)
- Evaluate whether a different optimizer (Adam) improves convergence
- Find a computationally practical way to improve quality on larger instances, if one exists
- Document findings either way, including dead ends, so this isn't re-investigated from scratch

## Acceptance Criteria

- [x] Determine whether `k_max` or `m` scaling matters more for quality on a280
- [x] Test an Adam-based coefficient update against the shipped plain gradient descent
- [x] Investigate replacing the brute-force `nearest_sample` search (O(n·m) per gradient
      step) with something faster, e.g. a KD-tree or binary search over curve samples,
      since that's what makes `m` scaling currently too expensive to be practical
- [x] Decide whether any of this should change production defaults, get exposed as new
      CLI/config knobs, or just stay documented as a known limitation — decided: expose
      as manual `--k-max`/`--m` CLI flags + `[stage.fourier]` pipeline TOML, no defaults
      changed, no auto-scaling, no runtime wall-time guard. See
      [[tasks/gh371-fourier-cli-flags]] (GH #371, completed).
- [x] If a fix ships, update `docs/algorithms/fourier.md` and `docs/benchmarks.md` —
      done as part of [[tasks/gh371-fourier-cli-flags]]

## Progress Log

### 2026-08-01

**Scaling experiment** (berlin52 defaults: `k_max=4, m=200`; a280 has n=280 cities):

| Config | Standalone gap | Wall time |
| --- | ---: | ---: |
| baseline (k_max=4, m=200) | +103.4% | 0.72s |
| m scaled alone (k_max=4, m=4n=1120) | +117.8% | — |
| k_max=16, m=4n=1120 | +21.9% | 18.6s |
| k_max=24, m=4n=1120 | +16.6% | 31.0s |
| k_max=32, m=4n=1120 | +15.9% | 45.2s |
| **k_max=32, m=200** | +24.6% | **8.5s** |

Findings:
- `k_max` (coefficient count) is what actually matters, not `m` (sample resolution).
  Scaling `m` alone, without more coefficients, made quality *worse*.
- `m` scaling is expensive: the gradient step's attraction term does a brute-force
  nearest-sample search per city per gradient step, O(n·m). Scaling `m` to `4n` turned
  a 0.7s solve into 18–45s (25–65x slower) for what's meant to be a fast constructive
  solver.
- Standalone quality doesn't rank the same as quality after a 2-opt pass. `k_max=32,
  m=200` (worse standalone than `k_max=32, m=4n`) gave the *best* result after piping
  into 2-opt (+7.6%, vs +8.3% for the m-scaled version), at a fraction of the wall time.
  Since Fourier is mainly used as a warm-start for local search, this is the more
  relevant number.

**Adam experiment (dead end):** Implemented Adam (per-component, applied independently to
real/imaginary parts of each complex coefficient) as a drop-in replacement for the plain
gradient-descent update in `gradient_step`, tested against `k_max=32, m=200` on a280.

| Method | Standalone gap | +2-opt gap |
| --- | ---: | ---: |
| Plain GD, lr=0.05 (shipped default) | +24.6% | **+7.6%** |
| Adam, lr swept 0.001–0.12 (best: lr=0.02) | +25.4% | +8.8% |

Adam needed a much lower learning rate than plain GD's 0.05 to be competitive at all (at
0.05 it was +44.3%, far worse), since Adam normalizes each step to roughly unit size via
`m̂/√v̂` rather than scaling naturally with gradient magnitude. Even at its best swept
learning rate, Adam still lost to plain GD on both standalone and post-2-opt quality, and
the lr sweep was noisy/non-monotonic (30.1% → 25.4% → 33.5% across lr=0.015/0.02/0.025),
suggesting real sensitivity rather than a smooth landscape for it here.

Working hypothesis for why plain GD wins: this is full-batch gradient descent (every city
contributes to every step), not stochastic, so Adam's main benefit (smoothing noisy
per-sample gradients) doesn't apply. The coarse-to-fine schedule also resets which
coefficients are active each stage; Adam's momentum/variance accumulators for
newly-unlocked coefficients start cold exactly when they're unlocked, which likely works
against the coarse-to-fine idea rather than helping it. Not verified further; noting as
the leading theory, not a confirmed conclusion.

**KD-tree nearest-sample experiment (positive result):** replaced the brute-force linear
scan in `nearest_sample` with a KD-tree built from the current `gamma` samples each
gradient step (reusing the existing `src/tsp/kdtree.rs` module already used by
`nearest_neighbor.rs`, `KDPoint::new_with_id(j, [re, im])` to carry the sample index back
through the query), tested on a280:

| Config | Brute force | KD-tree | Speedup | Gap (brute / kdtree) |
| --- | ---: | ---: | ---: | --- |
| k_max=4, m=200 (shipped default) | 706ms | 187ms | 3.8x | +103.4% / +104.5% |
| k_max=32, m=200 | 8.52s | 4.25s | 2.0x | +24.6% / +25.0% |
| k_max=32, m=1120 (4n) | 45.3s | 19.2s | 2.4x | +15.9% / +15.8% |

Quality is unaffected (the tiny gap differences are f32-vs-f64 coordinate precision and
tie-breaking noise from the existing KD-tree, which stores coords as `f32`; the gradient
itself still uses the returned index to compute in `f64`, so this doesn't touch the
solver's stated `f64` numerical-stability guarantee). Speedup is real but well under the
theoretical O(log m) ceiling (~22x expected at m=1120 vs ~2.4x measured), most likely
because the tree is rebuilt from scratch every single gradient step (up to
`k_max * epochs` times per solve) rather than being amortized; tree-construction overhead
is eating most of the theoretical gain. A grid/bucket structure exploiting the fact that
curve samples are evenly spaced in parameter `s` (though not necessarily in 2-D space)
might beat a full KD-tree rebuild here, but wasn't tried.

**Conclusion so far:** capacity (`k_max`) is the real lever, not the optimizer (Adam is a
dead end for the plain drop-in version tested). The KD-tree swap makes `m` scaling 2-4x
cheaper with no quality cost, which combined with `k_max=32` on a280 takes the standalone
solve from 45.3s down to 19.2s. That's still far from "fast" for a solver whose whole
value proposition on smaller instances is being near-instant, so this alone isn't a
slam-dunk fix; it just makes further `m` scaling less punishing than it currently is.

All experiments were run as throwaway code (a scratch integration test file, and
temporary in-module tests in `src/tsp/fourier.rs`), not committed; nothing in the
production solver changed as part of this investigation. No decision has been made yet
on whether to ship any of this (new CLI flags for `k_max`/`m`, new defaults, or a smarter
spatial structure) — that's still open, see the acceptance criteria above.

### 2026-08-01 (later)

**Shipped the KD-tree swap**: [PR #370](https://github.com/timgluz/teeline/pull/370),
branch `feat/fourier-kdtree-nearest-sample`. No defaults or public API changed; pure
internal optimization. All existing tests pass unmodified; `docs/algorithms/fourier.md`
updated with the new complexity bound and the f32-precision note. Follow-up decided at
the same time: exposing `k_max`/`m` as CLI flags (the actual quality lever, per the
scaling experiment above) is scoped as a **separate PR**, since it's a user-facing
capability change rather than a pure optimization. See [[tasks/gh-185-fourier-solver]]
for context on why `k_max`/`m` currently aren't CLI-reachable at all (only via the REST
API's `configs.fourier`). Tracked as
[GitHub issue #371](https://github.com/timgluz/teeline/issues/371) — explicitly scoped
to need a proper brainstorm/design pass (auto-scaling vs. manual flags, validation
bounds, pipeline TOML support) before implementation, not a straight-to-code PR like #370
was.

### 2026-08-01 (much later)

While filing #374 (LK KD-tree candidate-list migration, discovered during #372), quoted
this task's own diagnosis of the rebuild-overhead gap (measured 2-4x vs theoretical
~22x) back as #374's cautionary precedent — then the user flagged that this diagnosis
itself was never acted on. Filed
[GitHub issue #376](https://github.com/timgluz/teeline/issues/376) specifically to close
that gap (rebuild-every-gradient-step overhead in `build_gamma_tree`/`gradient_step`,
`src/tsp/fourier.rs`), rather than leaving it as a known-but-unaddressed limitation in
this doc indefinitely. Same "must show measured improvement, not just theory" bar as
#374 and the original #370 measurement discipline.

## Context

- Parent: [[tasks/gh-185-fourier-solver]] (original implementation)
- Triggered by: [[project_fourier_blog_post]] memory / PR #369 benchmarking
- Relevant code: `src/tsp/fourier.rs` (`gradient_step`, `nearest_sample`, `compute_basis`)
- Relevant config: `FourierOptions` in `src/tsp/mod.rs` (`k_max`, `m`, `lambda`,
  `lambda_decay`, `lr`, `epochs`)

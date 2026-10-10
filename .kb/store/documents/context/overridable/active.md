---
id: 019e2683-38ae-7bc2-8884-87a795ed4eef
slug: context/overridable/active
title: "Active Context"
type: context
status: active
priority: medium
tags: [auth, webauthn, web]
---

## Current Focus

**Interactive explainers: series complete — every algorithm now has an explainer (2026-08-16).**

All 21 explainers shipped (10 pre-Astro + 11 added in the #429–#434 series: greedy-edge,
savings, aco, stochastic-hill, or-opt, 3-opt, christofides, branch-bound, bhk, 2-opt, nn),
plus a new `/algorithms/` index page. Local `master` synced to `314c82e`; tree clean.

**Next session candidates (nothing claimed yet):**
- **#438** — add a "Papers citing this dataset" section to problem pages (docs-algorithms
  content work, same pipeline as the complexity-column work)
- **#323** — teeline-excel Office Add-in (unrelated, untouched)
- **WebAuthn auth Phase 0** — parked; design accepted in
  `tasks/webauthn-auth-replace-clerk` (Pages Functions + D1 + `@simplewebauthn/server` scaffold)

Process reminders: every change ships as its own PR, verified with astro check + tsc +
vitest + **Playwright** (`.pw-browsers` chromium) + `ocr` review — see [[notes/agent-dev-notes]]
for the environment gotchas before starting.

## Incoming focus: solver benchmarking (2026-10-12)

**In progress. First task done; two more prerequisites are next.**

Spec: `decisions/benchmarking-design`. Implementation plan (untracked, by design):
`docs/superpowers/plans/2026-10-12-benchmark-all-algorithms.md`.

Goal: measure every solver on a capability-tiered instance set, publish versioned JSON, and
render per-algorithm results on each algorithm page. Re-run differentially per release, keeping
old versions for regression checking.

Nine tasks created, split from the plan:

| Task | Status / depends on |
|---|---|
| `tasks/bench-sa-honours-epoch-budget` | ✅ **completed** (`72007e0`) |
| `tasks/bench-platoo-epochs-help-name` | ✅ **completed** (#563) |
| `tasks/bench-plateau-stop-all-iterative` | **next** — plan written |
| `tasks/bench-measurement-harness` | plateau only (other two landed) |
| `tasks/bench-tier-a-campaign` | harness |
| `tasks/bench-publish-provenance` | campaign |
| `tasks/bench-algorithm-pages` | publish |
| `tasks/bench-docs-spec-update` | publish |
| `tasks/bench-single-solver-skill` | publish |
| `tasks/bench-regression-history` | publish (optional, may stay unstarted) |

Where to resume: **`tasks/bench-plateau-stop-all-iterative`** — the last prerequisite before the
harness. Plan: `docs/superpowers/plans/2026-10-12-plateau-stop.md`.

Planning finding worth not rediscovering: the task says "reuse the existing `platoo_epochs`
semantics", but the two implementations disagree — `lin_kernighan` **stops** on `platoo_epochs`,
`stochastic_hill` **restarts**. The agreed resolution is a separate `stagnation_epochs` stop knob
(decided with the user), leaving `platoo_epochs` as the restart threshold so hill climbing's search
is unchanged. `lin_kernighan` migrates to the new knob.

Cost driver: `platoo_epochs` has ~78 references across 9 files (lib, CLI, api models + service,
wasm bindings + lib, qt), so a parallel field follows that whole path. Also note SOM is `som.rs`,
not `kohonen_som.rs`, despite what the docs say.

Lesson from the SA fix, worth applying to the plateau work: the iteration budget was only
observable by timing the process, so a bound that never took effect went unnoticed — on a small
instance the compute was ~17ms against ~180ms of fixed startup. Assert **counts**, not wall time.
Also note `HeuristicOptions::from_cli` always returns the generic 10k `epochs`, overwriting
anything set in a solver's `Default` impl; per-solver defaults belong in that solver's `from_cli`.

Three things worth remembering when picking this up:

1. **Per-solver provenance is correctness-critical and must land before the first differential
   publish.** `algorithms/{id}.json` currently assumes one commit for the whole dataset, which
   becomes a false claim once only some algorithms are re-measured — and unlike a wrong number it
   cannot be reconstructed after publication.
2. **"Convergence" is not yet measurable.** Only `lin_kernighan` and `stochastic_hill` have a
   plateau stop, and `sa` ignores `--epochs` entirely (flat runtime at 10 vs 10000 epochs), so the
   three prerequisite fixes gate everything downstream.
3. **`si175` cannot be parsed** (`UPPER_DIAG_ROW` unsupported) — substitute rather than skip
   silently when building the tier set.

Related pre-existing gap: `gen-problem-pages.mjs` reads only the COMMENT line, so several problem
pages show `optimalCost: null` despite a full `.opt.tour` being present.


## Recent merges (master, 2026-08-16 session)

| Commit | PR | Change |
|---|---|---|
| `7115c57` | #468 | Stochastic Hill Climbing explainer (#433) — 8 cities, seeded scenarios |
| `fa7e8a8` | #469 | Or-opt explainer (#432) — Or-1/2/3 + two_opt_stuck scenario |
| `ac41b21` | #470 | 3-opt explainer (#434) — all 7 cases + pattern diagram |
| `2d79a37` | #471 | refactor: shared `explainer-cities.ts` (ocr finding) |
| `9dbe4a4` | #472 | Christofides explainer (#431) — pipeline + ≤1.5× ratio vs brute force |
| `8352111` | #475 | Bellman–Held–Karp explainer (#429) — DP table animation (absorbed #473) |
| `e3fd1b3` | #474 | Branch & Bound explainer (#430) — tree viz, closed-cycle fix |
| `314c82e` | #476 | topbar dropdown → `/algorithms/` index + Complexity column + aria-labels |

Closed as redundant this session: **#473** (menubar PR — empty diff, content absorbed into
the #475 squash). Docs consistency: `docs/algorithms/*.md` all carry a Complexity meta row;
`nav-data.ts` mirrors it and a vitest test enforces the match.

## Open PRs / queue

- **#323** (open): teeline-excel Office Add-in — unrelated, untouched.
- **#438** (open): "Papers citing this dataset" section on problem pages — top web candidate
  for the next session.
- **Blog follow-up** (planned, after auth ships): `tasks/blog-passkeys-instead-of-clerk` —
  story post "Passkeys instead of Clerk" (no code, ~2000-2500 words) + later technical-recipe post.

## Local environment notes

- Local `master` synced to `314c82e`; tree clean; `teeline-web` node_modules on astro 7.2.1.
- SSH to GitHub is broken here — pushes/fetches need
  `GIT_SSH_COMMAND="ssh -F /dev/null -i /home/timgluz/.ssh/id_ed25519 -o StrictHostKeyChecking=accept-new"`.
- `npx` is unusable (read-only npm cache) — call `./node_modules/.bin/*` directly.
- Playwright browsers live in the repo at `.pw-browsers` (gitignored); run e2e with
  `PLAYWRIGHT_BROWSERS_PATH=/home/timgluz/repos/teeline/.pw-browsers` (+ `--project=chrome`).
  `webmcp.spec.ts` requires a production build (WASM only in `dist/`).
- Full gotcha list: [[notes/agent-dev-notes]].


## Older solver context (archived)

Prior focus (PR #402 Clarke-Wright `cw` solver, gh-394..gh-407 follow-ups) is superseded —
see `tasks/gh394-clarke-wright-savings` and the solver table in `context/immutable/patterns`.

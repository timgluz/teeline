---
id: 019e2683-38ae-7bc2-8884-87a795ed4eef
slug: context/overridable/active
title: "Active Context"
type: context
status: active
priority: medium
tags: [web, publishing, benchmarks]
---

## Current Focus

**Problem pages now publish their own minimap and social card (2026-10-11).**

Every `/problems/{id}/` page used to render its map as inline SVG in the markdown, which made a
regeneration a 223k-line diff and gave all 111 pages the same `og:image`. The images are now R2
objects (`minimaps/v1/{id}.svg`, `minimaps/v1/og/{id}.png`) and the docs are frontmatter-only.

| PR | Merged | Change |
|---|---|---|
| #583 | `bd02f9ba` | `fix(tsplib)`: read all nine `EDGE_WEIGHT_FORMAT` layouts (was: `FULL_MATRIX`, `UPPER_ROW`, `LOWER_DIAG_ROW` only), which is what made `si175`/`si535`/`si1032` parse |
| #584 | `674ccec3` | `feat(web)`: serve the minimaps from R2; docs drop 19.6 MB of inline SVG for 58 KB of frontmatter; MDS layout for the matrix-only instances, density grid above 3000 cities |
| #585 | `b79b7c36` | `feat(web)`: a 1200x630 card per problem page, `og:image` + `Dataset.image` as a raster, publisher uploads **per object** |
| #586 | `a96a42eb` | `ci(web)`: the non-auth Playwright suite runs in CI (`e2e-web`), not only the auth specs |
| #587 | `350c5919` | `fix(scripts)`: the page generator scores `EUC_2D` tours with TSPLIB's `nint` rounding, and refuses a route that is not a permutation of the instance |
| #588 | `dd0ee444` | `fix(web)`: the publisher refuses to publish (or verify) a page that links a card the bucket cannot serve |

Closed with them: `tasks/problem-minimaps-all-instances` (superseded), `problem-minimaps-as-r2-assets`,
`problem-social-cards`, `tsplib-all-edge-weight-formats`, `ci-runs-only-auth-e2e`,
`fix-euc-2d-rounding`, `minimap-pipeline-guards`, `gh-286-api-key-auth`, `gh-185-fourier-solver`.

Two content limitations are recorded rather than fixed: `brg180`'s derived layout is uninformative
(its matrix has six distinct values; the page says the layout is derived), and `tsp225`'s route
scores 3916 with f64 coordinates against 3919 under teeline's f32 storage, because 124 of its 225
edges sit within 1e-6 of a `.5` rounding boundary.

## Solver benchmarking (2026-10-11)

**The pipeline is built and published.** `bench/v2/index.json` reports `tier: a`, 22 solvers,
12 problems, measured at `fe89b6a9`; the algorithm and problem pages render it (#574, #576, #577),
the index marks which instances have results (#582), and the raw runs are committed
(`bench/runs/tier-a.jsonl`, 1245 rows with a per-row `status`).

The three prerequisites this section used to list are done: the SA epoch budget (`72007e0`), the
plateau stop knob (`stagnation_epochs`), and `UPPER_DIAG_ROW` parsing (#583). Two lessons from that
work are worth carrying forward:

- **A bound that never takes effect is invisible unless you assert counts.** The SA budget could only
  be observed by timing the process, where ~17ms of compute sat against ~180ms of fixed startup.
- **Per-solver provenance is correctness-critical** — `algorithms/{id}.json` now records a
  `measurement` block per solver (`git_commit`, `measured_at`, `version`, `os`, `rust_version`),
  because a wrong provenance claim cannot be reconstructed after publication.

Open benchmark tickets, by what they need: `bench-tier-a-campaign` (its raw data answers most of its
criteria; one to-convergence config is unaccounted for), `bench-docs-spec-update` (`docs/benchmarks.md`
is still not retired), `bench-single-solver-skill`, `bench-regression-history`,
`bench-lk-use-shared-plateau-option` (`lin_kernighan` still does not use `stagnation_epochs`),
`ci-cli-e2e-bats-workflow` (the suite runs inside `rust.yml`, not as its own workflow).

## Next session candidates (nothing claimed)

- **#438** — a "Papers citing this dataset" section on problem pages. Top web candidate; same
  pipeline as the complexity-column and minimap work.
- **#555** (open PR, mergeable, checks green) — renovate: ignore preact major updates. Needs a merge
  decision, and its premise predates Astro 7.3.5.
- The 11 open `draft` tickets listed above, plus `blog-passkeys-instead-of-clerk` (story post is
  written and published; the technical-recipe follow-up is deliberately waiting on feedback) and
  `ship-missing-optimal-tours` (25 problems publish an optimum but ship no `.opt.tour`).

## Recent merges (master, 2026-08-16 session, kept for history)

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

## Local environment notes

- Local `master` synced to `dd0ee444` (the #588 merge); `teeline-web` on astro 7.3.5, node 24.21.
- Work in a worktree per task, and symlink the gitignored state it needs:
  `ln -s /home/timgluz/repos/teeline/data <wt>/data` and the same for `teeline-web/node_modules`.
  Remove the worktree when its PR merges — they carry a Rust `target/` (one was 4.6 GB).
- **Playwright browsers are at `teeline-web/.e2e-state/browsers`** (not `.pw-browsers`, which is
  gone): `PLAYWRIGHT_BROWSERS_PATH=/home/timgluz/repos/teeline/teeline-web/.e2e-state/browsers`.
- **A killed `astro dev` leaves `.astro/dev.json` behind**, and the next run refuses to start with
  "Another astro dev server is already running" naming a PID that no longer exists. `rm
  teeline-web/.astro/dev.json` before a Playwright run; `pkill -f "[a]stro/bin/astro"` (brackets, or
  it matches its own command line and kills the shell).
- The WebMCP specs need the jco-transpiled bindings; a fresh worktree lacks them and those specs
  fail there while CI passes. `tests/problem-minimaps.spec.ts` is self-contained.
- `npx <installed tool>` works; installing a *new* package fails (read-only npm cache), so use the
  binaries already in `node_modules`.
- SSH to GitHub needs `git -c core.sshCommand="ssh -F /dev/null"` per command (a system
  `ssh_config.d` file is owned by `nobody`).
- Review every PR with `ocr` after opening it, and pass `--from origin/master` — a worktree's local
  `master` is usually stale. Full gotcha list: [[notes/agent-dev-notes]].

## Older solver context (archived)

Prior focus (PR #402 Clarke-Wright `cw` solver, gh-394..gh-407 follow-ups) is superseded —
see `tasks/gh394-clarke-wright-savings` and the solver table in `context/immutable/patterns`.

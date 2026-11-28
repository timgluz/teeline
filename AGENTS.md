# CLAUDE.md

Guidance for agents working in this repository. The governing principles live in
[`constitution.md`](constitution.md); it wins over this file. Dated decisions live in
[`docs/adr/`](docs/adr/).

## What this is

Teeline is a learning-oriented, open-source TSP solver: a pure Rust solver library plus a
CLI, API, WASM component, Qt GUI, and Astro site. The authoritative algorithm catalogue is
`teeline solvers`; the full table is in [`README.md#algorithms`](README.md#algorithms).

## Start here

| Need | Read |
| --- | --- |
| Architecture, core types, data flow | [`docs/architecture.md`](docs/architecture.md) |
| Adding a solver, `Solution`, spatial queries, pipeline seeding, validation, tests | [`docs/patterns.md`](docs/patterns.md) |
| Build, test, run, convert, release commands | [`README.md`](README.md) and `task --list` |
| Algorithm page contract | [`docs/patterns.md#algorithm-pages`](docs/patterns.md#algorithm-pages) |
| Code comment style | [`docs/patterns.md#code-comments`](docs/patterns.md#code-comments) |
| Distance source semantics | [`docs/patterns.md#distance-source-semantics`](docs/patterns.md#distance-source-semantics) |
| Fixtures and test layout | [`docs/patterns.md#fixtures`](docs/patterns.md#fixtures), [`docs/architecture.md#build-and-test-layout`](docs/architecture.md#build-and-test-layout) |
| Benchmarks and publication | [`docs/benchmarks.md`](docs/benchmarks.md), [`docs/benchmarks/schema.md`](docs/benchmarks/schema.md) |
| Web design and explainers | [`docs/web-design.md`](docs/web-design.md), [`.agents/skills/explainer-workflow/SKILL.md`](.agents/skills/explainer-workflow/SKILL.md) |
| WASM build, web client types, release | [`docs/wasm.md`](docs/wasm.md) |
| Environment and git gotchas | [`docs/dev-notes.md`](docs/dev-notes.md) |
| Prose and blog posts | [`docs/writing-style.md`](docs/writing-style.md) |
| Agents and model policy | [`docs/agents.md`](docs/agents.md) |

## Hard rules

- **One dispatch path.** Every solver is a `Solvers` variant reachable through
  `solve_with_context`; register it in all places listed in `docs/patterns.md`.
- **The root `teeline` crate is the pure solver library.** No GUI or transport dependency.
- **Exact solvers are capped at 24 cities** (`EXACT_MAX_CITIES`). Never present them as
  general-purpose.
- **Fixtures live in `tests/fixtures/`.** `data/` is git-ignored; a test that reads it passes
  locally and fails in CI.
- **Algorithm pages are part of the contract.** A behaviour change bumps the page version and
  adds a changelog row in the same PR.
- **Code comments explain why, not what.**
- **Prose uses no em-dashes** and avoids the anti-slop markers in `docs/writing-style.md`.
- **One PR per change**, branched from `master` as `type/short-description`, conventional
  commits, opened with `gh pr create`. The pre-commit hook runs fmt/clippy/markdownlint/tsc.
- **After opening a PR, ask `open-code-review` (`ocr`) for a review.** Blog posts also get
  `blog-editor`; explainers get `explainer-build-reviewer` (see `docs/agents.md`).

## Environment

- `Taskfile.yml` is the canonical local entry point (`task --list`).
- SSH pushes need `git -c core.sshCommand="ssh -F /dev/null" push` on this machine.
- `git rebase --continue` can hang in vim; use `GIT_EDITOR=true`.
- Cloudflare: use the `cf` CLI unless the project has a Wrangler configuration.

## Graphify

When `graphify-out/graph.json` exists, run `graphify query "<question>"` before your first
grep or source read for a codebase question, and put that rule in every code-exploring
subagent prompt. Use `graphify path` for relationships and `graphify explain` for concepts.
Dirty graph files are expected; only skip graphify when the task is about stale graph output.
After code changes, run `graphify update .`. The committed `graphify-out/graph.json` is
rewritten whole on every build, so a large diff is expected and is not churn.

## Knowledge & planning

- `openspec/` — capability specs and proposed changes (`openspec list`).
- `docs/backlog.md` — pending work not tracked as an issue.
- `docs/algorithm-candidates.md` — algorithm research backlog; check it before researching a
  new solver.
- GitHub issues via `gh issue` / `gh issue create`.

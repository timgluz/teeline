# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Teeline is a Traveling Salesman Problem (TSP) solver written in Rust. It reads TSPLIB instances — city coordinates or explicit edge weights — from stdin or a file, and outputs the tour cost and ordered city IDs. The Qt 6 desktop GUI (`teeline-qt`) renders solver progress in real time; the library exposes progress as a `ProgressMessage` channel.

## Constitution

`constitution.md` at the repository root holds the project's non-negotiable principles and
constraints (vision, hard contracts, architecture/API guardrails, governance). Read it
before proposing or implementing a change. Where this file and the constitution disagree
on a matter of principle, the constitution wins; this file remains the operational how-to.

## Commands

```bash
# Build (CLI binary is in teeline-cli workspace member)
cargo build -p teeline-cli           # debug
cargo build -p teeline-cli --release # optimized

# Or use Taskfile tasks
task build          # debug
task build:release  # optimized

# Run all tests
cargo test

# Run a specific test
cargo test test_coords_from_text_only_ints

# Run (debug binary is target/debug/teeline, release is target/release/teeline)
cat ./data/tsplib/berlin52.tsp | ./target/debug/teeline solve nn
./target/debug/teeline solve lk -i ./data/tsplib/berlin52.tsp
./target/debug/teeline --help
./target/debug/teeline solve --help
./target/debug/teeline convert --help

# Convert DiscOpt files to TSPLIB format
./target/debug/teeline convert -i ./data/raw/tsp_51_1 -o ./data/discopt/
./target/debug/teeline convert -i ./data/raw/ -o ./data/discopt/
```

## Architecture

**Entry point:** the `teeline` binary is produced by the `teeline-cli` workspace member (`teeline-cli/src/main.rs`). It parses CLI args with `clap`, reads TSPLIB data, and writes `<total_distance> <optimized_flag>\n<city_id> <city_id> ...` to stdout. Solver progress is a library-side `ProgressMessage` channel; the Qt GUI (`teeline-qt`) consumes it.

**Library root:** `src/lib.rs` exposes `pub mod config` and `pub mod tsp`.

**Core types (all in `src/tsp/mod.rs`):**

- `Solvers` enum — lists every algorithm with string aliases (e.g. `"nn"` → `NearestNeighbor`, `"2opt"` → `TwoOpt`)
- `AppOptions` — pure config container; one optional option struct per solver plus shared `HeuristicOptions`
- `TspProblem` — cities plus `Arc<dyn DistanceSource>`; `new` wraps a concrete source, `from_source` shares one. `cities` is the 2D layout geometry-based solvers read; `DistanceSource::is_coordinate_metric()` reports whether the distance source is the coordinates rather than an EXPLICIT matrix
- `Solution` — tour route and total distance computed from the problem's distance source
- `DistanceSource` — the distance-query trait (`src/tsp/distance_source.rs`); `Position` is a dense `0..n` city index, city IDs stay external `usize`, and the shared defaults keep backends byte-identical
- `Position` — `#[repr(transparent)]` newtype over `usize` for dense `0..n` city positions (`src/tsp/position.rs`); a position cannot be passed where a city ID is expected
- `NearestResult` / `NearestResultItem` — returned by both KD-tree and distance-source nearest-neighbor lookups

**Spatial data structures:**

- `src/tsp/kdtree.rs` — KD-tree for fast nearest-neighbor queries; `KDPoint` is the shared city/point type used everywhere
- `src/tsp/distance_matrix.rs` — the `Matrix` backend: a packed O(n²) lower triangle implementing `DistanceSource`; a brute-force alternative to the KD-tree
- `src/tsp/distance_source.rs` — the `DistanceSource` trait, its shared defaults, and the backend contract
- `src/tsp/embedding.rs` — deterministic classical-MDS layout for EXPLICIT instances without coordinates, introduced by the `explicit-embedding` OpenSpec change

**Input:** `src/tsp/tsplib.rs` — state-machine parser for TSPLIB format; accepts `NODE_COORD_SECTION`, `DISPLAY_DATA_SECTION`, and `EDGE_WEIGHT_SECTION` (all nine `EDGE_WEIGHT_FORMAT` layouts); reads from file or stdin. EXPLICIT files without coordinates currently get placeholder grid coordinates; the `explicit-embedding` change replaces them with a deterministic MDS layout at parse time.

**Solvers** (each in its own file under `src/tsp/`; authoritative catalogue: `teeline solvers`, full table in [`README.md#algorithms`](README.md#algorithms)):

| File                      | Algorithm                                                                                                                                                                        | Alias          |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| `bellman_karp.rs`         | Bellman–Held–Karp (exact, exponential)                                                                                                                                           | `bhk`          |
| `branch_bound.rs`         | Branch and bound (exact by default; ignores `n_nearest`)                                                                                                                         | —              |
| `nearest_neighbor.rs`     | Greedy nearest neighbor via KD-tree                                                                                                                                              | `nn`           |
| `two_opt.rs`              | 2-opt local search                                                                                                                                                               | `2opt`         |
| `stochastic_hill.rs`      | Hill climbing with random restarts                                                                                                                                               | —              |
| `simulated_annealing.rs`  | Simulated annealing                                                                                                                                                              | `sa`           |
| `tabu_search.rs`          | Tabu search                                                                                                                                                                      | —              |
| `genetic_algorithm.rs`    | Genetic algorithm                                                                                                                                                                | `ga`           |
| `particle_swarm.rs`       | Discrete PSO with order-crossover moves, 2-opt reversal perturbation, and a final 2-opt polish (NN-seeded swarm)                                                                  | `pso`          |
| `cuckoo_search.rs`        | Cuckoo Search via Lévy flights (k random 2-opt reversals, Bernoulli nest abandonment)                                                                                            | `cs`           |
| `flower_pollination.rs`   | Flower Pollination Algorithm (Lévy-scaled order crossover toward gbest; ε-scaled crossover toward a neighbour + 2-opt reversal; NN-seeded; final 2-opt polish)                     | `fpa`          |
| `lin_kernighan.rs`        | Lin-Kernighan style ILS: candidate-list 2-opt + double-bridge kicks                                                                                                              | `lk`           |
| `memetic_eax.rs`          | Memetic Edge Assembly Crossover: population of locally optimised tours, AB-cycle recombination, subtour patching, dirty-edge 2-opt polish                                        | `eax`          |
| `or_opt.rs`               | Or-opt local search: relocates segments of 1–3 cities (best-improvement)                                                                                                         | `or_opt`       |
| `christofides.rs`         | Christofides ≤1.5× approximation: MST + greedy matching + Eulerian shortcut                                                                                                      | `christofides` |
| `gravitational_search.rs` | Gravitational Search Algorithm: mass-weighted order crossover toward a sampled Kbest attractor, greedy accept, 2-opt reversal, NN-seeded, final 2-opt polish                                  | `gsa`          |
| `fourier.rs`              | Fourier-basis constructive solver: closed-curve gradient descent + argsort decode                                                                                                | `fourier`      |
| `greedy_edge.rs`          | Greedy edge construction: Kruskal-style, sorts all edges shortest-first and greedily accepts unless it creates degree 3+ or a premature sub-cycle (uses `graph.rs`'s union-find) | `gec`          |
| `insertion.rs`            | Deterministic nearest, farthest (default), or cheapest insertion construction                                                                                    | `ins`          |
| `space_filling_curve.rs`  | Space-filling curve construction: Hilbert/Morton indexing over a normalized 2^p grid; O(n log n) geometric warm start                                                             | `sfc`          |
| `ant_colony.rs`           | Ant Colony Optimization (Ant System): pheromone- and heuristic-biased probabilistic tour construction; global evaporate + all-ants-deposit update per epoch                      | `aco`          |
| `three_opt.rs`            | 3-opt local search: exhaustive 2-opt-style reversal cases                                                                                                                         | `3opt`         |
| `savings.rs`              | Clarke-Wright savings: parallel savings construction from shortest edges                                                                                                          | `sav`          |
| `som.rs`                  | Kohonen self-organising map: competitive-learning constructive solver                                                                                                             | `som`          |

**Tests:**

- Unit tests live inline in each source file (`#[cfg(test)]`)
- Integration tests in `tests/` use the public library crate
- Test helpers (approximate float comparison, etc.) in `src/test/helpers.rs`
- EXPLICIT fixtures: `gr17.tsp` and `ring6_explicit.tsp` have no coordinates; `bays29.tsp` has a `DISPLAY_DATA_SECTION`
- **Fixtures must live in `tests/fixtures/`.** The `data/` tree is git-ignored, so a test that
  reads `data/tsplib/<instance>.tsp` passes locally and fails in CI. Copy the instance into
  `tests/fixtures/` and reference it from there (e.g. `tests/fixtures/kroA100.tsp`).

## Important Notes

- The CLI binary is produced by the `teeline-cli` workspace member (`cargo build -p teeline-cli`). The binary is named `teeline` (at `target/debug/teeline` and `target/release/teeline`). Any older `target/release/bin` is a stale artifact from before the workspace split — ignore it.
- `bellman_karp` and `branch_bound` are exact algorithms with factorial/exponential complexity. The dispatcher rejects inputs above 24 cities (`EXACT_MAX_CITIES`).
- TSPLIB parsing normalizes all keys to uppercase and lowercases metadata values; city coordinates are stored as `f32`.
- `DistanceSource::is_coordinate_metric()` means "distances are computed from these coordinates" — it is `false` for every EXPLICIT matrix, even when the file supplies `DISPLAY_DATA_SECTION`. `two_opt`/`or_opt` use it to stay on the exact full scan. `TspLibData.has_real_coordinates` is the separate "the file supplied coordinates" flag.
- `teeline convert` converts DiscOpt coordinate files (first line ignored, remaining lines are `x y` pairs) to TSPLIB EUC_2D format. It replaces the old `convert2tsplib.py` script.
- `download_data.sh` fetches benchmark datasets.

## Code Comments

Comments explain **why**, not **what**. The code already says what it does — a reader can see
that. A comment earns its place only by adding something the code cannot express:

- **Good**: a non-obvious invariant, a deliberate trade-off, a workaround with its cause, why a
  constant has its value, why an approach was rejected, a surprising edge case.
- **Bad**: restating the line below it, narrating a sequence ("first we loop, then we return"),
  describing the signature, or repeating what a well-named function or test already conveys.

Prefer expressing intent through naming and structure over prose. Keep comments short; delete any
sentence that would still be obvious to someone reading the surrounding code. When a comment is
needed, put it next to the code it explains and point at the specific thing that is surprising.

Historical narrative does not belong in source comments — if a bug's story matters, the task,
PR description or commit message is the place for it.

## Algorithm pages & changelogs

A solver's page under `docs/algorithms/` is part of its contract. Any change that alters
observable behaviour — default parameters, search semantics, output quality, determinism,
registration, or a guard that rejects inputs — must update that page in the same PR:

- Bump the `version` in the page frontmatter. Follow the rule in
  `teeline-web/src/content.config.ts`: a behaviour-changing rewrite bumps the major;
  smaller corrections use minor or patch.
- Add a `## Changelog` row with the date and the user-visible change, including new
  benchmark numbers when the change affects them.
- Keep the description, options, usage and benchmark notes consistent with the new
  behaviour. Removed or ignored flags must be documented as such.

The changelog is where behavioural history belongs; source comments carry only the
non-obvious *why* (see Code Comments).

## teeline-web WASM Development Gotcha

`teeline-web/src/teeline-wasm.d.ts` is an **ambient module declaration** that TypeScript uses as the sole source of truth for `'teeline-wasm'` imports — it completely shadows `node_modules/teeline-wasm/teeline_wasm.d.ts`. **Any new WIT export must be added to this ambient file**, or tsc will fail with "Module 'teeline-wasm' has no exported member '...'".

After adding a new WIT export:

1. Build WASM: `cargo component build --manifest-path teeline-wasm/Cargo.toml --target wasm32-wasip2`
2. Run jco: `npx jco transpile ...` (see Taskfile)
3. **Update `teeline-web/src/teeline-wasm.d.ts`** with the new export's TypeScript type

## Git & PR Workflow

- Branch off `master` as `type/short-description` (e.g. `fix/web-opt-tour-url`, `feat/algorithms-index`, `docs/agent-git-pr-workflow`, `chore/web-prettier`).
- Use Conventional Commits scoped by area, e.g. `fix(web): ...`, `build(deps): ...`.
- **Open pull requests with `gh pr create`** (the `gh` CLI is authenticated as `timgluz`) — do not create PRs by hand.
- The `.githooks/pre-commit` hook runs cargo fmt/clippy, markdownlint, `tsc`, Prettier, etc. on staged files; let it run and fix any failures before committing.
- SSH pushes can fail with `Bad owner or permissions on /usr/etc/ssh/ssh_config.d/50-suse.conf` (that system file is owned by `nobody`). Work around it per-command without changing repo or remote config:
  `git -c core.sshCommand="ssh -F /dev/null" push -u origin <branch>`

## Releases & web deployment

- **Version bump:** update `Cargo.toml` and `teeline-cli/Cargo.toml` together. Bump
  `teeline-wasm/Cargo.toml` when the WASM changes; `/solve/` displays it as
  `teeline-solver <version>` (`env!("CARGO_PKG_VERSION")`).
- **Tag:** push `vX.Y.Z` matching the manifests. `.github/workflows/release.yml` builds the CLI
  artifacts, WASM, Qt bundles, and the GitHub Release.
- **Web:** a successful tag release triggers `.github/workflows/deploy-web.yml` via
  `workflow_run`. It downloads the latest release's `teeline-solver.wasm`, transpiles with jco,
  and deploys. Manual redeploy: `gh workflow run deploy-web.yml --ref master`.
- The web solver list comes from the WASM's `list_algorithms`, so a new solver reaches `/solve/`
  only after that deploy. `deploy-web` also runs on `teeline-web/**` pushes; the `workflow_run`
  guard ignores PR runs of the Release workflow.

## Knowledge & planning

- `constitution.md` — non-negotiable project principles.
- `openspec/` — capability specs and proposed changes (`openspec list`, `openspec new change`).
- `docs/adr/` — dated architecture decisions.
- `docs/backlog.md` — pending work not tracked as a GitHub issue.
- `docs/algorithm-candidates.md` — shared algorithm research backlog (state + references); check it before researching a new solver.
- GitHub issues — use `gh issue` / `gh issue create` for new tracked work.

## Cloudflare

When interacting with Cloudflare, use the `cf` CLI unless the project has a
Wrangler configuration file.

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

When the user types `/graphify`, use the installed graphify skill or instructions before doing anything else.

Rules:

- When graphify-out/graph.json exists, run `graphify query "<question>"` before your first grep, search or source-file read for a codebase question, and put this rule in every subagent prompt that explores code. Read raw files only for lines the graph or an earlier read already located. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- Dirty graphify-out/ files are expected after hooks or incremental updates; dirty graph files are not a reason to skip graphify. Only skip graphify if the task is about stale or incorrect graph output, or the user explicitly says not to use it.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
- A committed graphify-out/graph.json is rewritten whole on every build, so a large diff to it in a pull request is expected. Do not flag it as churn or suggest skipping the graph rebuild or its hooks.

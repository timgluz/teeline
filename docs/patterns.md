# Implementation Patterns

Conventions that keep new code consistent with the existing codebase. For the
non-negotiable rules behind them, see [`constitution.md`](constitution.md).

## Adding a solver

Every algorithm is a variant of the `Solvers` enum in `src/tsp/mod.rs`. Register it in
four coordinated places:

1. the `Solvers` variant,
2. the `FromStr` impl (canonical name and aliases),
3. the `solve_with_context` match arm,
4. `Solvers::all_meta()` (the CLI/UI catalogue).

Then add `src/tsp/<solver>.rs` with a `solve(...)` function, expose it as
`pub mod <solver>;` in `src/tsp/mod.rs`, and add documentation plus tests. A solver that
is added without all four registration points will be unreachable or invisible.

## Data flow

```text
stdin / file → tsplib parser → Vec<KDPoint> → TspProblem → solver → Solution → stdout
                                                          ↓
                                            ProgressMessage channel → consumer
```

City IDs are 1-based (TSPLIB convention) and coordinates are `f32`.

## Spatial queries

All nearest-neighbour lookups go through `KDTree` (preferred) or `DistanceMatrix`
(brute-force fallback). Both expose `nearest(&pt, n)` returning `NearestResult`.
Solvers that need candidate edge lists should use `KDTree`, not a hand-rolled scan.

## Solution contract

`Solution` is constructed from a route and a `TspProblem` (or from parts inside the
crate). It computes `total` from the distance matrix at construction. Do not mutate a
tour in place and expect the cost to follow: build a new `Solution` when the route
changes.

`validate_tour(tour, cities)` is the shared check that a tour is a permutation of the
instance's city IDs. The pipeline validates once at entry rather than in every solver.

## Pipeline seeding

Local-search and stochastic solvers warm-start from a constructive seed rather than a
random or sequential tour. The CLI auto-expands:

| Solver type | Auto-expands to |
| --- | --- |
| Deterministic local search (`2opt`, `3opt`, `tabu`, `or_opt`, `lk`, `branch_bound`) | `pipeline(nn, solver)` |
| Stochastic (`sa`, `stochastic_hill`, `ga`, `pso`, `cs`, `fpa`, `gsa`, `fourier`, `aco`) | `pipeline(shuffle, solver)` |
| Constructive (`nn`, `christofides`, `greedy_edge`, `sfc`, `bhk`) | no expansion |

Use `--no-seed` to opt out. Custom chains are available through
`teeline pipeline --steps=...`; the named presets are `fast`, `classic`, and `thorough`.

## Configuration and validation

`AppOptions` is the pure config container. Each solver reads its own option struct,
falling back to `Default` when the section is omitted. Every option struct exposes
`validate(&self) -> Result<(), String>`; call it before running the solver. Validation is
symmetric across CLI and TOML paths (see [ADR-005](adr/005-validation-approach.md)).

Option fields whose `0` value is meaningful (for example `epochs = 0` meaning "run until
the schedule stops") must document that sentinel and must not be rejected by
`validate()`.

## Testing

- Unit tests live inline in each module (`#[cfg(test)]`).
- Integration tests live in `tests/` and use the public library crate.
- Quality checks compare against TSPLIB known optima (berlin52 = 7542, att48 = 10628,
  bayg29 = 1610).
- Use `test::helpers::assert_approx` for float comparisons.
- CLI end-to-end tests are BATS scripts in `tests/e2e/`.

## Progress reporting

Solvers take `Option<&mpsc::Sender<ProgressMessage>>` and send updates without blocking
the solver. `ProgressMessage` lives in the library because it is the transport-agnostic
contract; the Qt GUI, the API and tests are the consumers (see
[ADR-008](adr/008-qt-gui.md)).

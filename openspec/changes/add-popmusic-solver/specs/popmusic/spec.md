# Spec Delta

## Purpose

Provides a deterministic POPMUSIC decomposition solver that improves a tour by exactly
re-optimizing short contiguous windows with fixed endpoints and splicing them back.

## ADDED Requirements

### Requirement: Valid tour construction

The system SHALL provide a POPMUSIC solver that returns a complete tour visiting every city exactly once, and whose reported cost equals the tour length recomputed from the problem's distance source.

#### Scenario: Valid permutation on a benchmark instance

- **WHEN** the solver runs on `berlin52`
- **THEN** the tour contains each city ID exactly once and the reported cost equals the recomputed tour length

#### Scenario: Tiny instances

- **WHEN** the problem has three or fewer cities
- **THEN** the solver returns a valid tour without entering the window loop

#### Scenario: Explicit matrix instance

- **WHEN** the solver runs on `gr17` (symmetric `EXPLICIT`)
- **THEN** it returns a valid tour whose cost is computed from the matrix, not from the derived layout

### Requirement: Exact window optimization

The system SHALL re-optimize the interior of each window exactly, keeping the window's first and last cities fixed, and SHALL accept a window result only when it strictly shortens the window's open path beyond a floating-point tolerance.

#### Scenario: Endpoints stay fixed

- **WHEN** a window is re-optimized
- **THEN** only interior cities change position and the window's first and last cities are unchanged

#### Scenario: Improvement accepted

- **WHEN** the exact interior order shortens the window's open path beyond the tolerance
- **THEN** the tour is updated with the new interior order

#### Scenario: Non-improvement rejected

- **WHEN** the exact interior order does not shorten the window's open path beyond the tolerance
- **THEN** the tour is unchanged

### Requirement: Window scan and termination

The system SHALL scan cyclic windows of `min(14, n)` cities at the two fixed offsets `0` and half the window size, SHALL count both offsets as one pass, SHALL stop after four passes or as soon as a pass changes no window, and SHALL handle windows that wrap past the end of the tour.

#### Scenario: Early stop

- **WHEN** a pass makes no splice
- **THEN** the solver stops and returns the current tour

#### Scenario: Pass cap

- **WHEN** every pass makes at least one splice
- **THEN** the solver stops after four passes

#### Scenario: Wrap-around window

- **WHEN** a window crosses the end of the tour representation
- **THEN** its cities are read and written with modular indexing and the tour remains a valid permutation

#### Scenario: Idempotence

- **WHEN** the solver runs on a tour produced by the solver
- **THEN** it makes no further splices and returns the same tour

### Requirement: Seeding and pipeline composition

The system SHALL consume a supplied warm-start tour as its starting tour and, when none is supplied, SHALL start from a nearest-neighbour tour; the CLI SHALL auto-expand a standalone run to `pipeline(nn, popmusic)`.

#### Scenario: Warm start consumed

- **WHEN** the solver is the second stage of `pipeline(nn, popmusic)`
- **THEN** it starts from the nearest-neighbour tour and the final cost is no worse than that seed

#### Scenario: Standalone run

- **WHEN** the solver is invoked without a warm start
- **THEN** it builds a nearest-neighbour tour internally and returns a valid result

#### Scenario: Automatic prefix

- **WHEN** the CLI resolves the solver's automatic seed behaviour
- **THEN** it prepends `nn` unless seeding is disabled

### Requirement: Deterministic output

The system SHALL produce the same tour for the same problem, options and starting tour on every run, using no random number generator.

#### Scenario: Repeated runs match

- **WHEN** the solver runs twice on the same problem and starting tour
- **THEN** both runs return the same route and cost

#### Scenario: Tie-breaking is stable

- **WHEN** several interior orders tie at the same cost
- **THEN** the same order is chosen on every run

### Requirement: Solver registration and aliases

The system SHALL register the solver as `popmusic` with alias `pop-music`, advertise it as a metaheuristic solver with no options, and classify it in the benchmark `meta` family.

#### Scenario: Aliases resolve

- **WHEN** a user supplies `popmusic` or `pop-music`
- **THEN** both resolve to the same solver

#### Scenario: Metadata advertises the solver

- **WHEN** the solver list is queried
- **THEN** it includes the solver as a metaheuristic with no options and a complexity description

#### Scenario: Benchmark family

- **WHEN** the benchmark index is read
- **THEN** the solver is listed in the `meta` family with an algorithm doc page

### Requirement: Cross-surface exposure

The system SHALL expose the solver through the CLI, the HTTP API, the WASM bridge and the website algorithm index, and SHALL list it alongside the existing solvers.

#### Scenario: CLI solves

- **WHEN** the user runs `teeline solve popmusic -i <instance>`
- **THEN** the CLI returns a valid tour

#### Scenario: API accepts the solver

- **WHEN** an API request selects `popmusic`
- **THEN** the service solves the instance and returns a valid tour

#### Scenario: WASM lists and runs the solver

- **WHEN** the WASM algorithm list is queried or the solver is invoked
- **THEN** it appears as a metaheuristic with no parameters and produces a valid tour

#### Scenario: Web indexes the solver

- **WHEN** the web algorithms index renders
- **THEN** it includes the solver under the metaheuristic group with a link to its algorithm page

### Requirement: Tour quality on TSPLIB benchmarks

The system SHALL run within the meta-family 300-second budget on every Tier A instance, SHALL return a valid tour for every instance, and SHALL improve on the nearest-neighbour seed gap on `berlin52` and `a280`.

#### Scenario: Valid on every Tier A instance

- **WHEN** the solver runs on the Tier A corpus within the meta budget
- **THEN** every run returns a valid tour and no run times out

#### Scenario: Improves the NN seed

- **WHEN** the solver runs on `berlin52` and `a280`
- **THEN** the tour cost is lower than the nearest-neighbour tour cost on both

#### Scenario: Recorded parameters

- **WHEN** the benchmark results are published
- **THEN** they record the chosen window size and the measured elapsed time

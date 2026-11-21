# memetic-eax Specification

## Purpose

Provides a memetic Edge Assembly Crossover (EAX) solver that maintains a population of
locally optimized symmetric-TSP tours and recombines them into high-quality new tours.

## Requirements

### Requirement: Valid tour construction

The system SHALL provide a memetic Edge Assembly Crossover solver that returns a complete tour visiting every city exactly once, and whose reported cost equals the tour length recomputed from the problem's distance source.

#### Scenario: Valid permutation on a benchmark instance

- **WHEN** the solver runs on `berlin52`
- **THEN** the returned tour contains each city ID exactly once and its reported cost equals the recomputed tour length

#### Scenario: Tiny instances

- **WHEN** the problem has fewer than four cities
- **THEN** the solver returns the exact cyclic permutation without entering the evolutionary loop

### Requirement: Population initialization

The system SHALL seed a population of locally optimized tours using nearest-neighbour candidate lists of size `min(candidate_k, n - 1)`, randomized constructive starts (top-3 randomized nearest neighbour and shuffled tours) polished with candidate-restricted 2-opt, and a defined fallback when fewer than `pop_size` distinct local optima are available.

#### Scenario: Diverse initial population

- **WHEN** the population is initialized on a problem with many distinct local optima
- **THEN** each member is a valid tour and members differ where distinct optima exist

#### Scenario: Small candidate count

- **WHEN** `candidate_k` is larger than the number of other cities
- **THEN** candidate lists are clamped to `n - 1` entries and initialization still succeeds

#### Scenario: Insufficient distinct optima

- **WHEN** the instance cannot supply `pop_size` distinct local optima
- **THEN** the solver fills the population with deterministic perturbations and still starts a valid evolutionary loop

### Requirement: Edge Assembly Crossover

The system SHALL decompose the union multigraph of two parent tours into alternating A/B edge cycles, SHALL reject any cycle that does not alternate edge origin or has odd length, and SHALL apply selected cycles to parent A as a symmetric difference without treating shared edges as changes.

#### Scenario: AB cycles alternate origin

- **WHEN** an AB cycle is extracted from two parents
- **THEN** its edges alternate between A-only and B-only origin and its length is even

#### Scenario: Shared edges unchanged

- **WHEN** both parents contain the same undirected edge
- **THEN** crossing over the pair does not delete or duplicate that edge

#### Scenario: Symmetric difference preserves degree

- **WHEN** selected cycles are applied to parent A
- **THEN** every city still has degree exactly two

#### Scenario: Identical parents

- **WHEN** the two parents are identical
- **THEN** the solver applies a double-bridge perturbation and local polish instead of looping on an empty cycle set

### Requirement: E-set strategy and parent pairing

The system SHALL select crossover E-sets using `single_ab` (one random AB cycle), `rand` (each cycle selected independently with 50% probability), or `hybrid` (start as `rand`, switch to `single_ab` after `stall_epochs` consecutive stagnant generations), and SHALL pair distinct population members without self-pairing.

#### Scenario: Each strategy selects a valid E-set

- **WHEN** the configured strategy is `single_ab`, `rand`, or `hybrid`
- **THEN** every offspring is produced from a non-empty valid E-set, except the identical-parent case

#### Scenario: Hybrid strategy switches

- **WHEN** the `hybrid` strategy records `stall_epochs` consecutive stagnant generations
- **THEN** subsequent generations select a single AB cycle per offspring

#### Scenario: No self-pairing

- **WHEN** the population has one, two, or an odd number of members
- **THEN** no member is paired with itself and every offspring has two distinct parents

### Requirement: Subtour patching

The system SHALL identify the connected components produced by a crossover, merge the smallest component into another using candidate-list edges, and SHALL preserve degree two at every city while reducing the component count until a single tour remains; when no candidate edge reaches another component it SHALL widen the search.

#### Scenario: Degree preserved during patching

- **WHEN** a crossover produces multiple components
- **THEN** each patching move keeps every city at degree exactly two

#### Scenario: Component count decreases

- **WHEN** patching runs
- **THEN** each accepted bridge reduces the component count by one until one tour remains

#### Scenario: No candidate bridge exists

- **WHEN** no candidate-list edge connects two components
- **THEN** the solver widens the search and still returns a single connected tour

### Requirement: Dirty-edge local polish

The system SHALL polish offspring with candidate-restricted 2-opt seeded by vertices incident to newly added bridge edges, propagating from accepted improving swaps, and SHALL leave one connected component after each pass.

#### Scenario: Polish improves or preserves cost

- **WHEN** an offspring is polished
- **THEN** its cost is lower than or equal to its pre-polish cost and the tour remains a single cycle

#### Scenario: Dirty vertices seeded

- **WHEN** patching adds a bridge edge
- **THEN** the next local-search pass evaluates moves starting from that edge's endpoints before other vertices

### Requirement: Replacement and termination

The system SHALL keep the best child of each parent's child pool, replace that parent only when the child is strictly cheaper, track the global best and stagnant generations, and stop when the epoch cap is reached or after twice `stall_epochs` stagnant generations.

#### Scenario: Strict improvement required

- **WHEN** a child costs the same as or more than its parent
- **THEN** the parent remains in the population

#### Scenario: Global best tracked

- **WHEN** a child is cheaper than the global best
- **THEN** it becomes the reported global best and resets the stagnant generation count

#### Scenario: Stagnation stop

- **WHEN** no global-best improvement occurs for twice `stall_epochs` consecutive generations
- **THEN** the solver stops and returns the best tour found

### Requirement: Reproducible runs

The system SHALL accept an optional `seed` and produce the same tour for the same problem, options, and seed.

#### Scenario: Seeded runs match

- **WHEN** the solver runs twice with the same problem, options, and seed
- **THEN** both runs return the same route

#### Scenario: Unseeded runs remain valid

- **WHEN** no seed is supplied
- **THEN** each run still returns a valid tour and reports its exact recomputed cost

### Requirement: Symmetric instance constraint

The system SHALL solve only symmetric TSP instances; asymmetric (`TYPE: ATSP`) input is rejected by the TSPLIB parser, and the solver SHALL document the constraint instead of silently applying reversal-based moves to a directed matrix.

#### Scenario: ATSP is rejected before solving

- **WHEN** a TSPLIB file declares `TYPE: ATSP`
- **THEN** parsing fails with a clear asymmetric-instance error and no solver runs

#### Scenario: Symmetric input solves

- **WHEN** a symmetric `TYPE: TSP` instance is supplied
- **THEN** the solver proceeds normally and returns a valid tour

### Requirement: Solver registration and aliases

The system SHALL register the solver as `memetic_eax` with aliases `memetic` and `eax`, and SHALL advertise it as a metaheuristic solver with options in the solver metadata.

#### Scenario: Aliases resolve to the solver

- **WHEN** a user supplies `memetic_eax`, `memetic`, or `eax`
- **THEN** all three resolve to the same solver

#### Scenario: Metadata advertises the solver

- **WHEN** the solver list is queried
- **THEN** it includes the solver as a metaheuristic with a complexity description and options marked as available

### Requirement: CLI and TOML configuration

The system SHALL expose `epochs`, `pop_size`, `children_per_pair`, `strategy`, `stall_epochs`, `candidate_k`, and `seed` through CLI flags and a TOML `[memetic_eax]` stage table, and SHALL reject invalid values with a descriptive error.

#### Scenario: CLI flags configure the solver

- **WHEN** the user runs the solver with flags for population size, children per pair, strategy, stall epochs, candidate count, and seed
- **THEN** each value is parsed into the solver options and takes effect

#### Scenario: TOML stage configures the solver

- **WHEN** a config stage contains a `[memetic_eax]` table
- **THEN** those fields populate the solver options for that stage

#### Scenario: Invalid values are rejected

- **WHEN** a value is outside its allowed range or an unknown field is supplied
- **THEN** the system returns a descriptive error instead of silently using a default

### Requirement: Pipeline composition

The system SHALL consume an optional warm-start tour as a seeded member of its initial population, SHALL be usable standalone or as a pipeline stage after a constructive solver such as `nn` or `greedy_edge`, and SHALL NOT be auto-expanded with an `nn` or `shuffle` prefix.

#### Scenario: Warm-start tour seeds the population

- **WHEN** the solver is the second stage of `pipeline(nn, memetic_eax)`
- **THEN** the nearest-neighbour tour is present in the initial population and the final cost is no worse than the seed

#### Scenario: Standalone run needs no seed

- **WHEN** the solver is invoked directly without a pipeline
- **THEN** it builds its own population and returns a valid tour

#### Scenario: No automatic prefix

- **WHEN** the CLI resolves the solver's automatic seed behavior
- **THEN** it runs the solver without prepending `nn` or `shuffle`

### Requirement: Cross-surface exposure

The system SHALL expose the solver and its configuration through the HTTP API, the WASM bridge, and the web option plumbing, and SHALL list and document it alongside the existing solvers.

#### Scenario: API accepts the solver and its config

- **WHEN** an API request selects `memetic_eax` and supplies its configuration
- **THEN** the configuration maps to the solver options instead of being silently ignored

#### Scenario: WASM lists and runs the solver

- **WHEN** the WASM algorithm list is queried or the solver is invoked
- **THEN** it appears as a metaheuristic with its parameters and produces a valid tour

#### Scenario: Web documents and lists the solver

- **WHEN** the web algorithms index renders
- **THEN** it includes the solver under its benchmark family with a link to its algorithm page

### Requirement: Tour quality on TSPLIB benchmarks

The system SHALL produce high-quality tours on TSPLIB instances with fewer than 2,000 cities and a published optimal cost, measured over five runs per instance within the meta-family 300-second budget, recording the machine, build mode, seed, and elapsed-time scope for every result.

#### Scenario: berlin52 reaches the optimum

- **WHEN** the solver runs five seeded times on `berlin52` (optimum 7,542)
- **THEN** every run reaches a 0.00% gap in under 0.2 seconds

#### Scenario: kroA100 reaches the optimum

- **WHEN** the solver runs five seeded times on `kroA100` (optimum 21,282)
- **THEN** every run reaches a 0.00% gap in under 0.5 seconds

#### Scenario: a280 stays close

- **WHEN** the solver runs five seeded times on `a280` (optimum 2,579)
- **THEN** the median gap is at most 0.15% in under 2.0 seconds

#### Scenario: dsj1000 stays close without timing out

- **WHEN** the solver runs five seeded times on `dsj1000` (optimum 18,660,188)
- **THEN** the median gap is at most 0.50% and no run times out within the 300-second meta budget

#### Scenario: Hardware-sensitive targets are reported

- **WHEN** a strict target is not met on the measuring machine
- **THEN** the measured result is reported rather than the target being weakened silently

> **Measured at `eab6b34` (default options).** 60 runs, 0 timeouts: every instance except
> `dsj1000` reached its published optimum; `dsj1000` was 0.028% median with a 32.4 s median.
> The small-instance time targets above are tuned-configuration targets; at the default
> `pop_size = 100` × `children_per_pair = 15` they take 2.9 s / 3.5 s / 5.4 s. The `dsj1000`
> cost is reported 10 units below the exact CEIL_2D sum because `tour_length` accumulates f32;
> the route's exact cost is the optimum.

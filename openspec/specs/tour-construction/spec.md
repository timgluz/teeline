# tour-construction Specification

## Purpose

Builds complete tours directly from city geometry, giving users fast constructive starting points that downstream local search can refine.

## Requirements

### Requirement: Valid tour construction

The system SHALL provide a space-filling-curve (SFC) constructive solver that returns a complete tour visiting every city exactly once, ordering cities by their position along a 1D space-filling curve over a normalized grid.

#### Scenario: Valid permutation on a benchmark instance

- **WHEN** the SFC solver runs on `berlin52`
- **THEN** the returned tour contains each city ID exactly once and its reported cost equals the recomputed tour length

#### Scenario: Degenerate inputs

- **WHEN** the problem has 0, 1, 2, identical, or collinear cities
- **THEN** the solver returns without panicking and produces a valid tour for the cities present

#### Scenario: Deterministic output

- **WHEN** the SFC solver runs twice on the same problem and options
- **THEN** both runs return the same route

### Requirement: Hilbert curve geometry

The system SHALL implement Hilbert curve indexing such that consecutive curve indices map to grid-adjacent cells, and the mapping is a bijection over the grid.

#### Scenario: Consecutive indices are grid neighbours

- **WHEN** the Hilbert index is computed for every cell of a `2^p × 2^p` grid with `p` from 1 through 6
- **THEN** consecutive indices differ by exactly Manhattan distance 1

#### Scenario: Index mapping is a bijection

- **WHEN** the Hilbert index is computed for every cell of a `2^p × 2^p` grid with `p` from 1 through 6
- **THEN** all `4^p` indices are distinct and cover the range `0..4^p`

#### Scenario: Morton indexing is available

- **WHEN** the curve type is `morton`
- **THEN** the solver orders cities by bit-interleaved Morton keys and produces a valid tour

### Requirement: Configurable grid normalization

The system SHALL normalize coordinates onto a `2^p × 2^p` grid with a configurable order `p` in the range `1..=16` and a configurable aspect mode of `uniform` or `stretch_to_square`.

#### Scenario: Uniform scaling preserves aspect ratio

- **WHEN** `aspect_scaling` is `uniform`
- **THEN** both axes are divided by the larger span so the normalized shape preserves its aspect ratio

#### Scenario: Stretch fills the grid on elongated maps

- **WHEN** `aspect_scaling` is `stretch_to_square`
- **THEN** each axis is normalized independently and the grid is used across both dimensions

#### Scenario: Maximum edge stays in bounds

- **WHEN** a city lies at the maximum coordinate on an axis
- **THEN** its grid coordinate is clamped to `2^p - 1` rather than overflowing the grid

#### Scenario: Order is bounded

- **WHEN** the configured order is `0` or greater than `16`
- **THEN** validation rejects the configuration with an error

### Requirement: Orientation transforms and ensemble

The system SHALL support rotation by 0/90/180/270 degrees, optional mirroring, and an optional multi-orientation mode that evaluates several orientations and keeps the shortest tour.

#### Scenario: Rotation and mirror change the curve entry and seam

- **WHEN** a non-zero rotation or mirroring is applied
- **THEN** the resulting tour reflects that symmetry transform and remains a valid permutation

#### Scenario: Multi-orientation never worse than single orientation

- **WHEN** multi-orientation is enabled
- **THEN** the returned tour cost is less than or equal to the cost of any single orientation it evaluated

#### Scenario: Multi-orientation is deterministic

- **WHEN** multi-orientation runs twice with the same problem and options
- **THEN** both runs return the same route

### Requirement: Bucket tie-breaking

The system SHALL resolve cities sharing a grid cell using a configurable tie-breaker: `input_order`, `lexicographic`, or `nearest_prev`.

#### Scenario: Input order preserves input sequence

- **WHEN** the tie-breaker is `input_order` and several cities share a cell
- **THEN** those cities appear in their original input order

#### Scenario: Lexicographic orders by coordinates

- **WHEN** the tie-breaker is `lexicographic` and several cities share a cell
- **THEN** those cities are ordered by their original coordinate pair

#### Scenario: Nearest previous picks the closest bucket city

- **WHEN** the tie-breaker is `nearest_prev` and a cell's bucket has at most 64 cities
- **THEN** each pick is the city closest to the last visited city, with a deterministic fallback on ties

#### Scenario: Oversized buckets preserve the bound

- **WHEN** a cell contains more than 64 cities and the tie-breaker is `nearest_prev`
- **THEN** that bucket keeps input order so the solver remains O(n log n)

### Requirement: Solver registration and aliases

The system SHALL register the solver under the canonical name `sfc` with aliases `hilbert` and `space_filling_curve`, and advertise it in the solver metadata as a constructive solver with options.

#### Scenario: Aliases resolve to the solver

- **WHEN** a user supplies `sfc`, `hilbert`, or `space_filling_curve`
- **THEN** all three resolve to the same solver

#### Scenario: Metadata advertises the solver

- **WHEN** the solver list is queried
- **THEN** it includes the SFC solver with a constructive category, an `O(n log n)` complexity, and options marked as available

### Requirement: CLI and TOML configuration

The system SHALL expose every SFC parameter through CLI flags and a TOML `[sfc]` table, and SHALL reject invalid values.

#### Scenario: CLI flags configure the solver

- **WHEN** the user runs the solver with CLI flags for curve type, order, aspect mode, rotation, mirroring, multi-orientation, and tie-breaker
- **THEN** each flag is parsed into the solver options and takes effect

#### Scenario: TOML table configures the solver

- **WHEN** a config stage contains a `[sfc]` table with SFC fields
- **THEN** those fields populate the solver options for that stage

#### Scenario: Invalid values are rejected

- **WHEN** a CLI or TOML value is outside its allowed range or an unknown field is supplied
- **THEN** the system returns a descriptive error instead of silently using a default

### Requirement: Pipeline composition

The system SHALL allow the SFC solver to be used as a pipeline stage that ignores a warm-start seed, and SHALL warn when it appears after the first stage.

#### Scenario: SFC seeds a later local search

- **WHEN** a pipeline runs SFC followed by 2-opt or Lin-Kernighan
- **THEN** the later stage starts from the SFC tour and returns a tour no worse than the SFC tour

#### Scenario: Non-first SFC discards the seed and warns

- **WHEN** the SFC solver appears at any stage after the first
- **THEN** the pipeline emits a non-fatal warning that the warm-start seed is discarded

#### Scenario: SFC as first stage needs no seed

- **WHEN** the SFC solver is the first pipeline stage
- **THEN** it runs without a seed and produces a valid tour

### Requirement: Cross-surface exposure

The system SHALL expose the solver through the HTTP API, the WASM bridge, and the web option plumbing.

#### Scenario: API accepts the solver and its config

- **WHEN** an API request selects `sfc` and supplies SFC configuration
- **THEN** the request is accepted and the configuration maps to the solver options

#### Scenario: WASM lists and runs the solver

- **WHEN** the WASM algorithm list is queried or the solver is invoked
- **THEN** `sfc` appears as a constructive solver with its parameters and produces a valid tour

#### Scenario: Web options include SFC parameters

- **WHEN** the web UI loads solver options for `sfc`
- **THEN** the SFC parameters are available to the user

### Requirement: Tour quality on TSPLIB benchmarks

The system SHALL produce a valid tour within a sane quality bound on TSPLIB instances that have fewer than 2,000 cities and a published optimal cost. Verification is limited to those instances until larger-instance support is addressed separately.

#### Scenario: Quality on berlin52

- **WHEN** the SFC solver runs on `berlin52` (52 cities, published optimum 7542)
- **THEN** the tour is a valid permutation and its cost is below 1.5 times the published optimum

#### Scenario: Quality on a larger TSPLIB instance

- **WHEN** the SFC solver runs on `att532` (532 cities, published optimum 27686) with multi-orientation enabled
- **THEN** the tour is a valid permutation and its cost is below 1.5 times the published optimum

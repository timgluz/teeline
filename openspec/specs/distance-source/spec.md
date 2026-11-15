# distance-source Specification

## Purpose

Defines how the system answers inter-city distance queries so the same solver logic runs against either a materialized distance table or distances computed from coordinates, enabling large instances without changing results.

## Requirements

### Requirement: Equivalent distance backends

The system SHALL support a materialized distance backend and a coordinate-computed backend for every coordinate-based TSPLIB distance type, and both SHALL return exactly the same distance for every city pair.

#### Scenario: Pairwise distances agree

- **WHEN** the same coordinate-only instance is queried through both backends for every ordered city pair
- **THEN** every returned distance is exactly equal

#### Scenario: Distances agree for every distance type

- **WHEN** EUC_2D, GEO, ATT, and CEIL_2D instances (the last from a synthetic fixture, since no CEIL_2D file is tracked) are queried through both backends
- **THEN** every returned distance is exactly equal for every type

#### Scenario: Deterministic solver results agree

- **WHEN** a deterministic solver (`nn`, `sfc`, `2opt`, `or_opt`, `bhk`) runs with the same options against both backends on a tracked fixture
- **THEN** the returned tour and reported cost are exactly equal

#### Scenario: Seeded Lin-Kernighan uses the declared metric

- **WHEN** `lk` runs with a fixed seed on a GEO fixture against either backend
- **THEN** it returns the same tour and cost, and the cost equals the GEO recomputation of that tour

### Requirement: Positions are independent of city IDs

The system SHALL treat distance-source positions as dense indices into the problem's city list, independent of the external city IDs, and SHALL preserve the original city IDs in every reported tour.

#### Scenario: Non-contiguous city IDs

- **WHEN** a problem's city IDs are non-contiguous (for example 10, 20, 30)
- **THEN** every distance lookup by position is correct, both backends agree, and the reported tour contains the original city IDs rather than positions

#### Scenario: One-based city IDs

- **WHEN** a 1-based TSPLIB instance is solved
- **THEN** the reported tour uses the file's 1-based city IDs and no position is mistaken for an ID

### Requirement: Explicit weight input uses the materialized backend

For input carrying EXPLICIT edge weights, the system SHALL always use the materialized backend regardless of the declared distance type, because EXPLICIT distances are already materialized and any coordinates supplied alongside them are a display or derived layout rather than the distance source.

#### Scenario: Automatic selection is materialized

- **WHEN** an EXPLICIT instance is solved with automatic backend selection
- **THEN** the materialized backend is used

#### Scenario: Distance-type override does not bypass explicit weights

- **WHEN** an EXPLICIT instance is run with a coordinate distance-type override such as EUC_2D
- **THEN** the materialized backend is still used and the explicit weights are scored

#### Scenario: Coordinate-computed backend is rejected

- **WHEN** an EXPLICIT instance is solved with the coordinate-computed backend selected
- **THEN** the system returns an error naming the incompatible selection

#### Scenario: Coordinate input cannot claim EXPLICIT

- **WHEN** coordinate-only input is run with an EXPLICIT distance-type override
- **THEN** the system returns a descriptive error instead of panicking

### Requirement: Automatic backend selection by instance size

For coordinate-based input, the system SHALL select the coordinate-computed backend above a documented city-count threshold and the materialized backend at or below it, and SHALL accept an explicit user override.

#### Scenario: At or below the threshold

- **WHEN** automatic selection runs at or below the threshold
- **THEN** the materialized backend is used

#### Scenario: Above the threshold

- **WHEN** automatic selection runs above the threshold
- **THEN** the coordinate-computed backend is used

#### Scenario: Explicit overrides win on coordinate input

- **WHEN** the user explicitly selects a backend for coordinate-based input
- **THEN** that backend is used regardless of the instance size

#### Scenario: Unknown backend is rejected

- **WHEN** the user supplies an unknown backend name
- **THEN** the system returns a descriptive error

### Requirement: Large-instance execution ceiling

The system SHALL solve a coordinate-only EUC_2D instance of 100,000 cities through the normal solver dispatch with `sfc` under automatic selection, keeping peak resident memory below 1 GiB in a fresh process.

#### Scenario: 100k cities complete

- **WHEN** a generated 100k-city EUC_2D instance is solved with automatic selection
- **THEN** the solver returns a valid tour visiting every city exactly once

#### Scenario: Memory stays under the ceiling

- **WHEN** the 100k-city run's peak resident memory is measured in a fresh process
- **THEN** it is below 1 GiB

### Requirement: Reproducible memory benchmark

The system SHALL include a reproducible benchmark that reports peak resident memory and construction time for both the materialized and coordinate-computed backends at 1k, 5k, 10k, 25k, 50k, and 100k cities for each coordinate distance type, plus the EXPLICIT path.

#### Scenario: Benchmark reports both backends

- **WHEN** the benchmark runs at the documented sizes
- **THEN** it reports peak resident memory and construction time for every size, distance type, and backend

#### Scenario: Infeasible matrix sizes are recorded honestly

- **WHEN** a machine cannot hold the materialized matrix for a documented size
- **THEN** the benchmark records that size as not measured or out of memory rather than fabricating a number

#### Scenario: Default tests stay cheap

- **WHEN** the default test suite runs
- **THEN** the large benchmark sizes are not executed

### Requirement: Backend performance comparison

The system SHALL compare the materialized and coordinate-computed backends on identical solver and instance cells over multiple runs per cell, and SHALL keep the coordinate-computed backend's median runtime within a documented per-family tolerance while reducing peak memory.

#### Scenario: Same cells under both backends

- **WHEN** the comparison sweep runs
- **THEN** each solver and instance cell is measured under both backends on the same commit, build, and budget

#### Scenario: Runtime stays within tolerance

- **WHEN** the comparison rows are `ok` status and run a fixed amount of work
- **THEN** the coordinate-computed backend's median wall-clock time is within the documented tolerance for that solver family

#### Scenario: An outlier does not select a change

- **WHEN** a candidate improvement wins on a single run but not on the median of the repeated runs
- **THEN** it is not accepted as an improvement

#### Scenario: Memory improvement is recorded

- **WHEN** the comparison report is published
- **THEN** it states the peak-memory ratio between the backends for every compared size

#### Scenario: Reproduction command is documented

- **WHEN** a reader follows the documented comparison command
- **THEN** the sweep reproduces the report's cells and report location

### Requirement: Neighbourhood-limited local search

The system SHALL bound local-search move evaluation to a k-nearest neighbourhood whose memory grows with the number of cities times the candidate count, and tours produced this way SHALL stay within a documented gap of the published TSPLIB optimum on named instances.

#### Scenario: Quality is measured against the published optimum

- **WHEN** a local-search solver runs with neighbourhood-limited move evaluation on `a280` or `att532`
- **THEN** its tour cost is within the documented gap of the published optimal cost, using seeded or median runs

#### Scenario: Memory is linear in candidates

- **WHEN** neighbourhood-limited move evaluation is used
- **THEN** its memory grows with the number of cities times the candidate count

#### Scenario: EXPLICIT input skips coordinate-based pruning

- **WHEN** the problem comes from EXPLICIT weights and therefore has no usable coordinates
- **THEN** no candidate list is built from placeholder coordinates, and a solver that can fall back to a full scan does so

#### Scenario: EXPLICIT local search uses exact matrix candidates

- **WHEN** a local-search solver requires a candidate list on an EXPLICIT instance, with or without display data
- **THEN** the list is built from the exact distance matrix instead of the coordinates

### Requirement: Compact EXPLICIT representation

The system SHALL represent EXPLICIT distances at no more than the integer width the instance's weight range requires when every weight is a non-negative integer, and SHALL preserve every distance exactly; non-integer or negative weights SHALL fall back to `f32`.

#### Scenario: Narrow weights use a narrow width

- **WHEN** an EXPLICIT instance's weights fit in 16 bits
- **THEN** its distance storage uses at most two bytes per pair

#### Scenario: Wide weights use a 32-bit width

- **WHEN** an EXPLICIT instance's weights require 32 bits
- **THEN** its distance storage uses at most four bytes per pair

#### Scenario: Compaction preserves costs

- **WHEN** an EXPLICIT instance is solved after compaction
- **THEN** every reported tour cost equals the value produced before compaction

#### Scenario: Non-integer weights fall back safely

- **WHEN** an EXPLICIT instance contains non-integer or negative weights
- **THEN** it keeps `f32` storage and every distance is preserved exactly

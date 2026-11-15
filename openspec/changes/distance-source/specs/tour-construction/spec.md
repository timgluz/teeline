# Spec Delta

## ADDED Requirements

### Requirement: Large-instance SFC execution

The system SHALL construct and score a 100,000-city EUC_2D tour through the normal solver dispatch with `sfc` and automatic backend selection, keeping peak resident memory within the ceiling defined by the distance-source capability.

#### Scenario: 100k cities complete

- **WHEN** the SFC solver runs on a coordinate-only instance with 100,000 cities
- **THEN** it returns a valid permutation of every city and its reported cost equals the recomputed tour length

#### Scenario: Memory stays under the ceiling

- **WHEN** the 100k-city run's peak resident memory is measured in a fresh process
- **THEN** it is within the ceiling defined by the distance-source capability

#### Scenario: Multi-orientation scoring uses the problem's distance source

- **WHEN** multi-orientation is enabled on any SFC instance
- **THEN** orientation costs are scored through the problem's configured distance source rather than a private matrix

## MODIFIED Requirements

### Requirement: Tour quality on TSPLIB benchmarks

The system SHALL produce a valid tour within a sane quality bound on TSPLIB instances that have fewer than 2,000 cities and a published optimal cost. Verification against a published optimum is limited to those instances; larger instances are covered by execution and memory requirements rather than optimality.

#### Scenario: Quality on berlin52

- **WHEN** the SFC solver runs on `berlin52` (52 cities, published optimum 7542)
- **THEN** the tour is a valid permutation and its cost is below 1.5 times the published optimum

#### Scenario: Quality on a larger TSPLIB instance

- **WHEN** the SFC solver runs on `att532` (532 cities, published optimum 27686) with multi-orientation enabled
- **THEN** the tour is a valid permutation and its cost is below 1.5 times the published optimum

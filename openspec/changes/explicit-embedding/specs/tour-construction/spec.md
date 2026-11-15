# Spec Delta

## ADDED Requirements

### Requirement: SFC on coordinate-less EXPLICIT instances

The system SHALL run the SFC solver on EXPLICIT instances that lack real coordinates by ordering cities over the derived layout, and SHALL NOT return an early error or order cities by placeholder grid positions.

#### Scenario: Valid improved tour on gr17

- **WHEN** `sfc` runs on `gr17` (17 cities, EXPLICIT, no coordinates, published optimum 2085)
- **THEN** it returns a valid permutation whose cost is lower than the tour produced from the placeholder grid coordinates

#### Scenario: Deterministic on derived layout

- **WHEN** `sfc` runs twice on a coordinate-less EXPLICIT instance with the same options
- **THEN** both runs return the same route

#### Scenario: Options still apply

- **WHEN** curve type, order, aspect scaling, rotation, mirroring, multi-orientation, or tie-breaker options are set on a coordinate-less EXPLICIT instance
- **THEN** the options are honoured over the derived layout and the tour remains a valid permutation

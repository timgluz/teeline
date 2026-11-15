# Spec Delta

## MODIFIED Requirements

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

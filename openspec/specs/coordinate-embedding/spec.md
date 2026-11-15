# coordinate-embedding Specification

## Purpose

Provides a deterministic 2D layout derived from a distance matrix so geometry-based solvers can operate on instances that carry distances but no coordinates.

## Requirements

### Requirement: Coordinate presence is tracked

The system SHALL record whether an instance supplied real city coordinates — through a node-coordinate section or display data — separately from whether its distances are EXPLICIT weights.

#### Scenario: Coordinate-less EXPLICIT instance

- **WHEN** an EXPLICIT instance supplies neither a node-coordinate section nor display data
- **THEN** the instance is marked as lacking real coordinates

#### Scenario: EXPLICIT instance with display data

- **WHEN** an EXPLICIT instance supplies a display-data section
- **THEN** the instance is marked as having real coordinates and no layout is derived

#### Scenario: Coordinate instance

- **WHEN** an instance supplies a node-coordinate section
- **THEN** the instance is marked as having real coordinates

#### Scenario: Parsed city list exposes the layout

- **WHEN** an EXPLICIT instance without coordinates is parsed
- **THEN** the parsed city list is the derived layout rather than a placeholder grid

### Requirement: Deterministic derived layout

When real coordinates are absent, the system SHALL derive a 2D layout from the distance matrix with classical multidimensional scaling and a fixed deterministic procedure.

#### Scenario: Repeated derivation is identical

- **WHEN** the layout is derived twice from the same distance matrix
- **THEN** both runs produce identical coordinates

#### Scenario: Planar Euclidean instance is reconstructed

- **WHEN** the distance matrix comes from a planar Euclidean point set
- **THEN** the derived layout reproduces every pairwise distance up to translation, rotation, and reflection

#### Scenario: Isotropic instance keeps both axes

- **WHEN** the distance matrix comes from a square or a regular polygon, where the top two eigenvalues are equal
- **THEN** the layout uses two dimensions and reconstructs the pairwise distances

#### Scenario: Collinear instance does not fabricate a second axis

- **WHEN** the distance matrix comes from collinear points, where the second eigenvalue is not positive
- **THEN** the second coordinate is zero instead of an arbitrary direction

#### Scenario: Elongated instance still converges

- **WHEN** the distance matrix comes from a planar point set with a high aspect ratio
- **THEN** the layout reconstructs the pairwise distances within the documented tolerance

#### Scenario: Non-Euclidean instance still yields a layout

- **WHEN** the distance matrix is not realizable as Euclidean distances (for example street distances)
- **THEN** a finite layout is produced and every city receives coordinates

#### Scenario: Degenerate instance

- **WHEN** the instance has fewer than two cities or every distance is equal
- **THEN** the layout is finite and deterministic

### Requirement: Geometry-based constructive solvers use the derived layout

The system SHALL make the derived layout available as the problem's city geometry so geometry-based constructive solvers return valid tours on coordinate-less EXPLICIT instances.

#### Scenario: Fourier on a coordinate-less instance

- **WHEN** `fourier` runs on `gr17` (EXPLICIT, no coordinates)
- **THEN** it returns a valid permutation and does not return an early error

#### Scenario: SOM on a coordinate-less instance

- **WHEN** `som` runs on `gr17`
- **THEN** it returns a valid permutation and does not return an early error

#### Scenario: Deterministic geometry solver

- **WHEN** `sfc` or `fourier` runs twice on a coordinate-less EXPLICIT instance with the same options
- **THEN** both runs return the same route

### Requirement: No solver is rejected solely for missing coordinates

The system SHALL NOT reject a registered solver because an EXPLICIT instance lacks coordinates; every solver SHALL return a valid tour for such an instance, except through an explicit opt-out.

#### Scenario: Every solver returns a valid tour

- **WHEN** each registered solver runs on `gr17` with a budget small enough to complete
- **THEN** each returns a permutation of every city ID and a finite cost

#### Scenario: Explicit opt-out is honoured

- **WHEN** a caller requests `RequireReal` while building a problem from an instance without real coordinates
- **THEN** problem construction returns a descriptive error instead of a problem with placeholder or derived coordinates

### Requirement: Derived-layout provenance is reported

The system SHALL record that a run used a derived layout, so benchmarks and explainers can distinguish derived-layout results from real-coordinate results.

#### Scenario: Benchmark row is labelled

- **WHEN** a benchmark cell runs on an instance whose coordinates were derived
- **THEN** the published row records the derived-layout provenance

#### Scenario: Algorithm benchmark table labels derived rows

- **WHEN** an algorithm page renders a benchmark result for a coordinate-less EXPLICIT instance
- **THEN** the result records the derived-layout provenance

#### Scenario: Problem page labels the minimap

- **WHEN** the problem page renders the minimap for a coordinate-less EXPLICIT instance
- **THEN** it labels the layout as approximate rather than presenting it as surveyed geography

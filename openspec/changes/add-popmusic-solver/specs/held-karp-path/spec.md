# Spec Delta

## Purpose

Provides one exact, deterministic fixed-endpoint shortest-path dynamic program that solvers
reuse instead of duplicating the Held–Karp recurrence.

## ADDED Requirements

### Requirement: Exact fixed-endpoint path

The system SHALL compute the cheapest open path from a start node through exactly a given set of interior nodes, in any order, ending at a given end node, and the returned path cost SHALL equal the minimum found by exhaustive enumeration for the same nodes.

#### Scenario: Matches brute force

- **WHEN** the problem has at most eight interior nodes
- **THEN** the returned cost equals the minimum over every interior permutation, and the returned order realizes that cost

#### Scenario: Fixed endpoints

- **WHEN** a path is returned
- **THEN** its first node is the start, its last node is the end, and neither appears among the interior nodes

#### Scenario: Empty interior

- **WHEN** the interior set is empty
- **THEN** the result is the direct edge from start to end

#### Scenario: Stable tie-breaking

- **WHEN** several interior orders tie at the minimum cost
- **THEN** the same order is returned on every run, chosen by first scan order

### Requirement: Bounded state space

The system SHALL support interior sets up to a documented bound and SHALL reject larger inputs rather than overflow the dynamic-programming table.

#### Scenario: Bound enforced

- **WHEN** the interior set exceeds the documented bound
- **THEN** the call fails instead of attempting a state table that cannot be represented

#### Scenario: Within bound

- **WHEN** the interior set is within the bound
- **THEN** the call completes and its memory use is proportional to the interior size times two to that size

### Requirement: Bellman–Held–Karp exactness preserved

The system SHALL keep the Bellman–Held–Karp solver exact after moving it onto the shared path primitive: it SHALL return a complete tour visiting every city exactly once whose cost equals the optimum recomputed from the problem's distance source.

#### Scenario: Optimal cost on known instances

- **WHEN** the solver runs on `burma14`, `gr17`, or `ulysses16`
- **THEN** the reported cost equals the published optimum and the route is a valid permutation

#### Scenario: Deterministic output

- **WHEN** the solver runs twice on the same problem
- **THEN** both runs return the same route and cost

#### Scenario: Small instances

- **WHEN** the instance has one or two cities
- **THEN** the solver returns a valid tour without panicking

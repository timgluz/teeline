# Spec Delta

## Purpose

Makes Teeline's solvers understandable in the browser by turning their geometry and state transitions into interactive, accessible visual explanations.

## ADDED Requirements

### Requirement: Space-Filling Curve explainer is discoverable

The system SHALL provide an interactive Space-Filling Curve explainer at the standard algorithm explainer route and SHALL link it from the solver's documentation page and the algorithms index.

#### Scenario: Explainer route renders

- **WHEN** a visitor opens `/algorithms/space_filling_curve/explainer/`
- **THEN** the page renders the Space-Filling Curve explainer with its title, controls, and visualization

#### Scenario: Docs page links to the explainer

- **WHEN** a visitor opens the Space-Filling Curve documentation page
- **THEN** the page shows the standard "Open interactive explainer" link to the explainer

#### Scenario: Algorithms index lists the explainer

- **WHEN** a visitor opens `/algorithms/`
- **THEN** the Space-Filling Curve row includes an interactive explainer link

### Requirement: Curve and city order are visible

The explainer SHALL display the selected space-filling curve over a `2^p × 2^p` grid, the order in which the curve visits the cities, and the resulting closed tour.

#### Scenario: Curve, order, and tour are shown together

- **WHEN** the explainer is displayed for any supported grid order
- **THEN** the curve, city order markers, and closed tour are all visible at the same time

#### Scenario: City positions follow the solver's normalization

- **WHEN** the explainer places cities on the grid
- **THEN** their positions use the same uniform bounding-box scaling and axis orientation as the solver's grid projection

### Requirement: Grid order is adjustable from 1 to 8

The explainer SHALL let the visitor change the grid order `p` from 1 through 8 and SHALL update the curve, city order, tour, and statistics for the selected value.

#### Scenario: Slider updates the visualization

- **WHEN** the visitor moves the grid-order slider from one value to another
- **THEN** the curve, city order, tour, and statistics reflect the newly selected order

#### Scenario: Subdivision can be played

- **WHEN** the visitor starts the subdivision animation
- **THEN** the explainer steps through grid orders from 1 toward 8 and stops at 8

#### Scenario: Reduced motion is respected

- **WHEN** the visitor's environment requests reduced motion
- **THEN** starting the subdivision animation does not animate through intermediate orders

### Requirement: Hilbert and Morton curves can be compared

The explainer SHALL let the visitor switch between Hilbert and Morton curve types and SHALL show how the choice changes the city ordering and the resulting tour.

#### Scenario: Switching curve type redraws the explainer

- **WHEN** the visitor switches the curve type
- **THEN** the curve, city order, tour, and statistics update for the selected curve type

#### Scenario: Long curve jumps are distinguishable

- **WHEN** the selected curve contains consecutive cells that are not grid neighbours
- **THEN** those jumps are counted in the long-curve-jump statistic and drawn in a distinct style from locally adjacent curve segments

### Requirement: Seam and longest edge are explained

The explainer SHALL identify the tour's closing seam edge, SHALL identify the longest tour edge when it differs from the seam, and SHALL explain why an SFC tour is commonly improved with a 2-opt pass.

#### Scenario: Closing edge is highlighted

- **WHEN** a tour is displayed
- **THEN** the edge from the last city back to the first is highlighted and labelled as the seam

#### Scenario: Longest edge is surfaced separately

- **WHEN** the longest tour edge is not the closing seam edge
- **THEN** the longest edge is highlighted separately from the seam

#### Scenario: 2-opt explanation is present

- **WHEN** the explainer displays the seam
- **THEN** it includes a concise explanation of how a 2-opt pass can remove the seam and shorten the tour

### Requirement: Teaching statistics expose the mechanism

The explainer SHALL display statistics that make the grid projection and curve locality legible: cities and cells used, the largest same-cell bucket, tour length, seam length, longest edge length, and the number of long curve jumps. A long curve jump is a step between consecutive curve cells that are not grid neighbours.

#### Scenario: Grid resolution effect is visible

- **WHEN** the visitor changes the grid order
- **THEN** the cells-used and largest-bucket statistics update to show how cities share or separate into cells

#### Scenario: Curve locality effect is visible

- **WHEN** the visitor switches between Hilbert and Morton
- **THEN** the long-curve-jump statistic reflects the locality difference between the two curves

### Requirement: Explainer is accessible and responsive

The explainer SHALL be operable by keyboard, expose a text alternative for the visualization, and remain usable on supported viewport sizes.

#### Scenario: Keyboard operation

- **WHEN** a visitor uses only the keyboard
- **THEN** every control can be reached and operated, and the grid-order slider reports its value and grid size

#### Scenario: Screen-reader summary

- **WHEN** a screen reader encounters the visualization
- **THEN** it receives a summary of the selected curve, grid order, city order, tour length, seam, and longest edge

#### Scenario: Narrow viewport

- **WHEN** the explainer is viewed on a narrow viewport
- **THEN** the visualization, controls, statistics, and order list remain visible and usable without horizontal scrolling

### Requirement: Explainer simulation is deterministic

The explainer SHALL produce the same curve, city order, tour, and statistics for the same city layout, grid order, curve type, and tie-breaking rule.

#### Scenario: Repeated renders agree

- **WHEN** the visitor selects the same grid order and curve type more than once
- **THEN** the explainer shows the same city order and tour each time

#### Scenario: Same-cell order follows input order

- **WHEN** several cities map to the same grid cell
- **THEN** their relative order follows the cities' input order

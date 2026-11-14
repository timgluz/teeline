# web-explainers Specification

## Purpose

Makes Teeline's solvers understandable in the browser by turning their construction steps, geometry, and state transitions into interactive, accessible visual explanations.

## Requirements

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

### Requirement: Construction is staged

The explainer SHALL present the construction in four ordered stages: Project (cities to cells), Index (cells to curve index), Sort (index to visit order), and Connect (visit order to tour). The initial state SHALL be Project with no completed tour.

#### Scenario: Initial state is not a finished tour

- **WHEN** the explainer first loads
- **THEN** it shows the Project stage with cities and grid cells and no closed tour

#### Scenario: Stepping forward reveals the construction

- **WHEN** the visitor steps forward from Project
- **THEN** the explainer advances through Index, Sort, and Connect in that order

#### Scenario: Connect adds edges in visit order

- **WHEN** the construction reaches Connect
- **THEN** tour edges appear in visit order and the closing seam is added last

#### Scenario: Stepping back reverses the construction

- **WHEN** the visitor steps back from a later stage or edge
- **THEN** the explainer hides the last revealed edge or returns to the previous stage

### Requirement: Construction playback is controllable

The explainer SHALL provide Back, Step, Run/Pause, and Reset controls for the construction, and SHALL provide Pause and discrete minus/plus stepping for the grid-order subdivision. The grid-order slider and curve toggle SHALL remain usable while playback is paused.

#### Scenario: Run and Pause freeze a transition

- **WHEN** the visitor pauses during construction or subdivision playback
- **THEN** the current stage, edge, or grid order remains visible until the visitor resumes

#### Scenario: Reset restores the initial state

- **WHEN** the visitor presses Reset
- **THEN** the explainer returns to Project at the initial grid order and curve type

#### Scenario: Subdivision can be stepped

- **WHEN** the visitor uses the grid-order minus or plus control
- **THEN** the grid order changes by one and subdivision playback pauses

#### Scenario: Reduced motion skips animation

- **WHEN** the visitor's environment requests reduced motion
- **THEN** Run jumps to the completed Connect stage and subdivision playback jumps to the final grid order

### Requirement: City-to-cell-to-index mapping is visible

The explainer SHALL show each city's projected cell, curve index, and visit order, and SHALL state that the tour connects cities in index order rather than following the curve path.

#### Scenario: Project shows the cell assignment

- **WHEN** the explainer is in the Project stage
- **THEN** occupied grid cells are visible and shared cells show how many cities they contain

#### Scenario: Index shows the curve index

- **WHEN** the explainer enters the Index stage
- **THEN** each occupied cell shows its curve index and the curve that defines it

#### Scenario: Sort shows the visit order

- **WHEN** the explainer enters the Sort stage
- **THEN** the sort table and 1D index strip order the cities by curve index

#### Scenario: The curve is distinguished from the tour

- **WHEN** the explainer explains the construction
- **THEN** it states that the curve supplies a 1D sort key and the tour connects the sorted cities, rather than following the curve's line

### Requirement: 1D index strip and sort table

The explainer SHALL display a 1D index strip below the canvas and a compact sort table with visit, city, cell, and index columns. Cities at the same index SHALL stack on the strip and SHALL be grouped in the table.

#### Scenario: Strip ticks follow the curve index

- **WHEN** the index strip is displayed
- **THEN** each city appears as a tick at its curve index, and same-index cities stack

#### Scenario: Table exposes the sort key

- **WHEN** the visitor reads the sort table
- **THEN** the visit, city, cell, and index columns identify the sort key for every city

#### Scenario: Hover is synchronized

- **WHEN** the visitor hovers or focuses a city, tick, or table row
- **THEN** the same city is highlighted in the canvas, strip, and table

#### Scenario: Playhead follows the tour

- **WHEN** the Connect stage advances an edge
- **THEN** the strip playhead highlights the current city's index and the current edge endpoints

### Requirement: Grid order is adjustable from 1 to 8

The explainer SHALL start at `p = 3` and SHALL let the visitor change the grid order from 1 through 8, updating the projection, curve, strip, table, tour, and statistics.

#### Scenario: Initial grid order is legible

- **WHEN** the explainer first loads
- **THEN** the grid order is 3, the curve is traceable, and at least one cell is shared by multiple cities

#### Scenario: Slider updates every view

- **WHEN** the visitor moves the grid-order slider
- **THEN** the projection, curve, strip, table, tour, and statistics reflect the selected order

#### Scenario: Play subdivision steps the order

- **WHEN** the visitor starts subdivision playback
- **THEN** the explainer steps the grid order toward 8 and stops at 8

### Requirement: Order changes are explained

When the grid order or curve type changes, the explainer SHALL identify the cities whose visit order changed and SHALL explain that finer grids split cells and same-cell cities use input order.

#### Scenario: Grid change highlights moved cities

- **WHEN** the visitor changes the grid order
- **THEN** the cities whose visit position changed are highlighted and the number of changed cities is reported

#### Scenario: Shared-cell ties use input order

- **WHEN** several cities share a grid cell
- **THEN** the table groups them and labels their relative order as input order

#### Scenario: Curve change updates the diff

- **WHEN** the visitor switches between Hilbert and Morton
- **THEN** the visit order, strip, table, and changed-city explanation update for the selected curve

### Requirement: Hilbert and Morton curves can be compared

The explainer SHALL let the visitor switch between Hilbert and Morton curve types and SHALL show how the choice changes the city ordering and the resulting tour.

#### Scenario: Switching curve type redraws the explainer

- **WHEN** the visitor switches the curve type
- **THEN** the curve, city order, tour, and statistics update for the selected curve type

#### Scenario: Morton quadrant crossings are called out

- **WHEN** the Morton curve is selected
- **THEN** the quadrant-crossing jumps are drawn in a distinct dashed style and explained

#### Scenario: Long curve jumps are counted

- **WHEN** the selected curve contains consecutive cells that are not grid neighbours
- **THEN** those jumps are counted in the long-curve-jump statistic

### Requirement: Seam, longest edge, and 2-opt preview

The explainer SHALL identify the tour's closing seam edge, SHALL identify the longest tour edge when it differs from the seam, and SHALL explain why an SFC tour is commonly improved with a 2-opt pass. It SHALL provide a preview of the best seam-removing 2-opt move.

#### Scenario: Closing edge is highlighted

- **WHEN** a tour is displayed
- **THEN** the edge from the last city back to the first is highlighted and labelled as the seam

#### Scenario: Longest edge is surfaced separately

- **WHEN** the longest tour edge is not the closing seam edge
- **THEN** the longest edge is highlighted separately from the seam

#### Scenario: Overlapping classifications remain visible

- **WHEN** the seam is also the longest edge
- **THEN** the stroke or halo still communicates both classifications

#### Scenario: 2-opt preview shows the actual swap

- **WHEN** the visitor enables the 2-opt preview
- **THEN** the seam and its best partner edge are ghosted, the two replacement edges are drawn, and the actual tour-length delta for the fixture is reported

#### Scenario: No fabricated improvement

- **WHEN** the 2-opt preview is shown
- **THEN** every length and delta is computed from the fixture coordinates

### Requirement: Teaching statistics expose the mechanism

The explainer SHALL display statistics that make the grid projection and curve locality legible: cities and cells used, the largest same-cell bucket, tour length, seam length, longest edge length, long curve jumps, and the number of cities whose order changed since the previous setting. A long curve jump is a step between consecutive curve cells that are not grid neighbours.

#### Scenario: Grid resolution effect is visible

- **WHEN** the visitor changes the grid order
- **THEN** the cells-used, largest-bucket, and changed-city statistics update

#### Scenario: Curve locality effect is visible

- **WHEN** the visitor switches between Hilbert and Morton
- **THEN** the long-curve-jump statistic reflects the locality difference between the two curves

#### Scenario: Tour statistics appear with the tour

- **WHEN** the construction reaches Connect
- **THEN** the tour, seam, and longest-edge statistics are displayed

### Requirement: Explainer is accessible and responsive

The explainer SHALL be operable by keyboard, expose a text alternative for the visualization, and remain usable on supported viewport sizes.

#### Scenario: Keyboard operation

- **WHEN** a visitor uses only the keyboard
- **THEN** every stage, playback, slider, toggle, and preview control can be reached and operated

#### Scenario: Table semantics

- **WHEN** a screen reader reads the sort table
- **THEN** it receives a caption, column headers, and grouped shared-cell rows

#### Scenario: Screen-reader summary

- **WHEN** a screen reader encounters the visualization
- **THEN** it receives the current stage, curve, grid order, bucket summary, city order, and tour statistics

#### Scenario: Narrow viewport

- **WHEN** the explainer is viewed on a narrow viewport
- **THEN** the canvas, strip, table, controls, and statistics remain usable without horizontal scrolling

### Requirement: Explainer simulation is deterministic

The explainer SHALL produce the same projection, curve, city order, tour, sort table, strip, and changed-city result for the same city layout, stage, grid order, curve type, and previous setting.

#### Scenario: Repeated renders agree

- **WHEN** the visitor selects the same stage, grid order, and curve type more than once
- **THEN** the explainer shows the same mapping, order, and tour each time

#### Scenario: Same-cell order follows input order

- **WHEN** several cities map to the same grid cell
- **THEN** their relative order follows the cities' input order

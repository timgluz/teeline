# Spec Delta

## ADDED Requirements

### Requirement: EAX explainer is discoverable

The system SHALL provide an interactive EAX explainer at `/algorithms/eax/explainer/`, link it from the EAX documentation page, and list it on the algorithms index.

#### Scenario: Explainer route renders

- **WHEN** a visitor opens `/algorithms/eax/explainer/`
- **THEN** the page renders the EAX explainer with its title, six-stage storyboard, and controls

#### Scenario: Docs page links to the explainer

- **WHEN** a visitor opens the EAX documentation page
- **THEN** the page shows the standard "Open interactive explainer" link

#### Scenario: Algorithms index lists the explainer

- **WHEN** a visitor opens `/algorithms/`
- **THEN** the Memetic EAX row includes an interactive explainer link

### Requirement: Six-stage storyboard

The explainer SHALL present one crossover event in six ordered stages: Compare Parents, Union Graph, AB-Cycles, Select E-set, Apply E-set, and Patch Subtours. The initial state SHALL be Compare Parents with two complete parent tours and no union graph.

#### Scenario: Initial state shows both parents

- **WHEN** the explainer first loads
- **THEN** it shows Parent A and Parent B, their tour lengths, and their shared-edge count

#### Scenario: Shared edges stay distinguishable

- **WHEN** the two parents share an edge
- **THEN** both tours remain visible on that edge (offset or alternating stroke) before the union graph fades it

#### Scenario: Stepping forward follows the storyboard

- **WHEN** the visitor steps forward
- **THEN** the explainer advances through the six stages in order, revealing each stage's edges and annotations

#### Scenario: Stepping back reverses the storyboard

- **WHEN** the visitor steps back from a later stage or sub-step
- **THEN** the explainer hides the last revealed operation and returns to the previous state

#### Scenario: Stage buttons jump

- **WHEN** the visitor selects a stage button
- **THEN** the explainer jumps to that stage's first frame and pauses playback

#### Scenario: Selection changes rebuild later frames

- **WHEN** the visitor changes a cycle or the strategy while paused
- **THEN** the frames after the selection are rebuilt from the new E-set

### Requirement: Union graph and AB-cycle decomposition

The explainer SHALL render the union multigraph of both parents, fade shared edges, highlight A-only and B-only edges separately, and decompose the symmetric difference into non-trivial alternating AB-cycles with even length.

#### Scenario: Shared and unique edges are distinguished

- **WHEN** the visitor reaches the Union Graph stage
- **THEN** shared edges are muted, A-only edges and B-only edges are drawn in distinct colours, and every active city shows degree two or four

#### Scenario: Cycles alternate origin

- **WHEN** the visitor reaches the AB-Cycles stage
- **THEN** every discovered cycle alternates A-only and B-only edges, has even length, and carries a cycle badge

#### Scenario: The fixture has two legible cycles

- **WHEN** the explainer loads its pinned 24-city fixture
- **THEN** the symmetric difference decomposes into exactly two non-trivial AB-cycles and the parents share at least 60% of their edges

### Requirement: Interactive E-set selection

The explainer SHALL let the visitor select the crossover E-set with `1-AB`, `Rand`, or by clicking a cycle, SHALL keep at least one cycle selected, SHALL keep selected cycles visually distinct from unselected ones, and SHALL make `Rand` reproducible for a given seed.

#### Scenario: One-AB selects one cycle

- **WHEN** the visitor selects `1-AB`
- **THEN** exactly one AB-cycle is active and the others are dimmed

#### Scenario: Rand selects a seeded subset

- **WHEN** the visitor selects `Rand` with a given seed
- **THEN** the same seed selects the same cycle subset on every run and at least one cycle is active

#### Scenario: Clicking a cycle toggles it

- **WHEN** the visitor clicks a cycle badge or legend row
- **THEN** that cycle joins or leaves the E-set and the visualization updates

#### Scenario: The last cycle cannot be deselected

- **WHEN** the visitor tries to deselect the only active cycle
- **THEN** it stays selected and the explainer explains that at least one cycle is required for a crossover

### Requirement: Symmetric difference and subtours

The explainer SHALL apply the selected E-set to Parent A as a symmetric difference, animate A-edge removal and B-edge insertion, and identify the resulting connected components as subtours.

#### Scenario: A-edges dissolve and B-edges activate

- **WHEN** the visitor reaches the Apply E-set stage
- **THEN** A-edges in the E-set are removed, B-edges in the E-set are added, and every city keeps degree two

#### Scenario: Subtours are colour-coded

- **WHEN** the symmetric difference produces more than one component
- **THEN** each component is labelled and visually distinguished as a distinct subtour without reusing the AB-cycle badge palette

#### Scenario: Selecting every cycle reproduces Parent B

- **WHEN** the E-set contains every AB-cycle
- **THEN** the result is exactly Parent B, the explainer says no patching is needed, and the child is not presented as a patched crossover

#### Scenario: One component needs no patching

- **WHEN** the selected E-set already forms one connected cycle
- **THEN** the explainer says no patching is needed and moves to the final tour

### Requirement: Greedy subtour patching

The explainer SHALL merge subtours one bridge at a time, starting with the smallest component and choosing the cheapest reconnection among candidate edges, and SHALL report the exact cost delta of each bridge.

#### Scenario: Smallest subtour is patched first

- **WHEN** more than one subtour remains
- **THEN** the smallest component is highlighted before the bridge is applied

#### Scenario: Cheapest bridge is shown

- **WHEN** a patch step is active
- **THEN** the chosen removed edge pair and the two replacement edges are drawn, and their exact length delta is reported

#### Scenario: Patching ends in one tour

- **WHEN** every subtour has been merged
- **THEN** the explainer shows one connected Hamiltonian tour and its total cost

### Requirement: 2-opt polish preview

The explainer SHALL offer an optional one-move 2-opt preview on the patched child, showing the removed edges, the replacement edges, and the exact length delta.

#### Scenario: Preview shows the actual move

- **WHEN** the visitor enables the 2-opt preview
- **THEN** one best improving move on the child is drawn with its exact old/new lengths

#### Scenario: No fabricated improvement

- **WHEN** the preview is shown
- **THEN** every length and delta is computed from the fixture coordinates

#### Scenario: No improving move

- **WHEN** the child has no improving 2-opt move
- **THEN** the preview reports that the child is already 2-opt-local and draws no replacement edges

### Requirement: Teaching statistics explain quality preservation

The explainer SHALL display statistics that make edge inheritance legible: shared edges and ratio, AB-cycle count and lengths, E-set size, subtour count, bridges added, child versus parent costs, and the child's edge inheritance (A, B, or new).

#### Scenario: Edge inheritance is visible

- **WHEN** the final child tour is shown
- **THEN** the statistics report how many child edges came from A, from B, and were new

#### Scenario: Cost comparison is visible

- **WHEN** the final child tour is shown
- **THEN** the child's cost is compared with both parent costs using the fixture's exact distances

### Requirement: EAX explainer is accessible and responsive

The explainer SHALL be operable by keyboard, expose a text alternative for the visualization, and remain usable on supported viewport sizes.

#### Scenario: Keyboard operation

- **WHEN** a visitor uses only the keyboard
- **THEN** every stage, playback control, cycle toggle, strategy toggle, and preview toggle can be reached and operated

#### Scenario: Screen-reader summary

- **WHEN** a screen reader encounters the visualization
- **THEN** it receives the current stage, parent tours, AB-cycle summary, selected E-set, subtours, patches, and final statistics

#### Scenario: Cycle toggles expose their state

- **WHEN** a screen reader reads the cycle legend
- **THEN** each cycle toggle is a button whose selected state is exposed

#### Scenario: Narrow viewport

- **WHEN** the explainer is viewed on a narrow viewport
- **THEN** the graph, legend, controls, and statistics remain usable without horizontal scrolling

### Requirement: EAX explainer simulation is deterministic

The explainer SHALL produce the same parents, union graph, AB-cycles, E-set (for a given seed), subtours, bridge sequence, and child tour for the same fixture and selections.

#### Scenario: Repeated renders agree

- **WHEN** the visitor selects the same stage, strategy, seed, and cycle selection more than once
- **THEN** the explainer shows the same graph, cycles, subtours, patches, and costs each time

#### Scenario: Seeded Rand is stable

- **WHEN** the visitor reloads the explainer with the same `Rand` seed
- **THEN** the same cycles are selected

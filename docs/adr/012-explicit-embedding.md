# ADR 012: Derived 2D layouts for coordinate-less EXPLICIT instances

## Status

Proposed

## Context

TSPLIB EXPLICIT instances may carry no `NODE_COORD_SECTION` or `DISPLAY_DATA_SECTION` (for
example `gr17`). The parser fabricated `grid_coords`, and geometry-based solvers (SFC, Fourier,
SOM) and KD-tree consumers (Lin-Kernighan, branch and bound) treated those placeholders as real
geometry. Measured on `gr17` against the published optimum of 2085, the gaps were +158.8% (sfc),
+127.7% (fourier), +142.9% (som), and +97.8% (lk); matrix-only solvers were correct.

The constitution's input contract records that a coordinate-less EXPLICIT instance is a complete
problem and that the distance matrix remains the objective.

## Decision

- `process_lines` derives a deterministic 2D layout from the distance matrix with classical MDS
  when an EXPLICIT instance supplies no coordinates, and stores it in `TspLibData.cities`.
- `TspLibData.has_real_coordinates` records whether the file supplied coordinates.
  `DistanceSource::is_coordinate_metric()` keeps its distinct meaning — "distances are computed from
  these coordinates" — and stays false for every EXPLICIT matrix.
- Geometry-based constructors consume `TspProblem.cities` unchanged.
- Lin-Kernighan builds its candidate list from the exact distance matrix whenever
  `!DistanceSource::is_coordinate_metric()`, including EXPLICIT instances with display data.
- `LayoutPolicy::RequireReal` is the opt-out; it fails at problem construction, not inside a
  solver.

The eigen-solver is a hand-rolled power iteration with `f64` internals, zero-mean trigonometric
starts, per-iteration re-centering, unshifted deflation with a recomputed shift, and sign
canonicalization. It mirrors `scripts/lib/problem-minimap.mjs`, but corrects that script's
all-ones start and shifted deflation.

## Alternatives considered

- **Add a linear-algebra dependency** (`nalgebra`/`faer`): rejected; two eigenvectors do not
  justify the build weight.
- **Landmark MDS**: rejected for now; the EXPLICIT corpus is small enough for full MDS, and
  landmark selection adds a second approximation.
- **Lazy embedding in the problem builders**: rejected; `TspLibData::cities()` feeds the Qt,
  WASM, and API parse surfaces, and lazy embedding leaves them stale or inconsistent.
- **Clear error for geometry-based solvers**: rejected as the default; it leaves 12 of the 17
  tracked EXPLICIT datasets unsolved. Retained as `LayoutPolicy::RequireReal`.

## Consequences

- Coordinate-less EXPLICIT instances become solvable by every registered solver, with tours
  scored by the exact matrix.
- The derived layout is approximate for non-Euclidean matrices; benchmark rows and the problem
  page label it as derived.
- `TspProblem.cities` no longer means "file coordinates"; the two flags make the distinction
  explicit.
- `city-id-position`'s planned ADR 011 collides with the existing `011-remove-codecov.md`;
  whichever change lands second renumbers.

# ADR-010: A distance-source boundary with swappable backends

## Status

Proposed

## Context

`TspProblem` bundles a concrete `DistanceMatrix`: the lower triangle of a symmetric matrix,
`n·(n−1)/2` `f32` entries. That is O(n²) memory — about 18.6 GiB of packed distances at
100,000 cities, before the city tables and parser transients. A 100k instance cannot be
constructed on ordinary hardware, so the Space-Filling Curve solver's 100k claim is blocked.

Every solver reads distances through a small, uniform set of methods, but the concrete type
also exposes two id spaces: dense matrix positions and external city IDs. Tests build points
with 0-based IDs, TSPLIB uses 1-based IDs, and the API accepts arbitrary IDs, so
"position == id" holds in tests and fails in production. `KDPoint::new` setting `id = 0`, and
`nearest` excluding candidates by id (with the Fourier `usize::MAX` sentinel as a workaround),
are symptoms of that ambiguity.

## Decision

Introduce a `DistanceSource` trait as the single distance-query boundary:

- **Five required primitives:** `num_cities`, `distance_by_pos`, `pos2city_id`,
  `city_id2pos`, `point_at`.
- **Shared defaults:** `distance_between`, `tour_length`, `tour_length_by_pos`,
  `distances_from`, `nearest`, `nearest_unvisited`. Backends do not reimplement the loops, so
  summation order, id handling, and tie-breaking cannot drift between them.
- **`Send + Sync`** because the API moves problems into `spawn_blocking` and the Qt GUI moves
  them into worker threads.
- **`is_coordinate_metric()`** distinguishes a coordinate-computed source from an EXPLICIT matrix;
  candidate pruning must use the matrix rather than a layout when it is false.
- **Storage:** `TspProblem` holds `Arc<dyn DistanceSource>`. The source is immutable, `clone()`
  becomes O(1) instead of deep-copying the matrix, and the same source can be shared.
- **Backends:** `Matrix` (the current packed table) and `OnTheFly` (compute the TSPLIB
  primitive per lookup). Both call the same four distance primitives, which is what makes
  their results byte-identical by construction.
- **Selection:** keyed off `has_explicit_weights()`, never the overridable declared distance
  type, because an EXPLICIT matrix is the distance source and any coordinates are a display or
  derived layout. Coordinate input defaults to the matrix at or below a documented threshold and
  to on-the-fly above it.
- **Output:** the reported tour remains a sequence of city IDs. Positions are an internal
  distance-indexing detail and never leak into the CLI/API/WASM contract.

The full `CityId`/`Position` newtype refactor is a follow-up decision; this ADR establishes the
boundary and the contract it must obey.

## Alternatives considered

- **A closed enum instead of a trait.** Simpler dispatch and exhaustiveness, but every new
  backend edits the enum and its match, and the backend set is expected to grow (candidate
  lists, compact EXPLICIT).
- **A generic `TspProblem<D: DistanceSource>`.** No dynamic dispatch, but the generic ripples
  through every solver, the API, WASM, Qt, and the public signatures.
- **`Box<dyn DistanceSource>`.** Needs a `clone_box` method; `Arc` is cheaper and the source is
  immutable.
- **Keep the matrix and document a size limit.** Rejected: it leaves the 100k claim
  unimplementable and makes the eventual boundary harder to introduce.

## Consequences

- Solver code depends on the trait instead of the concrete matrix; the field type and helper
  signatures change once, in a mechanical PR.
- Byte-identity is structural through the shared defaults, and is still guarded by a
  conformance suite across every backend and distance type.
- Dynamic dispatch costs a vtable call in hot loops; the Phase 0 benchmark records the
  pre-change baseline and the refactor must stay within the documented 15% median-wall bound.
- Removing the inherent query methods requires `use DistanceSource` imports at concrete call
  sites; that churn is accepted so there is one implementation, not two.
- The trait still speaks raw `usize` ids and positions; ADR-011 will make the distinction
  compile-time visible.

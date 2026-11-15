# Design

## Context

See `proposal.md` for motivation. Current state that shapes the approach:

- `KDPoint.id` is a raw `usize` (`src/tsp/kdtree.rs:296`). `build_points` assigns `0..n`, TSPLIB assigns the file's IDs (`tsplib.rs:360-367`), and the API uses `c.id.unwrap_or(i + 1)`.
- `DistanceMatrix` keeps a `city_idx: HashMap<usize, usize>` id→position and a `CityTable` position→point; the new `DistanceSource` exposes `pos2city_id`/`city_id2pos` as the conversion boundary.
- `Solution` copies `cities` and rebuilds `cities_idx` at every construction (`mod.rs:1934`, `1951`); there are 34 `Solution::new`/`from_parts` sites.
- Solvers split conventions: position-based (`christofides`, `greedy_edge`, `savings`, `fourier`), ID-based (`two_opt`, `or_opt`, `nearest_neighbor`, `three_opt`, `lin_kernighan`), and converting (`ant_colony`, `bellman_karp`).
- `NearestResult::add` excludes by `pt.id == target.id`, which forced the Fourier `usize::MAX` sentinel.
- The CLI, API, and WASM expose ordered city IDs; the output contract must not change.

## Goals / Non-Goals

**Goals:**

- Make the position/ID distinction compile-time visible.
- Own one canonical mapping, instead of rebuilding it per `Solution`.
- One solver convention: positions internally, IDs at the tour/output boundary.
- Remove the `KDPoint::new` id-0 collision and the `nearest` sentinel workaround.
- Preserve every output byte and every distance semantic.

**Non-Goals:**

- Changing the TSPLIB input/output contract or the distance metrics.
- Exposing newtypes in JSON or the WIT surface; they convert to `u32`/`usize` at the boundary.
- Changing any solver algorithm or result.
- Reworking `TspProblem.cities` into a shared `Arc<[KDPoint]>`; that is a separate memory question.

## Decisions

### D1: `CityId(usize)` and `Position(usize)` newtypes

```rust
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct CityId(usize);

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Position(usize);
```

`into_inner()`/`value()` exposes the raw number at serialization boundaries. Newtypes are zero-cost, prevent indexing one space with the other, and make signatures self-documenting. **Alternatives:** type aliases (no enforcement, rejected); documentation only (the current state, rejected).

### D2: One `CityIndex` owns the mapping

```rust
pub struct CityIndex {
    ids: Vec<CityId>,                 // position -> id
    by_id: HashMap<CityId, Position>,
}
```

`TspProblem` owns an `Arc<CityIndex>`; the distance sources and `Solution` share it. Construction rejects duplicate IDs, mirroring `DistanceMatrix::new`/`OnTheFly::new`. `Solution` stops rebuilding `cities_idx` and holds the shared index plus its existing city copy. **Alternative:** each source keeps its own map — rejected, because that is exactly the duplication this change removes.

### D3: `KDPoint` holds a `CityId`

`pub struct KDPoint { pub id: CityId, pub coords: [f32; 2] }`. `KDPoint::new` (which silently sets `id = 0`) is removed; `new_with_id` becomes `new(id: CityId, coords)`. `build_points` assigns `CityId(i)` for tests, TSPLIB assigns the parsed ID, and the API assigns `CityId(c.id.unwrap_or(i + 1))`. This is the widest mechanical diff (every `KDPoint` literal and constructor), but it removes the collision at the source.

### D4: The distance-source API uses positions and IDs explicitly

```rust
fn distance_by_pos(&self, a: Position, b: Position) -> Result<f32, &'static str>;
fn distance_between(&self, a: CityId, b: CityId) -> Result<f32, &'static str>;
fn tour_length(&self, path: &[CityId]) -> f32;
fn tour_length_by_pos(&self, path: &[Position]) -> f32;
fn point_at(&self, pos: Position) -> Option<KDPoint>;
fn pos2city_id(&self, pos: Position) -> Option<CityId>;
fn city_id2pos(&self, id: CityId) -> Option<Position>;
fn distances_from(&self, id: CityId) -> Vec<f32>;
fn nearest(&self, target: Position, n: usize) -> NearestResult;
fn nearest_unvisited(
    &self,
    target: Position,
    is_candidate: &dyn Fn(CityId) -> bool,
) -> Option<NearestResultItem>;
```

`nearest` now takes the query point's position and excludes it explicitly, instead of inferring exclusion from `target.id`. That removes the sentinel and the id-collision footgun in one step. The trait is new in `distance-source`, so this is a source-level change in-repo, not a public compatibility break.

### D5: One solver convention

- Constructive/position solvers (`christofides`, `greedy_edge`, `savings`, `fourier`) keep `Vec<Position>` internally and map to `CityId` with `CityIndex::id_at` at the end.
- Local-search solvers (`two_opt`, `or_opt`, `three_opt`, `lin_kernighan`, `nearest_neighbor`) keep `Vec<CityId>` routes and call `city_id2pos` only when they need a matrix position.
- `ant_colony` and `bellman_karp` delete their local `to_pos`/`to_id` closures and use `CityIndex`.
- `Solution.route` is `Vec<CityId>`; `route()` returns `&[CityId]` and a `route_ids()` helper yields `usize` for the output surfaces.

### D6: Output surfaces convert at the edge

The CLI prints `CityId` as `usize`; the API and WASM map `CityId` to `u32` in their response builders. No JSON/WIT shape changes. `validate_tour` stays ID-based.

### D7: `TspProblem` owns the index

`TspProblem { cities: Vec<KDPoint>, index: Arc<CityIndex>, distances: Arc<dyn DistanceSource> }`. Constructors (`new`, `from_source`, `from_points`) build or accept the index once. The position invariant from `distance-source` (`index.id_at(i) == cities[i].id`, `index.len() == cities.len()`) is asserted once here.

## Risks / Trade-offs

- **Public surface churn** — `KDPoint.id` and `Solution::route()` types change; CLI/API/WASM/Qt must be updated in the same stack. WASM and Qt are out-of-workspace and easy to miss → build them explicitly.
- **Wide mechanical diff** — every `KDPoint` literal and 34 `Solution` sites → split into the PRs below; the newtypes make missed conversions compile errors rather than silent bugs.
- **Output contract regression** — a position leaking into the output would be a real bug → tests with 1-based and non-contiguous IDs assert the exact printed IDs.
- **`nearest` API change** — Fourier and six test call sites → update in the same PR that changes the trait signature.
- **Performance** — newtypes are `#[repr(transparent)]` and zero-cost; the only added lookup is the shared `CityIndex`, which replaces the per-`Solution` map.
- **Stacking on `distance-source`** — the trait signatures change again → land after the full `distance-source` stack (PR 10), when the trait and solver conventions are stable.

## Migration Plan

Behaviour-preserving stack:

1. Write ADR 011, then add `CityId`, `Position`, `CityIndex` and their tests; no consumer changes yet.
2. Switch `KDPoint.id` to `CityId`, update `build_points`/TSPLIB/API construction, `Solution.route` to `Vec<CityId>`, and all 34 `Solution` sites.
3. Switch the `DistanceSource` signatures to the newtypes and `nearest` to `Position`; update Fourier and the six test sites.
4. Migrate solvers to the one convention and delete the local `to_pos`/`to_id` converters.
5. Remove `KDPoint::new` (id-0) and the Fourier sentinel; update docs, AGENTS.md core types, and graphify.

Rollback is a stack revert; no data migration.

## Open Questions

- Whether `route()` should return `&[CityId]` with a separate `route_ids()` (preferred) or keep `&[usize]` to reduce surface churn.
- Whether `TspProblem.cities` should become `Arc<[KDPoint]>` so `Solution` shares the city copy — defer; it is a memory question, not an identity one.

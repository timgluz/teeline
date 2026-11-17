# ADR-013: Position boundary and deferred CityId newtype

## Status

Proposed

## Context

City IDs and distance-source positions are both raw `usize`. A 23-solver census and a
critical review found that the original plan — `CityId` and `Position` newtypes across
every solver, 102 `KDPoint` sites, and 35 `Solution` sites — spends most of its effort
in code where the two spaces never meet. The reachable defects are narrower:

- `lin_kernighan` sizes its scratch arrays by the largest city ID (`vec![0; max_id + 1]`),
  and the API accepts arbitrary `id: usize`, so a few huge IDs can abort the process.
- Coordinate input with duplicate IDs reaches an `assert!` in `DistanceMatrix::from_parts`;
  only EXPLICIT input is validated.
- The id→position mapping is duplicated in `DistanceMatrix`, `OnTheFly`, `CandidateList`,
  and a per-`Solution` map that has no remaining callers.

## Decision

1. Introduce `Position(usize)` only, and type the boundary where positions meet IDs:
   the `DistanceSource` position methods, `CandidateList`, and the position-internal
   solvers. `CityId`, `KDPoint.id`, `Solution.route`, `Route`, `init_tour`,
   `ProgressMessage`, and `validate_tour` stay `usize`.
2. The distance source owns the canonical id→position index. `IdIndex::build` becomes
   fallible and rejects duplicate IDs; `TspProblem` does not build a second index.
3. `lin_kernighan` remaps IDs to positions on entry and back on exit, so its scratch
   arrays scale with the city count, not the largest ID.
4. `graph.rs` keeps its packed `(f32, u32, u32)` edge storage; `Position` may appear
   only at its `usize` boundary. This preserves the deliberate memory win of the
   O(n²) edge list.
5. Delete the dead `Solution` state (`cities`, `cities_idx`, `cities()`,
   `get_by_city_id()`) and the unused, ID-keyed `city_table_from_vec`.

## Alternatives considered

- **Full `CityId` + `Position` migration.** Rejected: most of the churn is in ID-only
  solver code, it does not fix the reachable defects, and the byte-identical output
  gate cannot cover the unseeded stochastic solvers.
- **Bug fixes only.** Rejected as the sole scope: it leaves the duplicated index and
  the position/ID boundary untyped.
- **A new `CityIndex` with an unconditional `HashMap`.** Rejected: `IdIndex` already
  has a dense/sparse strategy that must be retained.

## Consequences

- A position cannot be passed where an ID is expected, and vice versa, at the trait and
  candidate-list boundary.
- LK memory becomes O(n); duplicate IDs return an error instead of panicking or trapping.
- The `CityId` distinction remains untyped. Revisit if an ID-based solver moves to
  positions for speed, or if another ID/position bug lands in ID-space solver code.
- `graph.rs`'s memory profile is unchanged.
- Deterministic solvers can still be compared byte for byte; stochastic solvers must be
  verified as valid permutations plus a quality budget until they are seeded.

// Canonical benchmark solver index for the website.
//
// The data lives in the repo-root `bench/solvers.json`, which is exactly what the
// shell benchmark and publish scripts read (via jq). This module parses that same
// file rather than keeping a second copy, so a solver cannot be added, renamed, or
// re-classified on one side only. An earlier version duplicated the list here and
// relied on a test to catch drift; that test is still valuable for the nav-data
// cross-checks below, but the duplication itself is now gone.
//
// The file is loaded with `?raw` (a Vite feature typed by vite/client) instead of
// a JSON import: `bench/` sits outside `src/`, so a JSON import would be a
// non-TypeScript-extension file outside `rootDir` and would make `tsc --noEmit`
// fail on the repo. `?raw` is a plain string import, which tsc accepts and Vite
// resolves at build time.

import rawIndex from '../../../bench/solvers.json?raw'

export type SolverFamily =
  'exact' | 'constructive' | 'local' | 'meta' | 'utility'

export interface SolverIndexEntry {
  /** Algorithm doc id — also the `/algorithms/{id}/` route param and R2 shard name. */
  id: string
  /** Canonical CLI name (`teeline solvers` NAME column, `Solvers::variants()`). */
  cli: string
  family: SolverFamily
  /**
   * Whether repeated runs of the same command produce the identical tour.
   *
   * **Measured, never inferred.** Each `true` is backed by `determinismEvidence`
   * recording the instance and run count, because reading the source is not
   * enough: `2opt`, `3opt` and `or_opt` look deterministic (they are plain local
   * search) but auto-seed from `nn`, so they inherit its variance and are marked
   * `false` until the `nn` bug is fixed.
   *
   * When this is `false`, the published gap is a distribution (median/best/worst)
   * and the UI must not present a single run as *the* result.
   */
  deterministic: boolean
  /** What was actually run to justify the `deterministic` claim. Null when unmeasured. */
  determinismEvidence: {
    instance: string
    runs: number
    outcome: string
  } | null
  /** Filename of the algorithm doc under docs/algorithms/, without extension. */
  doc: string | null
  /** Configs the harness should measure (`default`, `no-seed`, `exact`, ...). */
  configs: readonly string[]
}

interface RawSolver {
  id: string
  cli: string
  family: string
  deterministic: boolean
  determinism_evidence: {
    instance: string
    runs: number
    outcome: string
  } | null
  doc: string | null
  default_configs: string[]
}

const parsed = JSON.parse(rawIndex) as {
  schema_version: number
  solvers: RawSolver[]
}

export const SOLVER_INDEX_SCHEMA_VERSION: number = parsed.schema_version

export const SOLVER_INDEX: readonly SolverIndexEntry[] = parsed.solvers.map(
  (s) => ({
    id: s.id,
    cli: s.cli,
    family: s.family as SolverFamily,
    deterministic: s.deterministic,
    determinismEvidence: s.determinism_evidence,
    doc: s.doc,
    configs: s.default_configs ?? ['default'],
  }),
)

const BY_ID = new Map(SOLVER_INDEX.map((s) => [s.id, s]))

/** Looks up a solver by doc id. Returns undefined for ids with no benchmark entry. */
export function solverById(id: string): SolverIndexEntry | undefined {
  return BY_ID.get(id)
}

/**
 * Whether a solver produces the identical tour on every run.
 *
 * Drives whether the UI may show a single number or must show a distribution.
 * Unknown ids return `false`: the safe default is to avoid presenting an
 * unmeasured solver's one run as authoritative.
 */
export function isDeterministic(id: string): boolean {
  return BY_ID.get(id)?.deterministic ?? false
}

/** The configs the harness should measure for a solver (never empty). */
export function solverConfigs(id: string): readonly string[] {
  return BY_ID.get(id)?.configs ?? ['default']
}

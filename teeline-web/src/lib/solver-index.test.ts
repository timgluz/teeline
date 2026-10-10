import { describe, expect, it } from 'vitest'

// `?raw` (a Vite feature, typed by vite/client) reads the canonical file as a
// string without needing filesystem APIs — so this test adds no `node` types to
// the project's tsconfig, which would otherwise let browser-targeted code under
// src/ reference Node globals like `process` or `Buffer` without tsc complaining.
import canonicalRaw from '../../../bench/solvers.json?raw'

import { SOLVER_GROUPS, SOLVER_META } from '../nav-data'
import { SOLVER_INDEX, isDeterministic, solverById } from './solver-index'

// ---------------------------------------------------------------------------
// The repo-root bench/solvers.json is the canonical list read by the shell
// benchmark/publish scripts; SOLVER_INDEX is the website's copy. These tests
// exist so the two can never silently drift — a mismatch means the website is
// displaying results for a solver the harness never ran, or vice versa.
// ---------------------------------------------------------------------------

interface CanonicalSolver {
  id: string
  cli: string
  family: string
  deterministic: boolean
  doc: string | null
  determinism_evidence: {
    instance: string
    runs: number
    outcome: string
  } | null
}

const canonical = JSON.parse(canonicalRaw) as {
  schema_version: number
  solvers: CanonicalSolver[]
}

describe('solver-index vs bench/solvers.json', () => {
  it('covers exactly the same solver ids', () => {
    expect(SOLVER_INDEX.map((s) => s.id).sort()).toEqual(
      canonical.solvers.map((s) => s.id).sort(),
    )
  })

  it('agrees on every field that both files carry', () => {
    for (const entry of SOLVER_INDEX) {
      const other = canonical.solvers.find((s) => s.id === entry.id)
      expect(other, `bench/solvers.json is missing ${entry.id}`).toBeDefined()
      expect(
        {
          cli: entry.cli,
          family: entry.family,
          deterministic: entry.deterministic,
          doc: entry.doc,
        },
        `field mismatch for ${entry.id}`,
      ).toEqual({
        cli: other!.cli,
        family: other!.family,
        deterministic: other!.deterministic,
        doc: other!.doc,
      })
    }
  })

  it('has no duplicate ids or CLI names', () => {
    const ids = SOLVER_INDEX.map((s) => s.id)
    const clis = SOLVER_INDEX.map((s) => s.cli)
    expect(new Set(ids).size).toBe(ids.length)
    expect(new Set(clis).size).toBe(clis.length)
  })
})

describe('solver-index vs nav-data', () => {
  it('covers every solver that has an algorithm page', () => {
    // nav-data carries exactly the documented algorithms; `shuffle` is a
    // benchmark-only baseline with no page, so it is the one expected extra.
    const indexIds = SOLVER_INDEX.map((s) => s.id)
      .filter((id) => id !== 'shuffle')
      .sort()
    expect(indexIds).toEqual(Object.keys(SOLVER_META).sort())
  })

  it('uses the same families as the algorithms-index groups', () => {
    const familyOf = new Map(SOLVER_INDEX.map((s) => [s.id, s.family]))
    const familyByGroup: Record<string, string> = {
      Exact: 'exact',
      Constructive: 'constructive',
      'Local search': 'local',
      Metaheuristic: 'meta',
    }
    for (const group of SOLVER_GROUPS) {
      const expected = familyByGroup[group.label]
      expect(
        expected,
        `unmapped SOLVER_GROUPS label ${group.label}`,
      ).toBeDefined()
      for (const id of group.ids) {
        expect(familyOf.get(id), `missing family for ${id}`).toBe(expected)
      }
    }
  })

  it('marks every documented solver with a docs slug', () => {
    for (const entry of SOLVER_INDEX) {
      if (entry.id === 'shuffle') continue
      expect(entry.doc, `${entry.id} has no docs slug`).toBeTruthy()
    }
  })
})

describe('determinism claims', () => {
  it('requires evidence for every solver claimed deterministic', () => {
    // A `deterministic: true` flag drives the UI to show one run as *the*
    // result. That claim must never be inferred from reading the source — an
    // earlier hand-checked guess marked 2opt deterministic because it happened
    // to be stable on berlin52, while it actually varies on a280. Require that
    // each claim carries a recorded measurement.
    for (const solver of canonical.solvers) {
      if (!solver.deterministic) continue
      expect(
        solver.determinism_evidence,
        `${solver.id} claims determinism with no recorded evidence`,
      ).not.toBeNull()
      expect(solver.determinism_evidence?.outcome).toBe('identical')
      expect(solver.determinism_evidence?.runs).toBeGreaterThanOrEqual(2)
    }
  })

  it('never claims determinism from a single run', () => {
    for (const solver of canonical.solvers) {
      const evidence = solver.determinism_evidence
      if (!evidence) continue
      expect(
        evidence.runs,
        `${solver.id}'s evidence uses too few runs to distinguish stable from lucky`,
      ).toBeGreaterThanOrEqual(2)
    }
  })

  it('treats nn-seeded solvers as a coupled set', () => {
    // 2opt/3opt/or_opt auto-seed from nn (main.rs:388). While nn is
    // non-deterministic they cannot be deterministic, because their starting
    // tour is not. Pin that coupling so fixing nn prompts a re-measurement of
    // all three rather than silently leaving a stale claim behind.
    const nn = canonical.solvers.find((s) => s.id === 'nn')
    const seeded = ['2opt', '3opt', 'or_opt']
    if (nn && !nn.deterministic) {
      for (const id of seeded) {
        const solver = canonical.solvers.find((s) => s.id === id)
        expect(
          solver?.deterministic,
          `${id} is marked deterministic while its nn seed is not`,
        ).toBe(false)
      }
    }
  })
})

describe('solverById / isDeterministic', () => {
  it('looks up by doc id', () => {
    expect(solverById('som')?.cli).toBe('kohonen_som')
    expect(solverById('2opt')?.cli).toBe('two_opt')
  })

  it('returns undefined for unknown ids', () => {
    expect(solverById('not_a_solver')).toBeUndefined()
    expect(isDeterministic('not_a_solver')).toBe(false)
  })

  it('defaults unknown solvers to non-deterministic', () => {
    // The safe default: an unmeasured solver must not be presented as producing
    // one authoritative number.
    expect(isDeterministic('bhk')).toBe(true)
    expect(isDeterministic('sa')).toBe(false)
  })

  it('does not claim determinism for nn-seeded local search', () => {
    expect(isDeterministic('2opt')).toBe(false)
    expect(isDeterministic('3opt')).toBe(false)
    expect(isDeterministic('or_opt')).toBe(false)
  })
})

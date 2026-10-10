import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

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
}

const canonical = JSON.parse(
  readFileSync(
    fileURLToPath(new URL('../../../bench/solvers.json', import.meta.url)),
    'utf8',
  ),
) as { schema_version: number; solvers: CanonicalSolver[] }

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
})

import { describe, it, expect } from 'vitest'
import { SOLVER_META, SOLVER_GROUPS, PAGED_SOLVERS, EXPLAINER_SOLVERS } from './nav-data'

describe('SOLVER_META', () => {
  it('contains exactly 21 solvers', () => {
    expect(Object.keys(SOLVER_META)).toHaveLength(21)
  })

  it('every entry has id and name', () => {
    for (const [key, meta] of Object.entries(SOLVER_META)) {
      expect(meta.id).toBe(key)
      expect(meta.name.length).toBeGreaterThan(0)
    }
  })
})

describe('SOLVER_GROUPS', () => {
  it('contains exactly 4 groups', () => {
    expect(SOLVER_GROUPS).toHaveLength(4)
  })

  it('every group has a non-empty label and ids', () => {
    for (const g of SOLVER_GROUPS) {
      expect(g.label.length).toBeGreaterThan(0)
      expect(g.ids.length).toBeGreaterThan(0)
    }
  })

  it('has no duplicate IDs across groups', () => {
    const allIds = SOLVER_GROUPS.flatMap(g => g.ids)
    expect(allIds).toHaveLength(new Set(allIds).size)
  })

  it('every ID in groups exists in SOLVER_META', () => {
    const allIds = SOLVER_GROUPS.flatMap(g => g.ids)
    for (const id of allIds) {
      expect(SOLVER_META, `missing SOLVER_META entry for "${id}"`).toHaveProperty(id)
    }
  })

  it('every SOLVER_META entry appears in exactly one group', () => {
    const allIds = SOLVER_GROUPS.flatMap(g => g.ids)
    for (const id of Object.keys(SOLVER_META)) {
      expect(allIds, `"${id}" not found in any group`).toContain(id)
    }
  })
})

describe('complexity', () => {
  it('matches each docs page Complexity row (so index and docs agree)', () => {
    // Vite ?raw glob — no node:fs needed
    const docs = import.meta.glob<string>('../../docs/algorithms/*.md', { query: '?raw', import: 'default', eager: true })
    const files = Object.keys(docs)
    expect(files.length).toBeGreaterThan(0)
    let checked = 0
    for (const f of files) {
      const md = docs[f]
      const id = md.match(/^id: "([^"]+)"/m)?.[1]
      if (!id || !(id in SOLVER_META)) continue
      const row = md.match(/^\| \*\*Complexity\*\* \| (.*) \|$/m)?.[1]
      expect(row, `${f} must have a Complexity row`).not.toBeUndefined()
      expect(row, `${f} Complexity row must match nav-data`).toBe(SOLVER_META[id].complexity)
      checked++
    }
    expect(checked).toBe(Object.keys(SOLVER_META).length)
  })
})

describe('PAGED_SOLVERS', () => {
  it('equals the full SOLVER_META id set (every solver gets a doc page)', () => {
    expect(PAGED_SOLVERS).toEqual(new Set(Object.keys(SOLVER_META)))
  })
})

describe('EXPLAINER_SOLVERS', () => {
  it('contains exactly the 13 ids with an interactive explainer', () => {
    expect(EXPLAINER_SOLVERS).toEqual(
      new Set(['pso', 'gsa', 'tabu', 'ga', 'cs', 'fpa', 'lk', 'sa', 'som', 'fourier', 'greedy_edge', 'savings', 'aco']),
    )
  })

  it('every explainer ID exists in SOLVER_META', () => {
    for (const id of EXPLAINER_SOLVERS) {
      expect(SOLVER_META, `EXPLAINER_SOLVERS has "${id}" but SOLVER_META does not`).toHaveProperty(id)
    }
  })
})

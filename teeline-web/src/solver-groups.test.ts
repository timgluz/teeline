import { describe, it, expect } from 'vitest'
import { SOLVER_META, SOLVER_GROUPS, PAGED_SOLVERS } from './solver-groups'

describe('SOLVER_META', () => {
  it('contains exactly 17 solvers', () => {
    expect(Object.keys(SOLVER_META)).toHaveLength(17)
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

describe('PAGED_SOLVERS', () => {
  it('every paged ID exists in SOLVER_META', () => {
    for (const id of PAGED_SOLVERS) {
      expect(SOLVER_META, `PAGED_SOLVERS has "${id}" but SOLVER_META does not`).toHaveProperty(id)
    }
  })
})

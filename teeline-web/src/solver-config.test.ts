import { describe, it, expect } from 'vitest'
import { SOLVERS, solverByAlias, paramsFor } from './solver-config'
import { defaultSolveOptions } from './solver-options'

describe('SOLVERS array', () => {
  it('has exactly 13 entries', () => {
    expect(SOLVERS).toHaveLength(13)
  })

  it('every entry has non-empty name, alias, kind, and params array', () => {
    for (const s of SOLVERS) {
      expect(s.name.length, `${s.alias} name`).toBeGreaterThan(0)
      expect(s.alias.length, `${s.alias} alias`).toBeGreaterThan(0)
      expect(s.kind.length, `${s.alias} kind`).toBeGreaterThan(0)
      expect(Array.isArray(s.params), `${s.alias} params`).toBe(true)
    }
  })

  it('aliases are unique', () => {
    const aliases = SOLVERS.map((s) => s.alias)
    expect(new Set(aliases).size).toBe(aliases.length)
  })

  it('bhk and branch_bound have kind exact', () => {
    expect(solverByAlias('bhk')?.kind).toBe('exact')
    expect(solverByAlias('branch_bound')?.kind).toBe('exact')
  })

  it('nn has kind constructive', () => {
    expect(solverByAlias('nn')?.kind).toBe('constructive')
  })

  it('2opt and 3opt have kind local-search', () => {
    expect(solverByAlias('2opt')?.kind).toBe('local-search')
    expect(solverByAlias('3opt')?.kind).toBe('local-search')
  })

  it('sa, ga, pso, cs, fpa, stochastic_hill, tabu_search have kind metaheuristic', () => {
    const metaheuristics = ['sa', 'ga', 'pso', 'cs', 'fpa', 'stochastic_hill', 'tabu_search']
    for (const alias of metaheuristics) {
      expect(solverByAlias(alias)?.kind, alias).toBe('metaheuristic')
    }
  })
})

describe('solverByAlias', () => {
  it('returns correct meta for known aliases', () => {
    expect(solverByAlias('nn')?.alias).toBe('nn')
    expect(solverByAlias('sa')?.alias).toBe('sa')
    expect(solverByAlias('ga')?.alias).toBe('ga')
    expect(solverByAlias('cs')?.alias).toBe('cs')
    expect(solverByAlias('fpa')?.alias).toBe('fpa')
    expect(solverByAlias('2opt')?.alias).toBe('2opt')
    expect(solverByAlias('bhk')?.alias).toBe('bhk')
  })

  it('returns undefined for unknown alias', () => {
    expect(solverByAlias('unknown')).toBeUndefined()
    expect(solverByAlias('')).toBeUndefined()
  })
})

describe('param coverage — exact and constructive solvers', () => {
  it('bhk, branch_bound, nn have empty params array', () => {
    expect(paramsFor('bhk')).toHaveLength(0)
    expect(paramsFor('branch_bound')).toHaveLength(0)
    expect(paramsFor('nn')).toHaveLength(0)
  })
})

describe('param coverage — SA', () => {
  it('includes epochs, platooEpochs, nNearest, coolingRate, maxTemperature, minTemperature', () => {
    const keys = paramsFor('sa').map((p) => p.key)
    expect(keys).toContain('epochs')
    expect(keys).toContain('platooEpochs')
    expect(keys).toContain('nNearest')
    expect(keys).toContain('coolingRate')
    expect(keys).toContain('maxTemperature')
    expect(keys).toContain('minTemperature')
  })

  it('coolingRate has type float with 0 < min and max <= 1', () => {
    const p = paramsFor('sa').find((p) => p.key === 'coolingRate')!
    expect(p.type).toBe('float')
    expect(p.min).toBeGreaterThan(0)
    expect(p.max).toBeLessThanOrEqual(1)
  })

  it('maxTemperature has type float with min > 0', () => {
    const p = paramsFor('sa').find((p) => p.key === 'maxTemperature')!
    expect(p.type).toBe('float')
    expect(p.min).toBeGreaterThan(0)
  })
})

describe('param coverage — GA', () => {
  it('includes epochs, mutationProbability, nElite', () => {
    const keys = paramsFor('ga').map((p) => p.key)
    expect(keys).toContain('epochs')
    expect(keys).toContain('mutationProbability')
    expect(keys).toContain('nElite')
  })

  it('mutationProbability has type float with min 0 and max 1', () => {
    const p = paramsFor('ga').find((p) => p.key === 'mutationProbability')!
    expect(p.type).toBe('float')
    expect(p.min).toBe(0)
    expect(p.max).toBe(1)
  })

  it('nElite has type int', () => {
    const p = paramsFor('ga').find((p) => p.key === 'nElite')!
    expect(p.type).toBe('int')
  })
})

describe('param coverage — CS and FPA', () => {
  it('both include mutationProbability', () => {
    expect(paramsFor('cs').map((p) => p.key)).toContain('mutationProbability')
    expect(paramsFor('fpa').map((p) => p.key)).toContain('mutationProbability')
  })

  it('neither includes nElite or temperature params', () => {
    for (const alias of ['cs', 'fpa']) {
      const keys = paramsFor(alias).map((p) => p.key)
      expect(keys, alias).not.toContain('nElite')
      expect(keys, alias).not.toContain('coolingRate')
      expect(keys, alias).not.toContain('maxTemperature')
      expect(keys, alias).not.toContain('minTemperature')
    }
  })
})

describe('all param keys are valid SolveOptions fields', () => {
  it('every param.key is a key of defaultSolveOptions()', () => {
    const validKeys = new Set(Object.keys(defaultSolveOptions()))
    for (const solver of SOLVERS) {
      for (const param of solver.params) {
        expect(validKeys.has(param.key), `${solver.alias}.${param.key}`).toBe(true)
      }
    }
  })
})

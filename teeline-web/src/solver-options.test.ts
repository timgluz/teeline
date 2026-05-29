import { describe, it, expect } from 'vitest'
import { defaultSolveOptions } from './solver-options'

describe('defaultSolveOptions', () => {
  it('returns an object with all required SolveOptions fields', () => {
    const opts = defaultSolveOptions()
    expect(typeof opts.epochs).toBe('number')
    expect(typeof opts.platooEpochs).toBe('number')
    expect(typeof opts.coolingRate).toBe('number')
    expect(typeof opts.maxTemperature).toBe('number')
    expect(typeof opts.minTemperature).toBe('number')
    expect(typeof opts.mutationProbability).toBe('number')
    expect(typeof opts.nElite).toBe('number')
    expect(typeof opts.nNearest).toBe('number')
  })

  it('minTemperature is less than maxTemperature', () => {
    const opts = defaultSolveOptions()
    expect(opts.minTemperature).toBeLessThan(opts.maxTemperature)
  })

  it('epochs is a positive integer', () => {
    const opts = defaultSolveOptions()
    expect(opts.epochs).toBeGreaterThan(0)
    expect(Number.isInteger(opts.epochs)).toBe(true)
  })
})

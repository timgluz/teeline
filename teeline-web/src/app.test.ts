import { describe, it, expect } from 'vitest'
import { appTitle } from './app'

describe('app', () => {
  it('exports the app title', () => {
    expect(appTitle()).toBe('Teeline TSP Solver')
  })
})

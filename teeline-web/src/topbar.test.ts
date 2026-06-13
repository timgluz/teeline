import { describe, it, expect } from 'vitest'
import { renderTopbarHtml } from './topbar'

describe('renderTopbarHtml', () => {
  const html = renderTopbarHtml()

  it('includes a link to the home page', () => {
    expect(html).toContain('href="/"')
  })

  it('includes the site name', () => {
    expect(html).toContain('teeline')
  })

  it('includes Algorithms dropdown (details/summary)', () => {
    expect(html).toContain('Algorithms')
    expect(html).toContain('<details')
    expect(html).toContain('<summary>')
  })

  it('includes a link to the fourier solver page', () => {
    expect(html).toContain('/algorithms/fourier/')
  })

  it('groups solvers by category', () => {
    expect(html).toContain('Exact')
    expect(html).toContain('Constructive')
    expect(html).toContain('Local search')
    expect(html).toContain('Metaheuristic')
  })

  it('includes a GitHub link', () => {
    expect(html).toContain('github.com')
  })
})

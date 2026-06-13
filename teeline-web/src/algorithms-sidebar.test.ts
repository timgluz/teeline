import { describe, it, expect } from 'vitest'
import { renderSidebarHtml } from './algorithms-sidebar'

describe('renderSidebarHtml', () => {
  it('renders all four solver group labels', () => {
    const html = renderSidebarHtml(null)
    expect(html).toContain('Exact')
    expect(html).toContain('Constructive')
    expect(html).toContain('Local search')
    expect(html).toContain('Metaheuristic')
  })

  it('renders fourier as an anchor when not current', () => {
    const html = renderSidebarHtml(null)
    expect(html).toContain('href="/algorithms/fourier"')
  })

  it('marks the current solver with aria-current="page"', () => {
    const html = renderSidebarHtml('fourier')
    expect(html).toContain('aria-current="page"')
  })

  it('does not add aria-current when currentId is null', () => {
    const html = renderSidebarHtml(null)
    expect(html).not.toContain('aria-current="page"')
  })

  it('renders current solver as span (not anchor)', () => {
    const html = renderSidebarHtml('fourier')
    // When fourier is current, it should NOT have a link to /algorithms/fourier
    expect(html).not.toContain('href="/algorithms/fourier"')
  })

  it('renders non-paged solvers as plain text (no link)', () => {
    const html = renderSidebarHtml(null)
    // bhk has no docs page yet — should be a span, not an anchor
    expect(html).not.toContain('href="/algorithms/bhk"')
  })
})

import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

// Each algorithm doc's summary table carries a row that jumps to the benchmark section on the
// same page. The row is hand-written into 21 markdown files and the anchor is rendered by a
// component, so the two can drift apart silently: a renamed section id or a new doc without the
// row produces a link that scrolls nowhere, and nothing else would notice.

const root = fileURLToPath(new URL('..', import.meta.url))
const docsDir = join(root, '..', 'docs', 'algorithms')
const componentPath = join(root, 'src', 'components', 'AlgorithmBenchmarks.tsx')

const ANCHOR = '#benchmarks'
const ROW = /^\| \*\*Benchmarks\*\* \| \[[^\]]+\]\(#benchmarks\) \|$/m

describe('algorithm docs link to their benchmark section', () => {
  const docs = readdirSync(docsDir).filter((name) => name.endsWith('.md'))

  it('covers every algorithm doc', () => {
    expect(docs.length).toBeGreaterThan(20)
  })

  it('gives every doc exactly one jump-to-benchmarks row', () => {
    const missing = []
    for (const name of docs) {
      const text = readFileSync(join(docsDir, name), 'utf8')
      const matches = text.match(new RegExp(ROW, 'gm')) ?? []
      if (matches.length !== 1) missing.push(`${name} (${matches.length})`)
    }
    expect(missing).toEqual([])
  })

  it('puts the row inside the summary table, not loose in the body', () => {
    // The row only reads as metadata if it sits among the `| **Alias** |` rows.
    for (const name of docs) {
      const lines = readFileSync(join(docsDir, name), 'utf8').split('\n')
      const rowAt = lines.findIndex((line) =>
        line.startsWith('| **Benchmarks** |'),
      )
      expect(rowAt, `${name} has no row`).toBeGreaterThan(-1)
      expect(
        lines[rowAt - 1]?.startsWith('|'),
        `${name} breaks the table`,
      ).toBe(true)
    }
  })

  it('renders the anchor the docs point at', () => {
    // The coupling that would silently break all 21 links at once.
    expect(readFileSync(componentPath, 'utf8')).toContain(
      `id="${ANCHOR.slice(1)}"`,
    )
  })

  it('keeps the anchor in every state the section can render', () => {
    // Loading, error and ready each render their own <section>; a jump works at first paint
    // only if the anchor is in the server-rendered placeholder too.
    const source = readFileSync(componentPath, 'utf8')
    const sections = source.match(/<section/g) ?? []
    const anchored =
      source.match(new RegExp(`id="${ANCHOR.slice(1)}"`, 'g')) ?? []
    expect(anchored.length).toBe(sections.length)
  })
})

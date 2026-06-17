#!/usr/bin/env node
// Generates algorithms/<id>/index.html from docs/algorithms/<name>.md at build time.
// Run before vite build so Vite finds all MPA entry points.
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs'
import { fileURLToPath } from 'url'
import { dirname, join } from 'path'
import { Marked } from 'marked'
import { highlightText } from '@speed-highlight/core'
import { Eta } from 'eta'

const __dirname = dirname(fileURLToPath(import.meta.url))
const WEB_ROOT = join(__dirname, '..')
const DOCS_ROOT = join(WEB_ROOT, '../docs/algorithms')

const eta = new Eta({ views: join(__dirname, 'templates') })

// Solver ID → source filename in docs/algorithms/
const SOLVER_DOCS = {
  'bhk':          'bellman-held-karp.md',
  'branch_bound': 'branch-bound.md',
  'nn':           'nearest-neighbor.md',
  'fourier':      'fourier.md',
  'christofides': 'christofides.md',
  '2opt':         'two-opt.md',
  '3opt':         'three-opt.md',
  'or_opt':       'or-opt.md',
  'sa':           'simulated-annealing.md',
  'tabu':         'tabu-search.md',
  'ga':           'genetic-algorithm.md',
  'pso':          'particle-swarm.md',
  'cs':           'cuckoo-search.md',
  'fpa':          'flower-pollination.md',
  'gsa':          'gravitational-search.md',
  'lk':           'lin-kernighan.md',
  'som':          'som.md',
}

// Extract page metadata directly from the Markdown source — no frontmatter needed.
function extractMeta(md, solverId) {
  const name = md.match(/^#\s+(.+)$/m)?.[1]?.trim() ?? solverId

  // Pull the Type row from the metadata table: | **Type** | value |
  const typeBadge = md.match(/\|\s*\*\*Type\*\*\s*\|\s*([^|\n]+?)\s*\|/)?.[1]?.trim() ?? ''

  // First paragraph under ## Description, stripped of markdown formatting, for <meta>
  const descSection = md.match(/^## Description\n+([\s\S]+?)(?=\n##|\n```|\n\||\n\n\n|$)/m)
  const description = (descSection?.[1]?.trim().split('\n\n')[0] ?? '')
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/\n/g, ' ')
    .slice(0, 250)
    .trim()

  return { name, typeBadge, description }
}

const HIGHLIGHT_LANGS = new Set(['bash', 'rs', 'ts', 'js', 'toml', 'yaml', 'json'])

function safeLang(lang) {
  return HIGHLIGHT_LANGS.has(lang) ? lang : 'plain'
}

// Pre-process: replace fenced code blocks with highlighted HTML before marked parses.
// marked.parse() with { html: true } passes raw HTML through unchanged.
// walkTokens doesn't support async in marked v18, so this two-pass approach is used instead.
async function preHighlightCode(md) {
  const fence = /^```(\w*)\n([\s\S]*?)^```/gm
  const matches = [...md.matchAll(fence)]
  if (matches.length === 0) return md

  const htmlBlocks = await Promise.all(
    matches.map(async m => {
      const lang = safeLang(m[1])
      const inner = await highlightText(m[2].trimEnd(), lang, false)
      return `<pre><code class="shj-lang-${lang}">${inner}</code></pre>`
    })
  )

  // Replace each fence in reverse order to preserve string indices
  let result = md
  for (let i = matches.length - 1; i >= 0; i--) {
    const { index } = matches[i]
    result = result.slice(0, index) + htmlBlocks[i] + result.slice(index + matches[i][0].length)
  }
  return result
}

function makeMarked() {
  let isFirstTable = true

  const m = new Marked()
  m.use({
    renderer: {
      table(token) {
        const cls = isFirstTable ? ' class="docs-meta-table"' : ''
        isFirstTable = false
        const headerCells = token.header
          .map(c => `<th>${this.parser.parseInline(c.tokens)}</th>`)
          .join('')
        const bodyRows = token.rows
          .map(row =>
            `<tr>${row.map(c => `<td>${this.parser.parseInline(c.tokens)}</td>`).join('')}</tr>`
          )
          .join('\n        ')
        return `<table${cls}>\n  <thead><tr>${headerCells}</tr></thead>\n  <tbody>\n        ${bodyRows}\n  </tbody>\n</table>\n`
      },
    },
  })

  return {
    marked: m,
    resetFirstTable() { isFirstTable = true },
  }
}

const { marked, resetFirstTable } = makeMarked()

for (const [solverId, filename] of Object.entries(SOLVER_DOCS)) {
  const srcPath = join(DOCS_ROOT, filename)
  if (!existsSync(srcPath)) {
    console.warn(`[build-docs] skipping ${solverId}: docs/algorithms/${filename} not found`)
    continue
  }

  const md = readFileSync(srcPath, 'utf8')
  const { name, typeBadge, description } = extractMeta(md, solverId)
  resetFirstTable()
  const processedMd = await preHighlightCode(md)
  const bodyHtml = marked.parse(processedMd, { breaks: false, gfm: true, html: true })
  const hasExplainer = existsSync(join(WEB_ROOT, `algorithms/${solverId}/explainer/index.html`))

  const outDir = join(WEB_ROOT, `algorithms/${solverId}`)
  mkdirSync(outDir, { recursive: true })
  const html = eta.render('algorithm-page', { solverId, name, typeBadge, description, bodyHtml, hasExplainer })
  writeFileSync(join(outDir, 'index.html'), html)
  console.log(`[build-docs] docs/algorithms/${filename} → algorithms/${solverId}/index.html`)
}

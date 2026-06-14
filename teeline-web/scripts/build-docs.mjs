#!/usr/bin/env node
// Generates algorithms/<id>/index.html from algorithms/<id>/docs.md at build time.
// Run before vite build so Vite finds all MPA entry points.
import { readFileSync, writeFileSync, mkdirSync } from 'fs'
import { fileURLToPath } from 'url'
import { dirname, join } from 'path'
import { globSync } from 'tinyglobby'
import { Marked } from 'marked'
import { highlightText } from '@speed-highlight/core'

const __dirname = dirname(fileURLToPath(import.meta.url))
const WEB_ROOT = join(__dirname, '..')

const HIGHLIGHT_CSS = readFileSync(
  join(WEB_ROOT, 'node_modules/@speed-highlight/core/dist/themes/github-light.css'),
  'utf8'
)

// Simple key: "value" frontmatter parser — no YAML library needed
function parseFrontmatter(src) {
  if (!src.startsWith('---\n')) return { meta: {}, body: src }
  const end = src.indexOf('\n---\n', 4)
  if (end === -1) return { meta: {}, body: src }
  const meta = {}
  for (const line of src.slice(4, end).split('\n')) {
    const colon = line.indexOf(':')
    if (colon === -1) continue
    const key = line.slice(0, colon).trim()
    const raw = line.slice(colon + 1).trim().replace(/^["']|["']$/g, '')
    meta[key] = raw === 'true' ? true : raw === 'false' ? false : raw
  }
  return { meta, body: src.slice(end + 5).trim() }
}

function escAttr(s) {
  return String(s ?? '').replace(/&/g, '&amp;').replace(/"/g, '&quot;')
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

  // Replace each fence with its rendered HTML (iterate in reverse to preserve indices)
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
    resetFirstTable() {
      isFirstTable = true
    },
  }
}

function renderShell({ meta, bodyHtml }) {
  const title = meta.title ?? `${meta.name} — Teeline`
  const explainerCta =
    meta.explainer === true
      ? `<a class="docs-explainer-cta" href="/algorithms/${meta.solver_id}/explainer/">▶ Open interactive explainer →</a>`
      : ''
  const typeBadge = meta.type_badge
    ? `<p class="docs-type-badge">${meta.type_badge}</p>`
    : ''

  return `<!doctype html>
<html lang="en" data-theme="light">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>${escAttr(title)}</title>
    <meta name="description" content="${escAttr(meta.description ?? '')}" />
    <style>${HIGHLIGHT_CSS}</style>
  </head>
  <body>
    <div id="topbar"></div>
    <div class="docs-layout">
      <aside class="docs-sidebar">
        <nav id="algo-sidebar" aria-label="Algorithms"></nav>
      </aside>
      <main class="docs-main">

        <nav aria-label="breadcrumb">
          <ul>
            <li><a href="/">teeline</a></li>
            <li>Algorithms</li>
            <li>${meta.name ?? meta.solver_id}</li>
          </ul>
        </nav>

        ${typeBadge}
        ${explainerCta}
        ${bodyHtml}
      </main>
    </div>
    <script type="module" src="/src/docs-init.ts"></script>
  </body>
</html>`
}

const { marked, resetFirstTable } = makeMarked()
const docFiles = globSync('algorithms/*/docs.md', { cwd: WEB_ROOT })

if (docFiles.length === 0) {
  console.warn('[build-docs] no algorithms/*/docs.md files found')
  process.exit(0)
}

for (const relPath of docFiles) {
  const src = readFileSync(join(WEB_ROOT, relPath), 'utf8')
  const { meta, body } = parseFrontmatter(src)
  resetFirstTable()
  const processedBody = await preHighlightCode(body)
  const bodyHtml = marked.parse(processedBody, { breaks: false, gfm: true, html: true })
  const html = renderShell({ meta, bodyHtml })
  const outPath = relPath.replace('docs.md', 'index.html')
  writeFileSync(join(WEB_ROOT, outPath), html)
  console.log(`[build-docs] ${relPath} → ${outPath}`)
}

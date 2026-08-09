#!/usr/bin/env node
// Generate docs/problems/*.md from data/tsplib/*.tsp files.
// Extracts TSPLIB header, parses coordinates for SVG minimap,
// looks up optimal cost from .opt.tour files.
// Run: node scripts/gen-problem-pages.mjs

import { readFileSync, readdirSync, writeFileSync, mkdirSync } from 'node:fs'
import { join, basename } from 'node:path'

const TSP_DIR = join(import.meta.dirname, '..', 'data', 'tsplib')
const DOCS_DIR = join(import.meta.dirname, '..', 'docs', 'problems')
const DATA_BASE = 'https://static.tspsolver.com/tsplib'
const SOURCE = 'TSPLIB95'
const SOURCE_URL = 'https://comopt.ifi.uni-heidelberg.de/software/TSPLIB95/'

mkdirSync(DOCS_DIR, { recursive: true })

const TSP_FILES = readdirSync(TSP_DIR).filter(f => f.endsWith('.tsp'))

// ── helpers ──────────────────────────────────────────────────────────

function sizeGroup(cities) {
  if (cities <= 100) return 'small'
  if (cities <= 500) return 'medium'
  if (cities <= 2000) return 'large'
  return 'xl'
}

function iconForSize(group) {
  return { small: '🟢', medium: '🟡', large: '🟠', xl: '🔴' }[group] || ''
}

function parseHeader(txt) {
  const fields = {}
  const re = /^([A-Z_ ]+?)\s*:\s*(.+)$/gm
  let m
  while ((m = re.exec(txt)) !== null) {
    fields[m[1].trim()] = m[2].trim()
  }
  return fields
}

// Extract optimal cost from .opt.tour COMMENT field
function getOptimalCost(name) {
  try {
    const path = join(TSP_DIR, `${name}.opt.tour`)
    const txt = readFileSync(path, 'utf8')
    const m = txt.match(/COMMENT\s*:\s*.*?\((\d+)\)/)
    if (m) return parseInt(m[1])
    // Alternative: single number at end of COMMENT
    const m2 = txt.match(/COMMENT\s*:\s*.*?(\d+)\s*$/)
    if (m2) return parseInt(m2[1])
  } catch { return null }
  return null
}

// Parse NODE_COORD_SECTION for (x,y) pairs
function parseCoords(txt) {
  const sectionMatch = txt.match(/NODE_COORD_SECTION\s*\n([\s\S]*?)(?:EOF|\n\s*\n|$)/i)
  if (!sectionMatch) return null
  const lines = sectionMatch[1].trim().split('\n').filter(l => l.trim())
  const pts = []
  for (const line of lines) {
    const parts = line.trim().split(/\s+/)
    // Format: index x y
    if (parts.length >= 3) {
      const x = parseFloat(parts[1])
      const y = parseFloat(parts[2])
      if (!isNaN(x) && !isNaN(y)) pts.push({ x, y })
    }
  }
  return pts.length > 0 ? pts : null
}

// Generate inline SVG minimap
function generateMinimap(coords) {
  if (!coords || coords.length === 0) return ''

  const xs = coords.map(p => p.x)
  const ys = coords.map(p => p.y)
  const minX = Math.min(...xs), maxX = Math.max(...xs)
  const minY = Math.min(...ys), maxY = Math.max(...ys)
  const w = (maxX - minX) || 1
  const h = (maxY - minY) || 1

  const PAD = 20
  const VIEW_W = 340, VIEW_H = 200
  const AREA_W = VIEW_W - PAD * 2, AREA_H = VIEW_H - PAD * 2

  const scale = Math.min(AREA_W / w, AREA_H / h)
  const offX = (VIEW_W - w * scale) / 2
  const offY = (VIEW_H - h * scale) / 2

  function sx(x) { return ((x - minX) * scale + offX).toFixed(1) }
  function sy(y) { return ((y - minY) * scale + offY).toFixed(1) }

  // For large datasets, use smaller dots
  const r = coords.length > 2000 ? 0.8 : coords.length > 500 ? 1.2 : 2

  const dots = coords.map(p =>
    `<circle cx="${sx(p.x)}" cy="${sy(p.y)}" r="${r}" fill="#0d9488" opacity="0.7"/>`
  ).join('\n    ')

  return `<svg viewBox="0 0 ${VIEW_W} ${VIEW_H}" width="100%" style="max-width:${VIEW_W}px;border-radius:8px;border:1px solid #d0d7de;background:#f6f8fa;" role="img" aria-label="City map for this problem">
  ${dots}
</svg>`
}

// ── main ─────────────────────────────────────────────────────────────

let count = 0
const problemIds = []

for (const file of TSP_FILES.sort()) {
  const name = basename(file, '.tsp')
  const txt = readFileSync(join(TSP_DIR, file), 'utf8')

  const header = parseHeader(txt)

  const id = name
  const title = header['COMMENT'] || header['NAME'] || name
  const cities = parseInt(header['DIMENSION']) || 0
  const ewt = header['EDGE_WEIGHT_TYPE'] || 'UNKNOWN'
  const group = sizeGroup(cities)
  const optimalCost = getOptimalCost(name)

  const hasCoords = !['EXPLICIT', 'MATRIX'].some(t => ewt.toUpperCase().includes(t))
  const coords = hasCoords ? parseCoords(txt) : null
  const minimap = generateMinimap(coords)

  const dataUrl = `${DATA_BASE}/${name}.tsp`

  const optLine = optimalCost !== null
    ? `| Optimal tour | ${optimalCost.toLocaleString()} |`
    : '| Optimal tour | Unknown |'

  const icon = iconForSize(group)

  const solverLink = optimalCost !== null
    ? `/?dataset=${name}&opt=${optimalCost}`
    : `/?dataset=${name}`

  const content = `---
id: "${id}"
name: "${title}"
description: "${title}"
cities: ${cities}
sizeGroup: "${group}"
edgeWeightType: "${ewt}"
optimalCost: ${optimalCost !== null ? optimalCost : 'null'}
dataUrl: "${dataUrl}"
source: "${SOURCE}"
sourceUrl: "${SOURCE_URL}"
---

# ${title}

${minimap ? `${minimap}\n\n` : ''}${title}

| Field | Value |
| ----- | ----- |
| Name | \`${id}\` |
| Cities | ${cities} |
| Type | \`${ewt}\` |
| Size group | ${icon} ${group} |
${optLine}
| Source | [${SOURCE}](${SOURCE_URL}) |

[Download dataset](${dataUrl}){.btn-primary}

[Open in Solver →](${solverLink}){.btn-accent}
`

  writeFileSync(join(DOCS_DIR, `${id}.md`), content)
  problemIds.push({ id, group, cities })
  count++
}

console.log(`Generated ${count} problem pages in ${DOCS_DIR}`)

// Print size group summary
for (const g of ['small', 'medium', 'large', 'xl']) {
  const ids = problemIds.filter(p => p.group === g).map(p => p.id)
  console.log(`  ${g} (${ids.length}): ${ids.join(', ')}`)
}

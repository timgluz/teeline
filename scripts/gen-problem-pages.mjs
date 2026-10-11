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
// Dataset structured data (schema.org) needs an explicit creator and license.
const CREATOR = 'TSPLIB — Gerhard Reinelt, University of Heidelberg'
const LICENSE = SOURCE_URL

mkdirSync(DOCS_DIR, { recursive: true })

const TSP_FILES = readdirSync(TSP_DIR).filter(f => f.endsWith('.tsp'))

// Optimal costs with their provenance, in a committed file rather than inlined so every value
// carries how it was established — the schema treats `optimal_cost` as a claim of proof, and a
// value with no evidence recorded is not one. Covers instances TSPLIB ships no optimal tour for,
// and the one whose shipped tour disagrees with the published optimum.
const LOCALLY_PROVEN = (() => {
  try {
    return JSON.parse(readFileSync(join(import.meta.dirname, '..', 'bench', 'optima.json'), 'utf8')).optima || {}
  } catch {
    return {}
  }
})()

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

// TSPLIB ships the optimal *route* for many instances, and states its cost in the COMMENT only
// sometimes. The forms vary — `(6528)`, `Length 6528`, `(Length 629)` — and some files carry no
// COMMENT at all. Where the cost is missing it is still determined: the shipped route is optimal,
// so its length under the instance's own distance definition *is* the optimum.
function getOptimalCost(name) {
  // Consulted first: a published optimum defines the word, while a length derived here is only a
  // proxy for it, and the two demonstrably disagree when the local metric differs. tsp225's shipped
  // .opt.tour states 3919 — the unrounded score of its route — while the published optimum is 3916.
  const known = LOCALLY_PROVEN[name]
  if (known) return known.cost

  let txt
  try {
    txt = readFileSync(join(TSP_DIR, `${name}.opt.tour`), 'utf8')
  } catch {
    return null
  }
  // The old fallback omitted the `m` flag, so a cost on any line but the last never matched and
  // `Length 6528` read as unknown.
  for (const re of [
    /COMMENT\s*:\s*[^\n]*?\(Length\s+(\d+)\)/,
    /COMMENT\s*:\s*[^\n]*?\((\d+)\)/,
    /COMMENT\s*:\s*[^\n]*?Length\s+(\d+)/,
  ]) {
    const m = txt.match(re)
    if (m) return parseInt(m[1], 10)
  }
  return tourLength(name, txt)
}

const nint = v => Math.floor(v + 0.5)

// TSPLIB ATT: r = sqrt((dx^2 + dy^2) / 10), then nint(r), plus one when that rounding went down.
function attDistance(a, b) {
  const r = Math.sqrt(((a.x - b.x) ** 2 + (a.y - b.y) ** 2) / 10)
  const t = nint(r)
  return t < r ? t + 1 : t
}

// TSPLIB CEIL_2D: Euclidean distance rounded up.
function ceilDistance(a, b) {
  return Math.ceil(Math.sqrt((a.x - b.x) ** 2 + (a.y - b.y) ** 2))
}

// TSPLIB GEO: coordinates are (degrees, minutes) pairs, and the result is truncated, not rounded.
function geoDistance(a, b) {
  const toRad = v => {
    const deg = Math.trunc(v)
    return (Math.PI * (deg + (5 * (v - deg)) / 3)) / 180
  }
  const [latA, lonA, latB, lonB] = [toRad(a.x), toRad(a.y), toRad(b.x), toRad(b.y)]
  const q1 = Math.cos(lonA - lonB)
  const q2 = Math.cos(latA - latB)
  const q3 = Math.cos(latA + latB)
  return Math.floor(6378.388 * Math.acos(0.5 * ((1 + q1) * q2 - (1 - q1) * q3)) + 1)
}

// Score TSPLIB's optimal route under the instance's own distance definition.
//
// Restricted to the types whose definition can be reproduced exactly here. EUC_2D is deliberately
// excluded: TSPLIB rounds it with nint() and teeline does not, so a length computed under our
// metric would be neither TSPLIB's optimum nor the optimum of the metric teeline actually
// optimises — and publishing it as `optimalCost` would be a claim nothing supports. See
// tasks/fix-euc-2d-rounding.
function tourLength(name, tourTxt) {
  let tsp
  try {
    tsp = readFileSync(join(TSP_DIR, `${name}.tsp`), 'utf8')
  } catch {
    return null
  }
  const type = (parseHeader(tsp)['EDGE_WEIGHT_TYPE'] || '').toUpperCase()
  const byType = { ATT: attDistance, CEIL_2D: ceilDistance, GEO: geoDistance }
  const distance = byType[type]
  if (!distance) return null

  const coords = new Map()
  const section = tsp.match(/NODE_COORD_SECTION\s*([\s\S]*?)(?:EOF|$)/)
  if (!section) return null
  for (const line of section[1].split('\n')) {
    const parts = line.trim().split(/\s+/)
    if (parts.length >= 3) {
      const [id, x, y] = [parseInt(parts[0], 10), parseFloat(parts[1]), parseFloat(parts[2])]
      if (Number.isInteger(id) && !isNaN(x) && !isNaN(y)) coords.set(id, { x, y })
    }
  }

  const route = (tourTxt.match(/TOUR_SECTION\s*([\s\S]*?)(?:-1|EOF)/)?.[1] || '')
    .split('\n')
    .map(s => parseInt(s.trim(), 10))
    .filter(Number.isInteger)
  if (route.length < 2 || route.some(id => !coords.has(id))) return null

  let total = 0
  for (let i = 0; i < route.length; i++) {
    total += distance(coords.get(route[i]), coords.get(route[(i + 1) % route.length]))
  }
  return Math.round(total)
}

// Read a section body as whitespace-split rows. The section ends at the next keyword
// line or EOF, not at a blank line: `swiss42` pads its headers with trailing spaces,
// and a coordinate-less instance's `DISPLAY_DATA_SECTION` is not always followed by
// `EOF`.
function readSection(txt, name) {
  const lines = txt.split(/\r?\n/)
  const header = new RegExp(`^${name}\\s*$`, 'i')
  const start = lines.findIndex(l => header.test(l.trim()))
  if (start < 0) return null

  const rows = []
  for (let i = start + 1; i < lines.length; i++) {
    const line = lines[i].trim()
    if (!line) continue
    if (/^EOF$/i.test(line) || /^[A-Z_]+\s*(:|$)/.test(line)) break
    rows.push(line.split(/\s+/))
  }
  return rows.length > 0 ? rows : null
}

// Minimap points, as `index x y` rows. NODE_COORD_SECTION holds the real coordinates;
// DISPLAY_DATA_SECTION holds TSPLIB's own 2-D projection for instances whose distances
// come from a matrix, which is why the five EXPLICIT instances that ship one can be
// drawn without deriving anything.
function parsePoints(txt) {
  for (const section of ['NODE_COORD_SECTION', 'DISPLAY_DATA_SECTION']) {
    const rows = readSection(txt, section)
    if (!rows) continue
    const pts = []
    for (const parts of rows) {
      if (parts.length < 3) continue
      const x = parseFloat(parts[1])
      const y = parseFloat(parts[2])
      if (!isNaN(x) && !isNaN(y)) pts.push({ x, y })
    }
    if (pts.length > 0) return pts
  }
  return null
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
  const comment = header['COMMENT'] || ''
  const title = comment || header['NAME'] || name
  const cleanTitle = title.replace(/[.,;:!?]+$/, '')
  // Derive a short display name from the TSPLIB NAME (e.g. "bayg29" → "Bayg 29")
  const shortName = name.replace(/([a-z])(\d)/gi, '$1 $2').replace(/([A-Z])([A-Z][a-z])/g, '$1 $2')
    .split(' ').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ')
  const cities = parseInt(header['DIMENSION']) || 0
  const ewt = header['EDGE_WEIGHT_TYPE'] || 'UNKNOWN'
  const group = sizeGroup(cities)
  const optimalCost = getOptimalCost(name)

  const points = parsePoints(txt)
  const minimap = generateMinimap(points)

  const dataUrl = `${DATA_BASE}/${name}.tsp`

  const optLine = optimalCost !== null
    ? `| Optimal tour | ${optimalCost.toLocaleString()} |`
    : '| Optimal tour | Unknown |'

  const icon = iconForSize(group)

  const isExplicit = ['EXPLICIT', 'MATRIX', 'GEO'].some(t => ewt.toUpperCase().includes(t))
  const solverOptimalCost = isExplicit ? null : optimalCost

  const solverLink = solverOptimalCost !== null
    ? `/?dataset=${name}&opt=${solverOptimalCost}`
    : `/?dataset=${name}`

  const content = `---
id: "${id}"
name: "${shortName}"
description: "${title}"
cities: ${cities}
sizeGroup: "${group}"
edgeWeightType: "${ewt}"
optimalCost: ${optimalCost !== null ? optimalCost : 'null'}
dataUrl: "${dataUrl}"
source: "${SOURCE}"
sourceUrl: "${SOURCE_URL}"
creator: "${CREATOR}"
license: "${LICENSE}"
---

${minimap}

${title}
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

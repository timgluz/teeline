#!/usr/bin/env node
// Generate docs/problems/*.md from data/tsplib/*.tsp files.
// Extracts the TSPLIB header, the optimal cost from .opt.tour files, and the URL of the instance's
// minimap — the image itself is generated separately (scripts/gen-problem-minimaps.mjs) and
// published to R2, so a regeneration here is a one-line diff per page.
// Run: node scripts/gen-problem-pages.mjs

import { readFileSync, readdirSync, writeFileSync, mkdirSync } from 'node:fs'
import { join, basename } from 'node:path'
import { minimapPoints, parseHeader } from './lib/problem-minimap.mjs'
import { tourLength } from './lib/problem-tour.mjs'

const TSP_DIR = join(import.meta.dirname, '..', 'data', 'tsplib')
const DOCS_DIR = join(import.meta.dirname, '..', 'docs', 'problems')
const DATA_BASE = 'https://static.tspsolver.com/tsplib'
// Deliberately outside the `tsplib/` dataset prefix — the images and the instances they draw
// should not share a listing — and versioned so that changing how a map is drawn is a prefix bump
// rather than a cache purge. See scripts/publish-minimaps.sh.
const MINIMAP_BASE = 'https://static.tspsolver.com/minimaps/v1'
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
  // No stated cost: score the route under the instance's own metric. The .tsp is read here so the
  // scorer stays pure and testable.
  let tspText
  try {
    tspText = readFileSync(join(TSP_DIR, `${name}.tsp`), 'utf8')
  } catch {
    return null
  }
  return tourLength(tspText, txt)
}

// ── main ─────────────────────────────────────────────────────────────

// Which map each instance has, resolved before anything is written.
//
// The two generators have to agree: a page linking an image the other one refused to draw would
// 404, and a corpus gap should fail the run rather than leave the tree half-regenerated. This is
// also the only reason the layout is computed here at all — the pages need `derived` and nothing
// else, and it costs a second on `si1032`.
const minimaps = new Map()
const undrawable = []
for (const file of TSP_FILES) {
  const id = basename(file, '.tsp')
  const txt = readFileSync(join(TSP_DIR, file), 'utf8')
  const header = parseHeader(txt)
  const dim = parseInt(header['DIMENSION'], 10) || 0
  const format = (header['EDGE_WEIGHT_FORMAT'] || '').trim().toUpperCase()

  const { points, derived } = minimapPoints(txt, dim, format)
  if (!points) {
    undrawable.push(`${id} has no coordinates and no parseable ${header['EDGE_WEIGHT_TYPE'] || '?'} matrix`)
    continue
  }
  // Both URLs assume the map has been published; gen-problem-minimaps.mjs renders the SVG and
  // teeline-web/scripts/gen-og-images.mjs the social card (see scripts/publish-minimaps.sh).
  minimaps.set(id, {
    url: `${MINIMAP_BASE}/${id}.svg`,
    ogUrl: `${MINIMAP_BASE}/og/${id}.png`,
    derived,
  })
}

if (undrawable.length > 0) {
  for (const item of undrawable) console.error(`Cannot draw ${item}`)
  process.exit(1)
}

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

  const map = minimaps.get(id)

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
minimapUrl: "${map.url}"
ogImageUrl: "${map.ogUrl}"
minimapDerived: ${map.derived}
source: "${SOURCE}"
sourceUrl: "${SOURCE_URL}"
creator: "${CREATOR}"
license: "${LICENSE}"
---
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

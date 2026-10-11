#!/usr/bin/env node
// Render one minimap SVG per problem instance into a staging directory, for upload to R2.
//
// Run: node scripts/gen-problem-minimaps.mjs [--out build/minimaps]
//
// The pages reference these by URL, so nothing is written into docs/problems/ — that is the point
// of the split. `index.json` records each file's hash so a re-publish can upload only what changed.

import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { basename, join } from 'node:path'
import { minimapPoints, parseHeader, renderMinimapSvg } from './lib/problem-minimap.mjs'

const TSP_DIR = join(import.meta.dirname, '..', 'data', 'tsplib')

const args = process.argv.slice(2)
const outIndex = args.indexOf('--out')
const OUT_DIR = outIndex >= 0 ? args[outIndex + 1] : join(import.meta.dirname, '..', 'build', 'minimaps')
if (outIndex >= 0 && !args[outIndex + 1]) {
  console.error('--out needs a directory')
  process.exit(2)
}

mkdirSync(OUT_DIR, { recursive: true })

const files = readdirSync(TSP_DIR)
  .filter((f) => f.endsWith('.tsp'))
  .sort()

const entries = {}
const failures = []
let totalBytes = 0

for (const file of files) {
  const id = basename(file, '.tsp')
  const txt = readFileSync(join(TSP_DIR, file), 'utf8')
  const header = parseHeader(txt)
  const dim = parseInt(header['DIMENSION'], 10) || 0
  const format = (header['EDGE_WEIGHT_FORMAT'] || '').trim().toUpperCase()

  const { points, derived } = minimapPoints(txt, dim, format)
  if (!points) {
    // Fail loudly: a missing map is a gap in the corpus, and a silent skip is how the 17 EXPLICIT
    // pages stayed empty. Nothing is written unless every instance renders.
    failures.push(`${id} has no coordinates and no parseable ${header['EDGE_WEIGHT_TYPE'] || '?'} matrix${format ? ` (${format})` : ''}`)
    continue
  }

  const svg = renderMinimapSvg(points, derived)
  writeFileSync(join(OUT_DIR, `${id}.svg`), svg)

  const bytes = Buffer.byteLength(svg)
  totalBytes += bytes
  entries[id] = {
    sha256: createHash('sha256').update(svg).digest('hex'),
    bytes,
    derived,
    cities: dim,
  }
}

if (failures.length > 0) {
  for (const failure of failures) console.error(`Cannot draw ${failure}`)
  process.exit(1)
}

const indexPath = join(OUT_DIR, 'index.json')
writeFileSync(
  indexPath,
  JSON.stringify({ generated_at: new Date().toISOString(), entries }, null, 2) + '\n'
)

const derived = Object.values(entries).filter((e) => e.derived).length
const largest = Object.entries(entries).sort((a, b) => b[1].bytes - a[1].bytes)[0]
console.log(`Generated ${Object.keys(entries).length} minimaps in ${OUT_DIR}`)
console.log(`  ${(totalBytes / 1e6).toFixed(2)} MB total, largest ${largest[0]} at ${(largest[1].bytes / 1024).toFixed(0)} KB`)
console.log(`  ${derived} derived from a distance matrix, ${Object.keys(entries).length - derived} from published points`)
console.log(`  index: ${indexPath}`)

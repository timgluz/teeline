#!/usr/bin/env node
// Render an Open Graph card for every staged problem minimap.
//
// Run: node scripts/gen-og-images.mjs      (from teeline-web/)
//
// Reads  ../build/minimaps/{id}.svg + index.json (see scripts/gen-problem-minimaps.mjs)
// Writes ../build/minimaps/og/{id}.png and records each hash in the index
//
// Social clients and structured-data consumers do not accept SVG, so the map has to exist as a
// raster to be shareable at all. It is the same picture: the SVG is inlined into a 1200x630 page
// and screenshotted, which is why this lives in teeline-web (Playwright is a dev dependency here)
// rather than in the root scripts, which have none.
//
// Needs the browser once: npx playwright install chromium

import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

// `@playwright/test` is what package.json declares and it re-exports chromium; importing the
// bare `playwright` package worked only because npm hoists it as a transitive dependency.
import { chromium } from '@playwright/test'

const WIDTH = 1200
const HEIGHT = 630
// The map's own ratio is 340:200, so it fills the card's height with the width that follows —
// 590 * 1.7 = 1003, leaving a 98px margin either side rather than letterboxing a stretched image.
const MAP_HEIGHT = 590

const minimapsDir = fileURLToPath(
  new URL('../../build/minimaps', import.meta.url),
)
const indexPath = join(minimapsDir, 'index.json')
const outDir = join(minimapsDir, 'og')

const index = JSON.parse(readFileSync(indexPath, 'utf8'))
const ids = Object.keys(index.entries).sort()
if (ids.length === 0) {
  console.error(
    `${indexPath} lists no instances — run scripts/gen-problem-minimaps.mjs first.`,
  )
  process.exit(1)
}

mkdirSync(outDir, { recursive: true })

const browser = await chromium.launch()

let rendered = 0
let totalBytes = 0
try {
  const page = await browser.newPage({
    viewport: { width: WIDTH, height: HEIGHT },
    deviceScaleFactor: 1,
  })

  for (const id of ids) {
    // A staged index whose SVG is gone is a half-generated tree; naming the file is what makes it
    // fixable, and the finally below is what keeps a Chromium from surviving the throw.
    const svgPath = join(minimapsDir, `${id}.svg`)
    let svg
    try {
      svg = readFileSync(svgPath, 'utf8')
    } catch (err) {
      // EACCES or EISDIR are not "missing", and losing the code turns a fixable permissions problem
      // into a misleading instruction.
      throw new Error(
        `${svgPath} could not be read (${err.code ?? 'unknown'}) — re-run scripts/gen-problem-minimaps.mjs if it is missing.`,
        { cause: err },
      )
    }
    // Inline the SVG rather than pointing at the file: an about:blank page loading file:// resources
    // is a browser-policy question, and inlining removes it entirely.
    await page.setContent(
      `<!doctype html><html><body style="margin:0;width:${WIDTH}px;height:${HEIGHT}px;background:#f6f8fa;display:flex;align-items:center;justify-content:center">
<style>svg{height:${MAP_HEIGHT}px;width:auto;display:block}</style>
${svg}
</body></html>`,
      { waitUntil: 'load' },
    )

    const png = await page.screenshot({
      type: 'png',
      clip: { x: 0, y: 0, width: WIDTH, height: HEIGHT },
    })
    writeFileSync(join(outDir, `${id}.png`), png)

    index.entries[id].og = {
      sha256: createHash('sha256').update(png).digest('hex'),
      bytes: png.length,
    }
    rendered++
    totalBytes += png.length
  }
} finally {
  // The index is only rewritten after every card renders, so a failure here leaves the previous
  // index intact and the next run re-renders — but never leaves a browser behind.
  await browser.close()
}

// The index is the publisher's manifest, so the PNGs are recorded in it rather than in a second
// file the publisher would have to know about.
writeFileSync(indexPath, JSON.stringify(index, null, 2) + '\n')

const largest = Object.entries(index.entries).sort(
  (a, b) => b[1].og.bytes - a[1].og.bytes,
)[0]
console.log(`Rendered ${rendered} OG cards in ${outDir}`)
console.log(
  `  ${(totalBytes / 1e6).toFixed(2)} MB total, largest ${largest[0]} at ${(largest[1].og.bytes / 1024).toFixed(0)} KB`,
)
console.log(`  recorded in ${indexPath}`)

import { copyFileSync, existsSync } from 'fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

// @astrojs/sitemap always emits a sitemap *index* at /sitemap-index.xml that
// points at one or more /sitemap-<n>.xml chunks — there is no option to name
// the index `sitemap.xml`.
//
// /sitemap.xml is the path crawlers, Google Search Console and third-party SEO
// tools assume by default. Without it the request falls through to Cloudflare
// Pages' SPA fallback (no top-level 404.html), which answers with the homepage
// as HTTP 200 HTML — so a submitted /sitemap.xml looks like a valid page that
// contains no sitemap, and no URLs get discovered.
//
// Mirror the generated index to /sitemap.xml so the conventional URL serves a
// real sitemap document. Copying stays correct no matter how many chunks the
// integration emits, because the index only lists `sitemap-<n>.xml` URLs.

/**
 * Copy `<distDir>/sitemap-index.xml` to `<distDir>/sitemap.xml`.
 *
 * @param {string} [distDir] Astro build output directory (default: `dist`).
 * @returns {string} absolute path of the written `/sitemap.xml` alias.
 */
export function aliasSitemap(distDir = 'dist') {
  const indexPath = resolve(distDir, 'sitemap-index.xml')
  const aliasPath = resolve(distDir, 'sitemap.xml')

  if (!existsSync(indexPath)) {
    throw new Error(`${indexPath} not found — did astro build run?`)
  }

  copyFileSync(indexPath, aliasPath)
  return aliasPath
}

// CLI entry point: `node scripts/sitemap-alias.mjs` (run by `npm run build`).
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    console.log(`[sitemap-alias] dist/sitemap-index.xml → ${aliasSitemap()}`)
  } catch (err) {
    console.error(`[sitemap-alias] ERROR: ${err.message}`)
    process.exit(1)
  }
}

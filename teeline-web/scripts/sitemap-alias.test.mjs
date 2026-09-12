import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { aliasSitemap } from './sitemap-alias.mjs'

const INDEX_XML =
  '<?xml version="1.0" encoding="UTF-8"?>' +
  '<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">' +
  '<sitemap><loc>https://tspsolver.com/sitemap-0.xml</loc></sitemap>' +
  '</sitemapindex>'

describe('aliasSitemap', () => {
  let dist

  beforeEach(() => {
    dist = mkdtempSync(join(tmpdir(), 'sitemap-alias-'))
  })

  afterEach(() => {
    rmSync(dist, { recursive: true, force: true })
  })

  it('copies the generated sitemap index to /sitemap.xml', () => {
    writeFileSync(join(dist, 'sitemap-index.xml'), INDEX_XML)

    const aliasPath = aliasSitemap(dist)

    expect(aliasPath).toBe(join(dist, 'sitemap.xml'))
    expect(readFileSync(aliasPath, 'utf8')).toBe(INDEX_XML)
  })

  it('throws when the build output has no sitemap index', () => {
    expect(() => aliasSitemap(dist)).toThrow(/sitemap-index\.xml not found/)
  })
})

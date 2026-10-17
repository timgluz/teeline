import { existsSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { expect, test, type Page } from '@playwright/test'

// The minimaps are R2 objects, not inline SVG, so the contract this file pins is the one that can
// silently break: the URL shape the page emits has to be the one scripts/publish-minimaps.sh
// writes, and a derived layout has to keep saying that it is derived.
const MINIMAP_BASE = 'https://static.tspsolver.com/minimaps/v1'

const stagedSvg = (id: string): string | null => {
  const path = fileURLToPath(
    new URL(`../../build/minimaps/${id}.svg`, import.meta.url),
  )
  return existsSync(path) ? readFileSync(path, 'utf8') : null
}

/** Serve the map from the staging dir when it exists, so the image actually renders. */
async function stubMinimaps(page: Page): Promise<void> {
  await page.route(`${MINIMAP_BASE}/*.svg`, async (route) => {
    const id = route.request().url().split('/').pop()!.replace('.svg', '')
    // The fallback is a faithful stand-in for a published map: an SVG with no width/height has no
    // intrinsic size, and the rendered check below would then see a default (255) rather than the
    // 340x200 the page asks for. This suite has to pass in a checkout with no staging dir, which is
    // exactly what CI has.
    const svg =
      stagedSvg(id) ??
      '<svg xmlns="http://www.w3.org/2000/svg" width="340" height="200" viewBox="0 0 340 200"><circle cx="10" cy="10" r="2"/></svg>'
    await route.fulfill({ body: svg, contentType: 'image/svg+xml' })
  })
}

test.describe('problem minimaps', () => {
  test('a published-coordinate instance links its map as a lazy image', async ({
    page,
  }) => {
    await stubMinimaps(page)
    await page.goto('/problems/berlin52/')

    const map = page.locator('main figure img')
    await expect(map).toHaveAttribute('src', `${MINIMAP_BASE}/berlin52.svg`)
    await expect(map).toHaveAttribute('width', '340')
    await expect(map).toHaveAttribute('height', '200')
    await expect(map).toHaveAttribute('loading', 'lazy')
    await expect(map).toHaveAttribute('alt', 'Map of Berlin 52')

    // Rendered, not merely referenced: the served document is a real 340x200 image.
    await expect
      .poll(async () =>
        map.evaluate((el) => (el as HTMLImageElement).naturalWidth),
      )
      .toBe(340)

    // Berlin publishes coordinates, so nothing on the page claims a derived layout.
    await expect(page.locator('main figure figcaption')).toHaveCount(0)
  })

  test('a matrix-only instance says its layout is derived', async ({
    page,
  }) => {
    await stubMinimaps(page)
    await page.goto('/problems/gr48/')

    const map = page.locator('main figure img')
    await expect(map).toHaveAttribute('src', `${MINIMAP_BASE}/gr48.svg`)
    await expect(map).toHaveAttribute('alt', /derived from its distance matrix/)
    await expect(page.locator('main figure figcaption')).toContainText(
      'Layout derived from the distance matrix',
    )
  })

  test('structured data points at the raster, not the SVG', async ({
    page,
  }) => {
    await page.goto('/problems/berlin52/')

    // Social clients and structured-data consumers reject SVG, so the card is what they get.
    const jsonLd = await page
      .locator('script[type="application/ld+json"]')
      .textContent()
    expect(JSON.parse(jsonLd ?? '{}').image).toBe(
      `${MINIMAP_BASE}/og/berlin52.png`,
    )
  })

  test('crawlers are given the card, sized and with an alt', async ({
    page,
  }) => {
    await page.goto('/problems/berlin52/')

    const card = `${MINIMAP_BASE}/og/berlin52.png`
    await expect(page.locator('meta[property="og:image"]')).toHaveAttribute(
      'content',
      card,
    )
    await expect(
      page.locator('meta[property="og:image:width"]'),
    ).toHaveAttribute('content', '1200')
    await expect(
      page.locator('meta[property="og:image:height"]'),
    ).toHaveAttribute('content', '630')
    await expect(page.locator('meta[property="og:image:alt"]')).toHaveAttribute(
      'content',
      'Map of Berlin 52',
    )
    await expect(page.locator('meta[name="twitter:card"]')).toHaveAttribute(
      'content',
      'summary_large_image',
    )
    await expect(page.locator('meta[name="twitter:image"]')).toHaveAttribute(
      'content',
      card,
    )
  })

  test('pages without a card keep the site default', async ({ page }) => {
    await page.goto('/problems/')

    await expect(page.locator('meta[property="og:image"]')).toHaveAttribute(
      'content',
      'https://tspsolver.com/og-image.png',
    )
    await expect(page.locator('meta[name="twitter:card"]')).toHaveAttribute(
      'content',
      'summary',
    )
  })
})

import { existsSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { expect, test, type Page } from '@playwright/test'

// The minimaps are R2 objects, not inline SVG, so the contract this file pins is the one that can
// silently break: the URL shape the page emits has to be the one scripts/publish-minimaps.sh
// writes, and a derived layout has to keep saying that it is derived.
const MINIMAP_BASE = 'https://static.tspsolver.com/tsplib/minimaps/v1'

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
    const svg =
      stagedSvg(id) ??
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 340 200"><circle cx="10" cy="10" r="2"/></svg>'
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

  test('the Dataset JSON-LD advertises the map', async ({ page }) => {
    await page.goto('/problems/berlin52/')

    const jsonLd = await page
      .locator('script[type="application/ld+json"]')
      .textContent()
    expect(JSON.parse(jsonLd ?? '{}').image).toBe(
      `${MINIMAP_BASE}/berlin52.svg`,
    )
  })
})

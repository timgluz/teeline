import { test, expect } from '@playwright/test'

// Regression guard for the Cloudflare Pages SPA fallback: with no top-level
// 404.html, Pages matches every unmatched path to `/` and answers with the
// homepage as HTTP 200 — which is how /sitemap.xml used to come back as HTML
// (so Search Console read a "valid" page that contained no sitemap).
test.describe('not-found handling', () => {
  test('unknown paths return the 404 page, not the homepage', async ({
    page,
  }) => {
    const response = await page.goto('/this-page-does-not-exist-xyz')

    expect(response?.status()).toBe(404)

    const main = page.getByRole('main')
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(
      'Page not found',
    )
    await expect(main.getByRole('link', { name: 'Algorithms' })).toBeVisible()
    await expect(main.getByRole('link', { name: 'Problems' })).toBeVisible()
  })
})

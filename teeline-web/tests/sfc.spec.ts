import { test, expect } from '@playwright/test'

// E2E smoke tests for the Space-Filling Curve explainer
// (/algorithms/space_filling_curve/explainer/). The component hydrates with
// `client:visible`, so the helper scrolls it into view and probes the Morton
// toggle until the summary changes, then returns to Hilbert.
async function waitHydrated(page: import('@playwright/test').Page) {
  const root = page.locator('.sfc-root')
  await root.scrollIntoViewIfNeeded()
  const summary = page.locator('.sfc-summary')
  const morton = page.getByRole('button', { name: 'Morton' })
  await expect(async () => {
    await morton.click()
    await expect(summary).toContainText('Morton', { timeout: 1500 })
  }).toPass({ timeout: 15_000 })
  await page.getByRole('button', { name: 'Hilbert' }).click()
  await expect(summary).toContainText('Hilbert')
}

test.describe('space-filling curve explainer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/algorithms/space_filling_curve/explainer/')
    await waitHydrated(page)
  })

  test('renders the curve, city order, tour, stats and controls', async ({
    page,
  }) => {
    await expect(page.locator('.sfc-title')).toHaveText(
      'Space-Filling Curve Construction',
    )
    await expect(
      page.getByRole('navigation', { name: 'Explainer' }),
    ).toContainText('SFC')
    await expect(page.locator('.sfc-canvas')).toBeVisible()
    await expect(page.locator('.sfc-overlay')).toBeVisible()
    await expect(page.locator('.sfc-city')).toHaveCount(24)
    await expect(page.locator('.sfc-order-badge')).toHaveCount(24)
    await expect(page.locator('.sfc-order-list li')).toHaveCount(24)

    const stats = page.locator('.sfc-statgrid')
    for (const label of [
      'grid order',
      'cities / cells',
      'largest bucket',
      'tour length',
      'seam length',
      'longest edge',
      'long curve jumps',
    ]) {
      await expect(stats).toContainText(label)
    }

    await expect(page.getByRole('button', { name: 'Hilbert' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
    await expect(page.getByRole('button', { name: 'Morton' })).toHaveAttribute(
      'aria-pressed',
      'false',
    )
  })

  test('slider updates the curve order and statistics', async ({ page }) => {
    const summary = page.locator('.sfc-summary')
    await expect(summary).toContainText('grid order 1')

    const slider = page.locator('.sfc-slider')
    await slider.focus()
    await page.keyboard.press('Home')
    await page.keyboard.press('ArrowRight')
    await page.keyboard.press('ArrowRight')
    await page.keyboard.press('ArrowRight')

    await expect(summary).toContainText('grid order 4')
    await expect(page.locator('.sfc-order-subtitle')).toContainText('p = 4')
  })

  test('switching to Morton changes the curve and long-jump statistic', async ({
    page,
  }) => {
    const summary = page.locator('.sfc-summary')
    await page.getByRole('button', { name: 'Morton' }).click()
    await expect(summary).toContainText('Morton')
    await expect(page.locator('.sfc-order-subtitle')).toContainText('Morton')
    await expect(page.locator('.sfc-statgrid')).toContainText(
      'long curve jumps',
    )

    await page.getByRole('button', { name: 'Hilbert' }).click()
    await expect(summary).toContainText('Hilbert')
  })

  test('Play advances the subdivision and Reset returns to p=1', async ({
    page,
  }) => {
    const summary = page.locator('.sfc-summary')
    await page.locator('.sfc-play').click()
    await expect(summary).toContainText('grid order 2', { timeout: 5_000 })

    await page.locator('.sfc-reset').click()
    await expect(summary).toContainText('grid order 1')
  })

  test('reduced motion jumps straight to p=8', async ({ page }) => {
    const summary = page.locator('.sfc-summary')
    await page.emulateMedia({ reducedMotion: 'reduce' })
    await page.locator('.sfc-play').click()
    await expect(summary).toContainText('grid order 8')
  })

  test('highlights the seam and explains the 2-opt follow-up', async ({
    page,
  }) => {
    await expect(page.locator('.sfc-seam')).toHaveCount(1)
    const callout = page.locator('.sfc-callout')
    await expect(callout).toContainText('Seam:')
    await expect(callout).toContainText('2-opt')
    await expect(callout.getByRole('link', { name: '2-opt' })).toHaveAttribute(
      'href',
      '/algorithms/two-opt/',
    )
  })

  test('keyboard reaches the slider and changes the order', async ({
    page,
  }) => {
    const summary = page.locator('.sfc-summary')
    await page.locator('.sfc-slider').focus()
    await page.keyboard.press('ArrowRight')
    await expect(summary).toContainText('grid order 2')
  })

  test('stays usable on a narrow viewport', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 900 })
    await page.goto('/algorithms/space_filling_curve/explainer/')
    await expect(page.locator('.sfc-root')).toBeVisible()
    await expect(page.locator('.sfc-canvas')).toBeVisible()
    await expect(page.locator('.sfc-order-list li')).toHaveCount(24)

    const hasHorizontalScroll = await page.evaluate(
      () =>
        document.documentElement.scrollWidth >
        document.documentElement.clientWidth,
    )
    expect(hasHorizontalScroll).toBe(false)
  })
})

test.describe('space-filling curve explainer discoverability', () => {
  test('docs page links to the explainer', async ({ page }) => {
    await page.goto('/algorithms/space_filling_curve/')
    const cta = page.getByRole('link', { name: /Open interactive explainer/ })
    await expect(cta).toHaveAttribute(
      'href',
      '/algorithms/space_filling_curve/explainer/',
    )
  })
})

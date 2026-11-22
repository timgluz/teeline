import { test, expect } from '@playwright/test'

// E2E smoke tests for the Space-Filling Curve explainer
// (/algorithms/space_filling_curve/explainer/). The component hydrates with
// `client:visible`, so the helper scrolls it into view and probes the stage
// controls until Preact is attached, then returns to Project.
async function waitHydrated(page: import('@playwright/test').Page) {
  const root = page.locator('.sfc-root')
  await root.scrollIntoViewIfNeeded()
  const lesson = page.locator('.sfc-lesson-label')
  await expect(async () => {
    await page.getByRole('button', { name: 'Sort', exact: true }).click()
    await expect(lesson).toHaveText('Sort', { timeout: 1000 })
  }).toPass({ timeout: 15_000 })
  await page.getByRole('button', { name: 'Project', exact: true }).click()
  await expect(lesson).toHaveText('Project')
}

test.describe('space-filling curve explainer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/algorithms/space_filling_curve/explainer/')
    await waitHydrated(page)
  })

  test('starts at Project on p = 3 with no completed tour', async ({
    page,
  }) => {
    await expect(page.locator('.sfc-title')).toHaveText(
      'Space-Filling Curve Construction',
    )
    await expect(page.locator('.sfc-lesson-label')).toHaveText('Project')
    await expect(page.locator('.sfc-order-value')).toHaveText('p = 3')
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(0)
    await expect(page.locator('.sfc-seam')).toHaveCount(0)
    await expect(page.locator('.sfc-strip-placeholder')).toBeVisible()
    await expect(page.locator('.sfc-strip')).toHaveCount(0)

    await expect(page.locator('.sfc-city')).toHaveCount(24)
    await expect(page.locator('.sfc-order-badge')).toHaveCount(24)
    await expect(page.locator('.sfc-cell')).toHaveCount(20)
    await expect(page.locator('.sfc-cell-shared')).toHaveCount(4)
    await expect(page.locator('.sfc-row')).toHaveCount(24)
    await expect(page.locator('.sfc-group-row')).toHaveCount(4)
    await expect(page.locator('.sfc-table thead th')).toHaveText([
      'Visit',
      'City',
      'Cell',
      'Index',
    ])

    const stats = page.locator('.sfc-statgrid')
    await expect(stats).toContainText('cities / cells')
    await expect(stats).toContainText('shared cells')
    await expect(stats).toContainText('largest bucket')
    await expect(stats).not.toContainText('tour length')

    await expect(
      page.getByRole('button', { name: 'Project', exact: true }),
    ).toHaveAttribute('aria-pressed', 'true')
    await expect(page.locator('.sfc-back')).toBeDisabled()
  })

  test('stages reveal projection, indices, sort order, and tour', async ({
    page,
  }) => {
    const lesson = page.locator('.sfc-lesson-label')

    await page.getByRole('button', { name: 'Index', exact: true }).click()
    await expect(lesson).toHaveText('Index')
    await expect(page.locator('.sfc-strip')).toBeVisible()
    await expect(page.locator('.sfc-strip-tick')).toHaveCount(24)
    await expect(page.locator('.sfc-cell-index')).toHaveCount(20)
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(0)

    await page.getByRole('button', { name: 'Sort', exact: true }).click()
    await expect(lesson).toHaveText('Sort')
    await expect(
      page.locator('.sfc-order-badge').filter({ hasText: /^1$/ }),
    ).toHaveCount(1)
    await expect(
      page.locator('.sfc-order-badge').filter({ hasText: /^24$/ }),
    ).toHaveCount(1)
    await expect(page.locator('.sfc-row').first()).toContainText('1')

    await page.getByRole('button', { name: 'Connect', exact: true }).click()
    await expect(lesson).toHaveText('Connect')
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(24)
    await expect(page.locator('.sfc-seam')).toHaveCount(1)
    await expect(page.locator('.sfc-callout')).toContainText('Seam:')
    await expect(page.locator('.sfc-callout')).toContainText('2-opt')
    await expect(page.locator('.sfc-statgrid')).toContainText('tour length')
  })

  test('Step and Back walk the construction in both directions', async ({
    page,
  }) => {
    const lesson = page.locator('.sfc-lesson-label')
    await page.locator('.sfc-step').click()
    await expect(lesson).toHaveText('Index')
    await page.locator('.sfc-step').click()
    await expect(lesson).toHaveText('Sort')
    await page.locator('.sfc-step').click()
    await expect(lesson).toHaveText('Connect')
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(1)

    await page.locator('.sfc-step').click()
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(2)
    await page.locator('.sfc-back').click()
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(1)
    await page.locator('.sfc-back').click()
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(0)
    await page.locator('.sfc-back').click()
    await expect(lesson).toHaveText('Sort')
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(0)

    await page.getByRole('button', { name: 'Connect', exact: true }).click()
    await page.locator('.sfc-back').click()
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(23)
    await expect(page.locator('.sfc-seam')).toHaveCount(0)
  })

  test('Run can be paused mid-construction', async ({ page }) => {
    const lesson = page.locator('.sfc-lesson-label')
    await page.locator('.sfc-run').click()
    await expect(lesson).toHaveText('Index', { timeout: 5_000 })
    await page.locator('.sfc-run').click()
    await expect(lesson).toHaveText('Index')
    await page.waitForTimeout(900)
    await expect(lesson).toHaveText('Index')
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(0)
  })

  test('Reset restores Project at p = 3 and Hilbert', async ({ page }) => {
    await page.getByRole('button', { name: 'Connect', exact: true }).click()
    await page.locator('.sfc-next-p').click()
    await page.getByRole('button', { name: 'Morton', exact: true }).click()
    await page.locator('.sfc-reset').click()

    await expect(page.locator('.sfc-lesson-label')).toHaveText('Project')
    await expect(page.locator('.sfc-order-value')).toHaveText('p = 3')
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(0)
    await expect(page.locator('.sfc-strip-placeholder')).toBeVisible()
    await expect(
      page.getByRole('button', { name: 'Hilbert', exact: true }),
    ).toHaveAttribute('aria-pressed', 'true')
  })

  test('grid order and subdivision controls update the projection', async ({
    page,
  }) => {
    const order = page.locator('.sfc-order-value')
    await expect(order).toHaveText('p = 3')

    await page.locator('.sfc-next-p').click()
    await expect(order).toHaveText('p = 4')
    await expect(page.locator('.sfc-group-row')).toHaveCount(0)
    await page.locator('.sfc-prev-p').click()
    await expect(order).toHaveText('p = 3')
    await expect(page.locator('.sfc-group-row')).toHaveCount(4)

    const subdivision = page.locator('.sfc-play-subdivision')
    await subdivision.click()
    await expect(order).toHaveText('p = 4', { timeout: 5_000 })
    await subdivision.click()
    await expect(page.locator('.sfc-play-subdivision')).toContainText('Play')
    const paused = await order.textContent()
    await page.waitForTimeout(900)
    await expect(order).toHaveText(paused ?? '')
  })

  test('reduced motion jumps construction and subdivision to the end', async ({
    page,
  }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' })
    await page.locator('.sfc-run').click()
    await expect(page.locator('.sfc-lesson-label')).toHaveText('Connect')
    await expect(page.locator('.sfc-tour-edge')).toHaveCount(24)

    await page.locator('.sfc-reset').click()
    await page.locator('.sfc-play-subdivision').click()
    await expect(page.locator('.sfc-order-value')).toHaveText('p = 8')
  })

  test('explains and highlights order changes after a grid change', async ({
    page,
  }) => {
    await page.getByRole('button', { name: 'Sort', exact: true }).click()
    await page.locator('.sfc-next-p').click()

    const note = page.locator('.sfc-change-note')
    await expect(note).toBeVisible()
    await expect(note).toContainText('4 cities changed order')
    await expect(page.locator('.sfc-row-changed')).toHaveCount(4)
    await expect(page.locator('.sfc-city-changed')).toHaveCount(4)
    await expect(page.locator('.sfc-statgrid')).toContainText('changed cities')
  })

  test('2-opt preview ghosts the seam and shows the real delta', async ({
    page,
  }) => {
    await page.getByRole('button', { name: 'Connect', exact: true }).click()
    const callout = page.locator('.sfc-callout')
    await expect(callout).toContainText('Preview 2-opt fix')

    await page.locator('.sfc-preview-btn').click()
    await expect(page.locator('.sfc-preview-removed')).toHaveCount(2)
    await expect(page.locator('.sfc-preview-added')).toHaveCount(2)
    const delta = page.locator('.sfc-preview-delta')
    await expect(delta).toContainText('1566')
    await expect(delta).toContainText('1503')

    await page.locator('.sfc-preview-btn').click()
    await expect(page.locator('.sfc-preview-removed')).toHaveCount(0)
  })

  test('Morton calls out quadrant crossings and long jumps', async ({
    page,
  }) => {
    await page.getByRole('button', { name: 'Index', exact: true }).click()
    await page.getByRole('button', { name: 'Morton', exact: true }).click()
    await expect(page.locator('.sfc-curve-note')).toContainText('quadrant')
    await expect(page.locator('.sfc-statgrid')).toContainText(
      'long curve jumps',
    )
    await expect(page.locator('.sfc-statgrid')).toContainText('31')
    await expect(page.locator('.sfc-strip')).toBeVisible()
  })

  test('table and strip expose the sort key', async ({ page }) => {
    await page.getByRole('button', { name: 'Sort', exact: true }).click()
    await expect(page.locator('.sfc-table caption')).toHaveText(
      'Cities sorted by curve index',
    )
    await expect(page.locator('.sfc-group-row').first()).toContainText(
      'input order',
    )
    await expect(page.locator('.sfc-strip-caption')).toContainText(
      '1D curve index',
    )
    await expect(page.locator('.sfc-strip-tick')).toHaveCount(24)
  })

  test('keyboard advances stages and changes the grid order', async ({
    page,
  }) => {
    await page.locator('.sfc-step').focus()
    await page.keyboard.press('Enter')
    await expect(page.locator('.sfc-lesson-label')).toHaveText('Index')

    await page.locator('.sfc-slider').focus()
    await page.keyboard.press('ArrowRight')
    await expect(page.locator('.sfc-order-value')).toHaveText('p = 4')
  })

  test('stays usable on a narrow viewport', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 900 })
    await page.goto('/algorithms/space_filling_curve/explainer/')
    await expect(page.locator('.sfc-root')).toBeVisible()
    await expect(page.locator('.sfc-canvas')).toBeVisible()
    await expect(page.locator('.sfc-table')).toBeVisible()
    await expect(page.locator('.sfc-strip-placeholder')).toBeVisible()

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

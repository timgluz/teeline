import { test, expect } from '@playwright/test'

// E2E smoke tests for the Memetic EAX explainer
// (/algorithms/eax/explainer/). The component hydrates with `client:visible`,
// so the helper scrolls it into view and probes the stage controls until Preact
// is attached, then returns to Compare Parents.
async function waitHydrated(page: import('@playwright/test').Page) {
  const root = page.locator('.eax-root')
  await root.scrollIntoViewIfNeeded()
  const label = page.locator('.eax-stage-label')
  await expect(async () => {
    await page.getByRole('button', { name: /2\. Union Graph/ }).click()
    await expect(label).toHaveText('Union Graph', { timeout: 1000 })
  }).toPass({ timeout: 15_000 })
  await page.getByRole('button', { name: /1\. Compare Parents/ }).click()
  await expect(label).toHaveText('Compare Parents')
}

test.describe('memetic EAX explainer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/algorithms/eax/explainer/')
    await waitHydrated(page)
  })

  test('starts at Compare Parents with both tours and the pinned fixture', async ({
    page,
  }) => {
    await expect(page.locator('.eax-title')).toHaveText(
      'Memetic EAX — one crossover event',
    )
    await expect(page.locator('.eax-stage-label')).toHaveText('Compare Parents')
    await expect(page.locator('.eax-city')).toHaveCount(16)
    // Both complete parent tours are drawn in Stage 1 (16 + 16 edges).
    await expect(page.locator('.eax-canvas line')).toHaveCount(32)
    await expect(page.locator('.eax-statgrid')).toContainText('11/16 (69%)')
    await expect(page.locator('.eax-statgrid')).toContainText('subtours')
    await expect(page.locator('.eax-statgrid')).toContainText('bridges')
    await expect(page.locator('.eax-back')).toBeDisabled()
    await expect(
      page.getByRole('button', { name: /1\. Compare Parents/ }),
    ).toHaveAttribute('aria-pressed', 'true')
  })

  test('stage buttons jump through the six stages', async ({ page }) => {
    const label = page.locator('.eax-stage-label')
    for (const [n, name] of [
      [2, 'Union Graph'],
      [3, 'AB-Cycles'],
      [4, 'Select E-set'],
      [5, 'Apply E-set'],
      [6, 'Patch Subtours'],
      [1, 'Compare Parents'],
    ] as const) {
      await page
        .getByRole('button', { name: new RegExp(`${n}\\. ${name}`) })
        .click()
      await expect(label).toHaveText(name)
    }
  })

  test('Step and Back walk the same frames in both directions', async ({
    page,
  }) => {
    const label = page.locator('.eax-stage-label')
    await page.locator('.eax-step').click()
    await expect(label).toHaveText('Union Graph')
    await page.locator('.eax-step').click()
    await expect(label).toHaveText('AB-Cycles')
    await page.locator('.eax-back').click()
    await expect(label).toHaveText('Union Graph')
    await page.locator('.eax-back').click()
    await expect(label).toHaveText('Compare Parents')
  })

  test('Run advances and Reset rewinds', async ({ page }) => {
    await page.locator('.eax-run').click()
    await expect(page.locator('.eax-stage-label')).not.toHaveText(
      'Compare Parents',
      {
        timeout: 5_000,
      },
    )
    await page.locator('.eax-run').click()
    await page.locator('.eax-reset').click()
    await expect(page.locator('.eax-stage-label')).toHaveText('Compare Parents')
  })

  test('the default 1-AB selection produces two subtours and one bridge', async ({
    page,
  }) => {
    await expect(
      page.getByRole('button', { name: '1-AB', exact: true }),
    ).toHaveAttribute('aria-pressed', 'true')
    const stats = page.locator('.eax-statgrid')
    await expect(stats).toContainText('subtours')
    await expect(stats).toContainText('2')
    await expect(stats).toContainText('bridges')
    await expect(stats).toContainText('1 cycle')
    await expect(page.locator('.eax-footer')).toContainText('E-set: C1')
  })

  test('selecting every cycle reproduces Parent B (no patching)', async ({
    page,
  }) => {
    // Add Cycle #2 to the default Cycle #1 selection.
    await page.getByRole('button', { name: /Cycle #2/ }).click()
    await expect(
      page.getByRole('button', { name: /Cycle #2/ }),
    ).toHaveAttribute('aria-pressed', 'true')
    await expect(page.locator('.eax-footer')).toContainText('E-set: C1 + C2')
    const stats = page.locator('.eax-statgrid')
    await expect(stats).toContainText('2 cycles')
    // cost A / B / child: the child equals Parent B (no bridge).
    await expect(stats).toContainText('1082.2 / 1156.5 / 1156.5')
    await page.getByRole('button', { name: /6\. Patch Subtours/ }).click()
    await expect(page.locator('.eax-stage-label')).toHaveText('Patch Subtours')
    await expect(page.locator('.eax-callout')).toContainText(
      'Parent A becomes Parent B exactly',
    )
  })

  test('C2 alone needs no patching', async ({ page }) => {
    // Add C2, then deselect C1 so only C2 remains.
    await page.getByRole('button', { name: /Cycle #2/ }).click()
    await page.getByRole('button', { name: /Cycle #1/ }).click()
    await expect(page.locator('.eax-footer')).toContainText('E-set: C2')
    await page.getByRole('button', { name: /6\. Patch Subtours/ }).click()
    await expect(page.locator('.eax-callout')).toContainText(
      'No patching needed',
    )
  })

  test('the last active cycle cannot be deselected', async ({ page }) => {
    await page.getByRole('button', { name: /Cycle #1/ }).click()
    await expect(
      page.getByRole('button', { name: /Cycle #1/ }),
    ).toHaveAttribute('aria-pressed', 'true')
    await expect(page.locator('.eax-notice')).toContainText(
      'At least one AB-cycle is required',
    )
  })

  test('Rand is seeded and reproducible across reloads', async ({ page }) => {
    await page.getByRole('button', { name: 'Rand', exact: true }).click()
    const first = await page.locator('.eax-footer').innerText()
    await expect(page.locator('.eax-seed')).toContainText('seed 7')

    await page.reload()
    await waitHydrated(page)
    await page.getByRole('button', { name: 'Rand', exact: true }).click()
    const second = await page.locator('.eax-footer').innerText()
    expect(second).toBe(first)
  })

  test('2-opt preview draws the real move and delta', async ({ page }) => {
    await page.getByRole('button', { name: /6\. Patch Subtours/ }).click()
    // Stage 6 has a bridge frame then the final child frame.
    await page.locator('.eax-step').click()
    await expect(page.locator('.eax-stage-label')).toHaveText('Patch Subtours')
    await page.locator('.eax-preview-btn').click()
    await expect(page.locator('.eax-preview-removed')).toContainText('0-5')
    await expect(page.locator('.eax-preview-added')).toContainText('0-3')
    await expect(page.locator('.eax-preview-delta')).toContainText('-15.3')
    await expect(page.locator('.eax-preview-delta')).toContainText('173.2')
  })

  test('cycle toggles expose their state and the stage is a live region', async ({
    page,
  }) => {
    await expect(page.locator('.eax-stage-label')).toHaveAttribute(
      'aria-live',
      'polite',
    )
    const cycle = page.getByRole('button', { name: /Cycle #2/ })
    await expect(cycle).toHaveAttribute('aria-pressed', 'false')
    await cycle.click()
    await expect(cycle).toHaveAttribute('aria-pressed', 'true')
  })

  test('keyboard advances the storyboard', async ({ page }) => {
    await page.locator('.eax-step').focus()
    await page.keyboard.press('Enter')
    await expect(page.locator('.eax-stage-label')).toHaveText('Union Graph')
  })

  test('stays usable on a narrow viewport', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 900 })
    await page.goto('/algorithms/eax/explainer/')
    await expect(page.locator('.eax-root')).toBeVisible()
    await expect(page.locator('.eax-canvas')).toBeVisible()
    await expect(page.locator('.eax-table')).toBeVisible()
    const hasHorizontalScroll = await page.evaluate(
      () =>
        document.documentElement.scrollWidth >
        document.documentElement.clientWidth,
    )
    expect(hasHorizontalScroll).toBe(false)
  })
})

test.describe('memetic EAX explainer discoverability', () => {
  test('docs page links to the explainer', async ({ page }) => {
    await page.goto('/algorithms/eax/')
    const cta = page.getByRole('link', { name: /Open interactive explainer/ })
    await expect(cta).toHaveAttribute('href', '/algorithms/eax/explainer/')
  })

  test('the algorithms index lists the explainer', async ({ page }) => {
    await page.goto('/algorithms/')
    const row = page.getByRole('row').filter({ hasText: 'Memetic EAX' })
    await expect(
      row.getByRole('link', {
        name: /Open interactive explainer for Memetic EAX/,
      }),
    ).toBeVisible()
  })
})

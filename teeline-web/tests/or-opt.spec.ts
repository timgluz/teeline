import { test, expect } from '@playwright/test'

// E2E smoke tests for the Or-opt explainer (/algorithms/or_opt/explainer/).
// The component hydrates with `client:visible`, so the tests first scroll it
// into view and probe until the Preact listeners are attached (a click that
// changes the status chip), then reset to the idle phase.
async function waitHydrated(page: import('@playwright/test').Page) {
  const root = page.locator('.or-root')
  await root.scrollIntoViewIfNeeded()
  const chip = page.locator('.or-chip')
  const step = page.getByRole('button', { name: 'Step' })
  // Retry the click until it lands post-hydration (pre-hydration clicks are no-ops).
  await expect(async () => {
    await step.click()
    await expect(chip).toContainText('Candidate', { timeout: 1500 })
  }).toPass({ timeout: 15_000 })
  // back to the idle phase for deterministic tests
  await page.getByRole('button', { name: 'Reset' }).click()
  await expect(chip).toContainText('Click Step')
}

test.describe('or-opt explainer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/algorithms/or_opt/explainer/')
    await waitHydrated(page)
  })

  test('renders the canvas, stats panel and controls', async ({ page }) => {
    await expect(page.locator('.or-title')).toHaveText('Or-opt Local Search')
    await expect(page.locator('.or-canvas')).toBeVisible()
    await expect(page.locator('.or-section-label', { hasText: 'Cost over passes' })).toBeVisible()

    const stats = page.locator('.or-statgrid')
    await expect(stats).toContainText('pass')
    await expect(stats).toContainText('moves')
    await expect(stats).toContainText('best cost')
    await expect(stats).toContainText('distance')
    await expect(stats).toContainText('last Δ')
    await expect(stats).toContainText('step')

    for (const label of ['Single segment', 'Triplet move', 'Already optimal', '2-opt stuck']) {
      await expect(page.getByRole('button', { name: label })).toBeVisible()
    }
  })

  test('Step advances the two-click model (candidate → applied)', async ({ page }) => {
    const chip = page.locator('.or-chip')
    const pass = page.locator('.or-statgrid').locator('div', { hasText: /^pass/ }).locator('.or-mono')
    const moves = page.locator('.or-statgrid').locator('div', { hasText: /^moves/ }).locator('.or-mono')

    await page.getByRole('button', { name: 'Step' }).click()
    // single_segment scenario: the best move is an Or-1 relocation of city 8
    await expect(chip).toContainText('Candidate')
    await expect(chip).toContainText('k=1')

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(chip).toContainText('Moved')
    await expect(pass).toHaveText('1')
    await expect(moves).toHaveText('1')

    // sparkline appears once there are ≥ 2 cost samples
    await expect(page.locator('.or-spark')).toBeVisible()
  })

  test('Back restores the previous phase', async ({ page }) => {
    const chip = page.locator('.or-chip')
    const stepStat = page.locator('.or-statgrid').locator('div', { hasText: /^step/ }).locator('.or-mono')

    await page.getByRole('button', { name: 'Step' }).click() // candidate
    await page.getByRole('button', { name: 'Step' }).click() // applied
    await expect(stepStat).toHaveText('2')

    await page.getByRole('button', { name: 'Back' }).click()
    await expect(chip).toContainText('Candidate') // back to the candidate phase
    await expect(stepStat).toHaveText('1')
  })

  test('Run animates passes; Pause and Reset stop and reset', async ({ page }) => {
    const pass = page.locator('.or-statgrid').locator('div', { hasText: /^pass/ }).locator('.or-mono')

    await page.getByRole('button', { name: 'Run' }).click()
    await expect(pass).not.toHaveText('0', { timeout: 10_000 })

    await page.getByRole('button', { name: 'Pause' }).click()
    const paused = await pass.textContent()
    await page.waitForTimeout(500)
    await expect(pass).toHaveText(paused ?? '')

    await page.getByRole('button', { name: 'Reset' }).click()
    await expect(pass).toHaveText('0')
    await expect(page.locator('.or-chip')).toContainText('Click Step')
  })

  test('already_optimal reaches the local optimum in one Step', async ({ page }) => {
    await page.getByRole('button', { name: 'Already optimal' }).click()
    await expect(page.locator('.or-chip')).toContainText('Click Step')

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(page.locator('.or-chip')).toContainText('Local optimum')
  })

  test('scenario buttons restart the run', async ({ page }) => {
    const pass = page.locator('.or-statgrid').locator('div', { hasText: /^pass/ }).locator('.or-mono')

    await page.getByRole('button', { name: 'Step' }).click()
    await page.getByRole('button', { name: 'Step' }).click()
    await expect(pass).not.toHaveText('0')

    await page.getByRole('button', { name: 'Triplet move' }).click()
    await expect(pass).toHaveText('0')
    await expect(page.locator('.or-chip')).toContainText('Click Step')

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(page.locator('.or-chip')).toContainText('k=3')
  })
})

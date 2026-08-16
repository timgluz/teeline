import { test, expect } from '@playwright/test'

// E2E smoke tests for the Stochastic Hill Climbing explainer
// (/algorithms/stochastic_hill/explainer/). The component hydrates with
// `client:visible`, so the tests first scroll it into view and probe until
// the Preact listeners are attached (a click that changes the status chip),
// then reset to the idle phase.
async function waitHydrated(page: import('@playwright/test').Page) {
  const root = page.locator('.shc-root')
  await root.scrollIntoViewIfNeeded()
  const chip = page.locator('.shc-chip')
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

test.describe('stochastic hill explainer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/algorithms/stochastic_hill/explainer/')
    await waitHydrated(page)
  })

  test('renders the canvas, stats panel and controls', async ({ page }) => {
    await expect(page.locator('.shc-title')).toHaveText('Stochastic Hill Climbing')
    await expect(page.locator('.shc-canvas')).toBeVisible()
    // sparkline section label is present; the line itself appears after the
    // first verdict (needs ≥ 2 cost samples)
    await expect(page.locator('.shc-section-label', { hasText: 'Best cost over epochs' })).toBeVisible()

    // stats panel
    const stats = page.locator('.shc-statgrid')
    await expect(stats).toContainText('epoch')
    await expect(stats).toContainText('restarts')
    await expect(stats).toContainText('best cost')
    await expect(stats).toContainText('current cost')
    await expect(stats).toContainText('accept rate')

    // all three scenario buttons
    for (const label of ['Quick convergence', 'Rugged landscape', 'Needle in haystack']) {
      await expect(page.getByRole('button', { name: label })).toBeVisible()
    }
  })

  test('Step advances the two-phase machine (propose → verdict)', async ({ page }) => {
    const chip = page.locator('.shc-chip')

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(chip).toContainText('Candidate')

    await page.getByRole('button', { name: 'Step' }).click()
    // verdict chip: accepted, rejected or restarting
    await expect(chip).toContainText(/Accepted|Rejected|Restarting/)
    // sparkline appears once there are ≥ 2 best-cost samples
    await expect(page.locator('.shc-spark')).toBeVisible()

    // epoch advanced by exactly one verdict
    const epoch = page.locator('.shc-statgrid').locator('div', { hasText: /^epoch/ }).locator('.shc-mono')
    await expect(epoch).toHaveText('1')
  })

  test('Back restores the previous phase', async ({ page }) => {
    const chip = page.locator('.shc-chip')
    const stepStat = page.locator('.shc-statgrid').locator('div', { hasText: /^step/ }).locator('.shc-mono')

    await page.getByRole('button', { name: 'Step' }).click() // propose
    await page.getByRole('button', { name: 'Step' }).click() // verdict
    await expect(stepStat).toHaveText('2')

    await page.getByRole('button', { name: 'Back' }).click()
    await expect(chip).toContainText('Candidate') // back to the propose phase
    await expect(stepStat).toHaveText('1')
  })

  test('Run animates epochs; Pause and Reset stop and reset', async ({ page }) => {
    const epoch = page.locator('.shc-statgrid').locator('div', { hasText: /^epoch/ }).locator('.shc-mono')
    const run = page.getByRole('button', { name: 'Run' })

    await run.click()
    await expect(epoch).not.toHaveText('0', { timeout: 10_000 })

    await page.getByRole('button', { name: 'Pause' }).click()
    const paused = await epoch.textContent()
    await page.waitForTimeout(500)
    await expect(epoch).toHaveText(paused ?? '')

    await page.getByRole('button', { name: 'Reset' }).click()
    await expect(epoch).toHaveText('0')
    await expect(page.locator('.shc-chip')).toContainText('Click Step')
  })

  test('scenario buttons restart the run with their own parameters', async ({ page }) => {
    const epoch = page.locator('.shc-statgrid').locator('div', { hasText: /^epoch/ }).locator('.shc-mono')
    const restarts = page.locator('.shc-statgrid').locator('div', { hasText: /^restarts/ }).locator('.shc-mono')

    await page.getByRole('button', { name: 'Rugged landscape' }).click()
    await expect(epoch).toHaveText('0')
    await expect(restarts).toHaveText('0')
    await expect(page.locator('.shc-chip')).toContainText('Click Step')

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(page.locator('.shc-chip')).toContainText('Candidate')
  })

  test('epochs / restart-patience inputs commit on blur and restart the run', async ({ page }) => {
    const epochsInput = page.locator('#shc-epochs')
    const epoch = page.locator('.shc-statgrid').locator('div', { hasText: /^epoch/ }).locator('.shc-mono')

    // advance a few steps first
    await page.getByRole('button', { name: 'Run' }).click()
    await expect(epoch).not.toHaveText('0', { timeout: 10_000 })
    await page.getByRole('button', { name: 'Pause' }).click()

    await epochsInput.fill('20')
    await epochsInput.blur()
    // changing parameters restarts the run
    await expect(epoch).toHaveText('0')
    await expect(epochsInput).toHaveValue('20')
  })
})

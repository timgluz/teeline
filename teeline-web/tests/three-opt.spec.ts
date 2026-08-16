import { test, expect } from '@playwright/test'

// E2E smoke tests for the 3-opt explainer (/algorithms/3opt/explainer/).
// The component hydrates with `client:visible`, so the tests first scroll it
// into view and probe until the Preact listeners are attached (a click that
// changes the status chip), then reset to the idle phase.
async function waitHydrated(page: import('@playwright/test').Page) {
  const root = page.locator('.t3-root')
  await root.scrollIntoViewIfNeeded()
  const chip = page.locator('.t3-chip')
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

test.describe('3-opt explainer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/algorithms/3opt/explainer/')
    await waitHydrated(page)
  })

  test('renders the canvas, pattern diagram, stats and controls', async ({ page }) => {
    await expect(page.locator('.t3-title')).toHaveText('3-opt Local Search')
    await expect(page.locator('.t3-canvas')).toBeVisible()

    // the 7-reconnection pattern diagram
    await expect(page.locator('.t3-pattern-grid')).toBeVisible()
    await expect(page.locator('.t3-pattern-cell')).toHaveCount(7)

    const stats = page.locator('.t3-statgrid')
    await expect(stats).toContainText('pass')
    await expect(stats).toContainText('swaps')
    await expect(stats).toContainText('best cost')
    await expect(stats).toContainText('distance')
    await expect(stats).toContainText('last Δ')
    await expect(stats).toContainText('step')

    for (const label of ['Single 3-opt', 'Beyond 2-opt', 'Already 3-optimal', 'Deep sweep']) {
      await expect(page.getByRole('button', { name: label })).toBeVisible()
    }
  })

  test('Step shows the candidate with the chosen reconnection pattern highlighted, then applies', async ({ page }) => {
    const chip = page.locator('.t3-chip')
    const pass = page.locator('.t3-statgrid').locator('div', { hasText: /^pass/ }).locator('.t3-mono')
    const swaps = page.locator('.t3-statgrid').locator('div', { hasText: /^swaps/ }).locator('.t3-mono')

    await page.getByRole('button', { name: 'Step' }).click()
    // single_3opt scenario: the best move is a case-4 segment swap
    await expect(chip).toContainText('Candidate')
    await expect(chip).toContainText('case 4')
    await expect(page.locator('.t3-pattern-active')).toHaveCount(1)

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(chip).toContainText('Applied')
    await expect(pass).toHaveText('1')
    await expect(swaps).toHaveText('1')

    // sparkline appears once there are ≥ 2 cost samples
    await expect(page.locator('.t3-spark')).toBeVisible()
  })

  test('Back restores the previous phase', async ({ page }) => {
    const chip = page.locator('.t3-chip')
    const stepStat = page.locator('.t3-statgrid').locator('div', { hasText: /^step/ }).locator('.t3-mono')

    await page.getByRole('button', { name: 'Step' }).click() // candidate
    await page.getByRole('button', { name: 'Step' }).click() // applied
    await expect(stepStat).toHaveText('2')

    await page.getByRole('button', { name: 'Back' }).click()
    await expect(chip).toContainText('Candidate') // back to the candidate phase
    await expect(stepStat).toHaveText('1')
  })

  test('Run animates the deep sweep to the local optimum; Reset restores idle', async ({ page }) => {
    const pass = page.locator('.t3-statgrid').locator('div', { hasText: /^pass/ }).locator('.t3-mono')
    const swaps = page.locator('.t3-statgrid').locator('div', { hasText: /^swaps/ }).locator('.t3-mono')

    await page.getByRole('button', { name: 'Deep sweep' }).click()
    await page.getByRole('button', { name: 'Run' }).click()
    // deep_sweep needs several passes to converge
    await expect(page.locator('.t3-chip')).toContainText('Local optimum', { timeout: 15_000 })
    await expect(swaps).not.toHaveText('0')
    await expect(pass).not.toHaveText('0')

    await page.getByRole('button', { name: 'Reset' }).click()
    await expect(pass).toHaveText('0')
    await expect(page.locator('.t3-chip')).toContainText('Click Step')
  })

  test('Pause stops the animation mid-run', async ({ page }) => {
    const pass = page.locator('.t3-statgrid').locator('div', { hasText: /^pass/ }).locator('.t3-mono')

    await page.getByRole('button', { name: 'Run' }).click()
    await page.getByRole('button', { name: 'Pause' }).click()
    const paused = await pass.textContent()
    await page.waitForTimeout(700)
    await expect(pass).toHaveText(paused ?? '')
  })

  test('already_3optimal reaches the local optimum in one Step', async ({ page }) => {
    await page.getByRole('button', { name: 'Already 3-optimal' }).click()
    await expect(page.locator('.t3-chip')).toContainText('Click Step')

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(page.locator('.t3-chip')).toContainText('Local optimum')
  })

  test('scenario buttons restart the run', async ({ page }) => {
    const pass = page.locator('.t3-statgrid').locator('div', { hasText: /^pass/ }).locator('.t3-mono')

    await page.getByRole('button', { name: 'Step' }).click()
    await page.getByRole('button', { name: 'Step' }).click()
    await expect(pass).not.toHaveText('0')

    await page.getByRole('button', { name: 'Beyond 2-opt' }).click()
    await expect(pass).toHaveText('0')
    await expect(page.locator('.t3-chip')).toContainText('Click Step')

    await page.getByRole('button', { name: 'Step' }).click()
    // beyond_2opt: the best move is a case-6 segment swap
    await expect(page.locator('.t3-chip')).toContainText('case 6')
  })
})

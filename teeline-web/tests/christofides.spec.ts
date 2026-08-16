import { test, expect } from '@playwright/test'

// E2E smoke tests for the Christofides explainer (/algorithms/christofides/explainer/).
// The component hydrates with `client:visible`, so the tests first scroll it
// into view and probe until the Preact listeners are attached (a click that
// changes the status chip), then reset to the idle phase.
async function waitHydrated(page: import('@playwright/test').Page) {
  const root = page.locator('.chr-root')
  await root.scrollIntoViewIfNeeded()
  const chip = page.locator('.chr-chip')
  const step = page.getByRole('button', { name: 'Step' })
  await expect(async () => {
    await step.click()
    await expect(chip).toContainText('MST —', { timeout: 1500 })
  }).toPass({ timeout: 15_000 })
  await page.getByRole('button', { name: 'Reset' }).click()
  await expect(chip).toContainText('grow the tree')
}

test.describe('christofides explainer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/algorithms/christofides/explainer/')
    await waitHydrated(page)
  })

  test('renders the canvas, pipeline stepper, ratio meter, proof and compare panels', async ({ page }) => {
    await expect(page.locator('.chr-title')).toHaveText('Christofides — a ≤1.5× Approximation')
    await expect(page.locator('.chr-canvas')).toBeVisible()

    // six pipeline phases
    await expect(page.locator('.chr-phase-btn')).toHaveCount(6)

    // ratio meter + proof + compare
    await expect(page.locator('.chr-meter')).toBeVisible()
    await expect(page.locator('.chr-proof')).toBeVisible()
    await expect(page.locator('.chr-compare-row')).toHaveCount(3)

    const stats = page.locator('.chr-statgrid')
    await expect(stats).toContainText('MST cost')
    await expect(stats).toContainText('matching cost')
    await expect(stats).toContainText('tour cost')
    await expect(stats).toContainText('ratio')

    for (const label of ['Balanced', 'Near-optimal', 'Matching-heavy', 'Clustered', 'Worst case']) {
      await expect(page.getByRole('button', { name: label })).toBeVisible()
    }
  })

  test('Step reveals MST edges one at a time', async ({ page }) => {
    const chip = page.locator('.chr-chip')
    await page.getByRole('button', { name: 'Step' }).click()
    await expect(chip).toContainText('added edge')
    await expect(chip).toContainText('1/9')
    await page.getByRole('button', { name: 'Step' }).click()
    await expect(chip).toContainText('2/9')
  })

  test('the phase selector jumps straight to any phase', async ({ page }) => {
    const chip = page.locator('.chr-chip')

    await page.getByRole('button', { name: 'Odd vertices' }).click()
    await expect(chip).toContainText('Odd-degree vertices')

    await page.getByRole('button', { name: 'Euler walk' }).click()
    await expect(chip).toContainText('Eulerian circuit')

    await page.getByRole('button', { name: 'Done' }).click()
    await expect(chip).toContainText('Done —')
    await expect(page.locator('.chr-meter-value')).toContainText('× optimal')
  })

  test('Run animates through the whole pipeline to Done; Reset restores idle', async ({ page }) => {
    const chip = page.locator('.chr-chip')
    const step = page.locator('.chr-statgrid').locator('div', { hasText: /^step/ }).locator('.chr-mono')

    await page.getByRole('button', { name: 'Run' }).click()
    await expect(chip).toContainText('Done —', { timeout: 20_000 })
    await expect(step).not.toHaveText('0')

    await page.getByRole('button', { name: 'Reset' }).click()
    await expect(chip).toContainText('grow the tree')
    await expect(step).toHaveText('0')
  })

  test('Pause stops the animation mid-run', async ({ page }) => {
    const step = page.locator('.chr-statgrid').locator('div', { hasText: /^step/ }).locator('.chr-mono')

    await page.getByRole('button', { name: 'Run' }).click()
    await page.getByRole('button', { name: 'Pause' }).click()
    const paused = await step.textContent()
    await page.waitForTimeout(600)
    await expect(step).toHaveText(paused ?? '')
  })

  test('Back restores the previous step', async ({ page }) => {
    const step = page.locator('.chr-statgrid').locator('div', { hasText: /^step/ }).locator('.chr-mono')

    await page.getByRole('button', { name: 'Step' }).click()
    await page.getByRole('button', { name: 'Step' }).click()
    await expect(step).toHaveText('2')

    await page.getByRole('button', { name: 'Back' }).click()
    await expect(step).toHaveText('1')
  })

  test('scenario buttons restart the run with their own ratio', async ({ page }) => {
    await page.getByRole('button', { name: 'Worst case' }).click()
    await page.getByRole('button', { name: 'Done' }).click()
    // worst_case is pinned at ~1.39× — the ratio stat reflects it
    const ratio = page.locator('.chr-statgrid').locator('div', { hasText: /^ratio/ }).locator('.chr-mono')
    await expect(ratio).toHaveText('1.39×')
  })
})

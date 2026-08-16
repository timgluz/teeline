import { test, expect } from '@playwright/test'

// E2E smoke tests for the Bellman-Held-Karp explainer (/algorithms/bhk/explainer/).
async function waitHydrated(page: import('@playwright/test').Page) {
  const root = page.locator('.bhk-root')
  await root.scrollIntoViewIfNeeded()
  const chip = page.locator('.bhk-chip')
  const step = page.getByRole('button', { name: 'Step' })
  await expect(async () => {
    await step.click()
    await expect(chip).toContainText('dp[', { timeout: 1500 })
  }).toPass({ timeout: 15_000 })
  await page.getByRole('button', { name: 'Reset' }).click()
}

test.describe('bhk explainer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/algorithms/bhk/explainer/')
    await waitHydrated(page)
  })

  test('renders the DP table, map, stats and scenarios', async ({ page }) => {
    await expect(page.locator('.bhk-title')).toContainText('Bellman-Held-Karp')
    await expect(page.locator('.bhk-table')).toBeVisible()
    await expect(page.locator('.bhk-map')).toBeVisible()

    const stats = page.locator('.bhk-statgrid')
    await expect(stats).toContainText('subset size')
    await expect(stats).toContainText('bits set')
    await expect(stats).toContainText('phase')

    for (const label of ['6-city grid', 'Circle', 'Two clusters']) {
      await expect(page.getByRole('button', { name: label })).toBeVisible()
    }
  })

  test('Step fills one DP cell at a time', async ({ page }) => {
    const chip = page.locator('.bhk-chip')

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(chip).toContainText('dp[')
    await expect(chip).toContainText('via city')
  })

  test('clicking a cell explains its computation', async ({ page }) => {
    // the first size-2 cell (mask 00011, row 0) exists after one step
    await page.getByRole('button', { name: 'Step' }).click()
    await page.locator('.bhk-cell', { hasText: /^[0-9]/ }).first().click()
    await expect(page.locator('.bhk-cellinfo')).toContainText('dp[')
  })

  test('Run completes the whole DP, then read-back reveals the optimal route', async ({ page }) => {
    const chip = page.locator('.bhk-chip')

    // Run goes all the way through the table and the read-back to Done
    await page.getByRole('button', { name: 'Run' }).click()
    await expect(chip).toContainText('Done — optimal tour', { timeout: 30_000 })

    // jump to the read-back mode and step: the route is revealed city by city
    await page.getByRole('button', { name: 'Read-back' }).click()
    await page.getByRole('button', { name: 'Step' }).click()
    await expect(chip).toContainText('Read-back')
    await expect(chip).toContainText('revealed')
  })

  test('Done shows the full optimal tour; Reset restores', async ({ page }) => {
    const chip = page.locator('.bhk-chip')

    await page.getByRole('button', { name: 'Done' }).click()
    await expect(chip).toContainText('Done — optimal tour')

    await page.getByRole('button', { name: 'Reset' }).click()
    await expect(chip).toContainText('fills subset by subset')
  })

  test('Back restores the previous step', async ({ page }) => {
    const step = page.locator('.bhk-statgrid').locator('div', { hasText: /^step/ }).locator('.bhk-mono')
    const back = page.getByRole('button', { name: '⏴ Back' })

    await page.getByRole('button', { name: 'Step' }).click()
    await page.getByRole('button', { name: 'Step' }).click()
    const after = await step.textContent()

    await back.click()
    await expect(step).not.toHaveText(after ?? '')
  })

  test('scenario buttons restart the run', async ({ page }) => {
    const step = page.locator('.bhk-statgrid').locator('div', { hasText: /^step/ }).locator('.bhk-mono')

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(step).not.toHaveText('0')

    await page.getByRole('button', { name: 'Circle' }).click()
    await expect(step).toHaveText('0')
  })
})

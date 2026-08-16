import { test, expect } from '@playwright/test'

// E2E smoke tests for the Branch & Bound explainer (/algorithms/branch_bound/explainer/).
async function waitHydrated(page: import('@playwright/test').Page) {
  const root = page.locator('.bb-root')
  const chip = page.locator('.bb-chip')
  const step = page.getByRole('button', { name: 'Step' })
  // scroll + probe in a retry loop: `client:visible` hydration swaps the SSR
  // DOM, which can detach the locator mid-scroll — a transient failure is retried
  await expect(async () => {
    await root.scrollIntoViewIfNeeded()
    await step.click()
    await expect(chip).toContainText('Expanded', { timeout: 1500 })
  }).toPass({ timeout: 15_000 })
  await page.getByRole('button', { name: 'Reset' }).click()
}

test.describe('branch & bound explainer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/algorithms/branch_bound/explainer/')
    await waitHydrated(page)
  })

  test('renders the search tree, city map, stats and scenarios', async ({ page }) => {
    await expect(page.locator('.bb-title')).toContainText('Branch & Bound')
    await expect(page.locator('.bb-tree')).toBeVisible()
    await expect(page.locator('.bb-map')).toBeVisible()

    const stats = page.locator('.bb-statgrid')
    await expect(stats).toContainText('nodes')
    await expect(stats).toContainText('explored')
    await expect(stats).toContainText('pruned')
    await expect(stats).toContainText('leaves')
    await expect(stats).toContainText('best')

    for (const label of ['Small grid', 'Good bound', 'Worst case', 'Early best']) {
      await expect(page.getByRole('button', { name: label })).toBeVisible()
    }
  })

  test('Step expands the search tree one node at a time', async ({ page }) => {
    const chip = page.locator('.bb-chip')
    const nodes = page.locator('.bb-statgrid').locator('div', { hasText: /^nodes/ }).locator('.bb-mono')

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(chip).toContainText('Expanded')
    await expect(nodes).toHaveText('2')

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(chip).toContainText(/Expanded|Leaf|Pruned|Backtracked/)
  })

  test('Run completes the search with the optimal tour; Reset restores', async ({ page }) => {
    const chip = page.locator('.bb-chip')

    await page.getByRole('button', { name: 'Run' }).click()
    await expect(chip).toContainText('Done — optimal tour', { timeout: 30_000 })

    await page.getByRole('button', { name: 'Reset' }).click()
    await expect(chip).toContainText('search tree grows')
  })

  test('Back restores the previous step', async ({ page }) => {
    const step = page.locator('.bb-statgrid').locator('div', { hasText: /^step/ }).locator('.bb-mono')

    await page.getByRole('button', { name: 'Step' }).click()
    await page.getByRole('button', { name: 'Step' }).click()
    const after = await step.textContent()

    await page.getByRole('button', { name: 'Back' }).click()
    await expect(step).not.toHaveText(after ?? '')
  })

  test('Pause stops the animation mid-run', async ({ page }) => {
    const step = page.locator('.bb-statgrid').locator('div', { hasText: /^step/ }).locator('.bb-mono')

    await page.getByRole('button', { name: 'Run' }).click()
    await page.getByRole('button', { name: 'Pause' }).click()
    const paused = await step.textContent()
    await page.waitForTimeout(600)
    await expect(step).toHaveText(paused ?? '')
  })

  test('scenario buttons restart the run', async ({ page }) => {
    const step = page.locator('.bb-statgrid').locator('div', { hasText: /^step/ }).locator('.bb-mono')

    await page.getByRole('button', { name: 'Step' }).click()
    await expect(step).not.toHaveText('0')

    await page.getByRole('button', { name: 'Good bound' }).click()
    await expect(step).toHaveText('0')
    await page.getByRole('button', { name: 'Step' }).click()
    await expect(page.locator('.bb-chip')).toContainText('Expanded')
  })

  test('clicking a tree node pins it in the info panel', async ({ page }) => {
    await page.getByRole('button', { name: 'Step' }).click()
    const node = page.locator('.bb-node').first()
    await node.click()
    await expect(page.locator('.bb-nodeinfo')).toContainText('status')
  })
})

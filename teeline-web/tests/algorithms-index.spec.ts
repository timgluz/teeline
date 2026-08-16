import { test, expect } from '@playwright/test'

// E2E for the algorithms index page and the topbar navigation change:
// the old "Algorithms" drop-down is now a plain link to /algorithms/.
test.describe('algorithms index & topbar nav', () => {
  test('the topbar links to the algorithms index', async ({ page }) => {
    await page.goto('/')
    const link = page.getByRole('link', { name: 'Algorithms', exact: true })
    await expect(link).toBeVisible()
    await expect(link).toHaveAttribute('href', '/algorithms/')

    await link.click()
    await expect(page).toHaveURL(/\/algorithms\/$/)
    await expect(page.getByRole('heading', { level: 1 })).toHaveText('Algorithms')
  })

  test('the index lists the solver groups with explainer links', async ({ page }) => {
    await page.goto('/algorithms/')
    // one "▶ interactive" link per explainer (21 at the time of writing)
    await expect(page.getByRole('link', { name: '▶ interactive' })).toHaveCount(21)
    // a known solver doc link is present
    await expect(page.getByRole('link', { name: 'Christofides' })).toBeVisible()
  })

  test('an explainer link from the index opens the explainer page', async ({ page }) => {
    await page.goto('/algorithms/')
    const christofidesRow = page.getByRole('row').filter({ hasText: 'Christofides' })
    await christofidesRow.getByRole('link', { name: '▶ interactive' }).click()
    await expect(page).toHaveURL(/\/algorithms\/christofides\/explainer\/$/)
    await expect(page.locator('.chr-root')).toBeVisible({ timeout: 10_000 })
  })
})

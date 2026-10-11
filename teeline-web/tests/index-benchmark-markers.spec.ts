import { expect, test, type Page } from '@playwright/test'

// A manifest naming exactly the given ids, so the assertions do not depend on what is published.
function manifest(datasets: string[], solvers: string[]): string {
  return JSON.stringify({
    schema_version: 2,
    generated_at: '2026-10-11T12:05:56Z',
    tier: 'a',
    budgets: { local: { seconds: 300, agreed: false } },
    solvers: solvers.map((id) => ({ id, family: 'local' })),
    problems: datasets.map((id) => ({ id, cities: 52 })),
  })
}

/** Route the manifest, and collect every request for it. */
async function serveManifest(
  page: Page,
  body: string | null,
): Promise<string[]> {
  const requests: string[] = []
  page.on('request', (request) => {
    if (request.url().includes('/bench/v2/index.json'))
      requests.push(request.url())
  })
  await page.route('**/bench/v2/index.json', (route) =>
    body === null
      ? route.fulfill({ status: 500, contentType: 'text/plain', body: 'boom' })
      : route.fulfill({
          status: 200,
          contentType: 'application/json',
          body,
        }),
  )
  return requests
}

test.describe('index benchmark markers', () => {
  test('marks the problem rows the manifest publishes, and only those', async ({
    page,
  }) => {
    const requests = await serveManifest(
      page,
      manifest(['berlin52', 'att48'], []),
    )
    await page.goto('/problems/')

    const marked = page.locator(
      'tr[data-dataset="berlin52"] [data-bench-marker]',
    )
    await expect(marked).toBeVisible()
    await expect(marked.locator('a')).toHaveAttribute(
      'title',
      'It has benchmark results',
    )
    await expect(marked.locator('a')).toHaveAttribute(
      'href',
      '/problems/berlin52/#results',
    )
    await expect(
      page.locator('tr[data-dataset="att48"] [data-bench-marker]'),
    ).toBeVisible()
    // A row the manifest does not name stays unmarked rather than claiming no results.
    await expect(
      page.locator('tr[data-dataset="eil76"] [data-bench-marker]'),
    ).toBeHidden()

    await expect(page.locator('[data-bench-count-text]')).toHaveText(
      /^2 of \d+ instances have published benchmark results$/,
    )
    // One request for the page, not one per row: there are over a hundred rows.
    expect(requests).toHaveLength(1)
  })

  test('marks the solver rows the manifest publishes', async ({ page }) => {
    await serveManifest(page, manifest([], ['lk', '2opt']))
    await page.goto('/algorithms/')

    const marked = page.locator('tr[data-solver="lk"] [data-bench-marker]')
    await expect(marked).toBeVisible()
    await expect(marked.locator('a')).toHaveAttribute(
      'href',
      '/algorithms/lk/#benchmarks',
    )
    await expect(
      page.locator('tr[data-solver="aco"] [data-bench-marker]'),
    ).toBeHidden()
    await expect(page.locator('[data-bench-count-text]')).toHaveText(
      /^2 of \d+ solvers have published benchmark results$/,
    )
  })

  test('claims nothing when the manifest cannot be read', async ({ page }) => {
    await serveManifest(page, null)
    await page.goto('/problems/')

    // The list is untouched; only the claim is withheld.
    await expect(page.locator('tr[data-dataset="berlin52"]')).toBeVisible()
    await expect(page.locator('[data-bench-marker]:visible')).toHaveCount(0)
    await expect(page.locator('[data-bench-count]')).toBeHidden()
  })

  test.describe('without JavaScript', () => {
    test.use({ javaScriptEnabled: false })

    test('still lists every row, and claims nothing', async ({ page }) => {
      await page.goto('/problems/')

      expect(await page.locator('tr[data-dataset]').count()).toBeGreaterThan(
        100,
      )
      await expect(page.locator('tr[data-dataset="berlin52"]')).toBeVisible()
      await expect(page.locator('[data-bench-marker]:visible')).toHaveCount(0)
    })
  })
})

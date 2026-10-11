import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { expect, test, type Page } from '@playwright/test'

// Routes are fulfilled from the committed fixtures, so this spec asserts what the page
// renders without depending on R2 being reachable. The fixtures are the same ones the unit
// tests use: a real shard (mixed gap / no-gap rows) and a synthetic one covering a cell
// whose every run was killed.
function fixture(name: string): string {
  return readFileSync(
    fileURLToPath(new URL(`../src/lib/__fixtures__/${name}`, import.meta.url)),
    'utf8',
  )
}

const INDEX = JSON.stringify({
  schema_version: 2,
  generated_at: '2026-10-11T11:00:18Z',
  tier: 'a',
  budgets: {
    exact: { seconds: 60, agreed: false },
    local: { seconds: 300, agreed: false },
    meta: { seconds: 300, agreed: true },
  },
  solvers: [],
  problems: [],
})

async function fulfil(page: Page, body: string): Promise<void> {
  await page.route('**/bench/v2/index.json', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: INDEX,
    }),
  )
  await page.route('**/bench/v2/algorithms/*.json', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body }),
  )
}

// The section is a `client:visible` island at the foot of a docs page, so it only hydrates
// (and only fetches) once it is scrolled into view.
async function scrollToSection(page: Page): Promise<void> {
  await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight))
}

test.describe('algorithm benchmark section', () => {
  test('renders the table, the chart and provenance from a published shard', async ({
    page,
  }) => {
    await fulfil(page, fixture('algorithm-2opt.json'))
    await page.goto('/algorithms/2opt/')
    await scrollToSection(page)

    const section = page.locator(
      'section[aria-labelledby="benchmarks-heading"]',
    )
    await expect(section).toBeVisible()
    await expect(
      section.getByRole('heading', { name: 'Benchmarks' }),
    ).toBeVisible()

    // The allowance is stated next to the numbers, and marked provisional when it is not
    // settled by a decision.
    await expect(section).toContainText('local budget of 300 s (provisional)')

    // A gap column exists because some rows have an optimum, and carries the published
    // median rather than a recomputed one.
    await expect(
      section.getByRole('columnheader', { name: 'Gap (median)' }),
    ).toBeVisible()
    await expect(section).toContainText('15.25%')

    // berlin52 has no published optimum: it is listed, and its gap is a dash rather than
    // a zero that would read as "reached the optimum".
    const berlin = section.locator('tbody tr', { hasText: 'Berlin 52' })
    await expect(berlin).toBeVisible()
    await expect(berlin).toContainText('—')

    // Provenance names the commit the numbers came from, not the manifest's date.
    await expect(section).toContainText('e4588d6f (jsonl)')

    // The chart is drawn and says what it plots and what it left out.
    await expect(section.locator('svg[role="img"]')).toBeVisible()
    await expect(section.locator('figcaption')).toContainText(
      'one point per instance',
    )
    await expect(section.locator('figcaption')).toContainText(
      'not plotted — a gap and a tour length are not the same axis',
    )
  })

  test('renders a killed cell as lost runs rather than as a result', async ({
    page,
  }) => {
    await fulfil(page, fixture('algorithm-synthetic.json'))
    await page.goto('/algorithms/aco/')
    await scrollToSection(page)

    const section = page.locator(
      'section[aria-labelledby="benchmarks-heading"]',
    )
    await expect(section).toBeVisible()
    await expect(section).toContainText('5 timeouts, 1 error')
    // aco's tours vary between runs, so the spread is shown rather than one number.
    await expect(
      section.getByRole('columnheader', { name: 'Gap (best)' }),
    ).toBeVisible()
    await expect(
      section.getByRole('columnheader', { name: 'Tour (best–worst)' }),
    ).toBeVisible()
    // Its provenance is publish-time only, which the footer says out loud.
    await expect(section).toContainText('not the measured binary')
    // No environment was recorded, and none is invented.
    await expect(section).toContainText('not recorded')
  })

  test('renders nothing, and no error, for a solver with no published shard', async ({
    page,
  }) => {
    await page.route('**/bench/v2/index.json', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: INDEX,
      }),
    )
    await page.route('**/bench/v2/algorithms/*.json', (route) =>
      route.fulfill({
        status: 404,
        contentType: 'text/plain',
        body: 'not found',
      }),
    )

    // The island only fetches once scrolled into view, so wait for the 404 response rather
    // than for a fixed delay: absence has to mean "answered, then rendered nothing".
    const answered = page.waitForResponse((response) =>
      response.url().includes('/bench/v2/algorithms/lk.json'),
    )
    await page.goto('/algorithms/lk/')
    await scrollToSection(page)
    await answered
    await expect(page.getByRole('heading', { name: 'Benchmarks' })).toHaveCount(
      0,
    )
  })

  test('states the failure when the shard cannot be read', async ({ page }) => {
    await page.route('**/bench/v2/index.json', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: INDEX,
      }),
    )
    await page.route('**/bench/v2/algorithms/*.json', (route) =>
      route.fulfill({ status: 500, contentType: 'text/plain', body: 'boom' }),
    )

    await page.goto('/algorithms/2opt/')
    await scrollToSection(page)

    const section = page.locator(
      'section[aria-labelledby="benchmarks-heading"]',
    )
    await expect(section).toBeVisible()
    await expect(section).toContainText('could not be read')
    // A broken shard is not silently rendered as "no results yet".
    await expect(section).toContainText('HTTP 500')
  })
})

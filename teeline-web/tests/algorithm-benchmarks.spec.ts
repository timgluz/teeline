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

/** One shard per solver id, so a comparison sees two different solvers. */
async function fulfilPerSolver(
  page: Page,
  bySolver: Record<string, string>,
): Promise<void> {
  await page.route('**/bench/v2/index.json', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: INDEX,
    }),
  )
  await page.route('**/bench/v2/algorithms/*.json', (route) => {
    const id =
      route.request().url().split('/').pop()?.replace('.json', '') ?? ''
    const body = bySolver[id]
    return body === undefined
      ? route.fulfill({ status: 404, contentType: 'text/plain', body: 'none' })
      : route.fulfill({ status: 200, contentType: 'application/json', body })
  })
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

    // The prose carries the reading of the table, not the aggregates: those belong to the
    // summary row, and a figure in two places is a figure that can disagree with itself.
    const prose = section.locator('p').first()
    await expect(prose).toContainText('small fraction of its budget')
    await expect(prose).not.toContainText('Median gap')
    await expect(prose).not.toContainText('Median wall time')

    // And the summary row carries them, as medians of the per-instance medians.
    const summaryRow = section.locator('tfoot tr')
    await expect(summaryRow).toContainText('All 3 instances')
    await expect(summaryRow).toContainText('7.62%')
    // Runs and lost runs are totals in that row, not medians.
    await expect(summaryRow).toContainText('15')
    await expect(summaryRow.locator('td').last()).toHaveText('—')
  })

  test('the summary table jumps to the benchmark section, clear of the nav', async ({
    page,
  }) => {
    await fulfil(page, fixture('algorithm-2opt.json'))
    await page.goto('/algorithms/2opt/')

    // The row is part of the doc's markdown summary table, above the fold.
    const link = page.locator('a[href="#benchmarks"]').first()
    await expect(link).toBeVisible()

    // Hydrate the section first, so the layout has settled by the time we jump to it.
    const section = page.locator('#benchmarks')
    await section.scrollIntoViewIfNeeded()
    await page.waitForSelector('#benchmarks tfoot tr', { timeout: 20000 })
    await page.evaluate(() => window.scrollTo(0, 0))

    await link.click()
    await expect(page).toHaveURL(/#benchmarks$/)

    // scroll-padding-top exists because the topbar is sticky (56px): without it the heading
    // lands underneath the nav, which is the whole reason the offset is in global.css.
    const box = await section.boundingBox()
    expect(box?.y ?? 0).toBeGreaterThan(40)
    expect(box?.y ?? 0).toBeLessThan(200)
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

  test('offers its own shard and the manifest without fetching either', async ({
    page,
  }) => {
    await fulfil(page, fixture('algorithm-2opt.json'))

    const shardRequests: string[] = []
    page.on('request', (request) => {
      if (request.url().includes('/bench/v2/'))
        shardRequests.push(request.url())
    })

    await page.goto('/algorithms/2opt/')
    await scrollToSection(page)
    const section = page.locator(
      'section[aria-labelledby="benchmarks-heading"]',
    )
    await expect(section).toBeVisible()

    // Cross-origin `download` is ignored, so the label says "shard", not "download".
    await expect(
      section.locator('a[href$="/bench/v2/algorithms/2opt.json"]'),
    ).toHaveText(/shard/)
    await expect(
      section.locator('a[href$="/bench/v2/index.json"]'),
    ).toBeVisible()
    await expect(section.locator('a[href$="schema.md"]')).toBeVisible()

    // The links are rendered, not followed: the page loads exactly one algorithm shard, its own.
    const algorithmRequests = shardRequests.filter((url) =>
      url.includes('/algorithms/'),
    )
    expect(algorithmRequests).toHaveLength(1)
    expect(algorithmRequests[0]).toContain('/algorithms/2opt.json')
  })

  test('compares with another solver, keeping unlike things apart', async ({
    page,
  }) => {
    // 2opt is `local search`, branch_bound is `exact`; they share gr17 and nothing else.
    await fulfilPerSolver(page, {
      '2opt': fixture('algorithm-2opt.json'),
      branch_bound: fixture('algorithm-branch-bound.json'),
    })
    await page.goto('/algorithms/2opt/')
    await scrollToSection(page)

    const section = page.locator(
      'section[aria-labelledby="benchmarks-heading"]',
    )
    await expect(section).toBeVisible()
    // Wait for the table itself: the island renders a placeholder first, which has no columns.
    await expect(section.locator('tfoot tr')).toHaveCount(1)
    const columnsBefore = await section.locator('thead th').count()

    await section.locator('select').selectOption('branch_bound')
    // A second summary row is the signal that the comparison rendered.
    await expect(section.locator('tfoot tr')).toHaveCount(2)
    expect(await section.locator('thead th').count()).toBe(columnsBefore + 2)

    // The comparison is a link someone can share.
    await expect(page).toHaveURL(/[?&]compare=branch_bound/)

    // Different families: both allowances are stated, because their gaps are not like-for-like.
    await expect(section).toContainText('different families')

    // branch_bound publishes an `exact` config that 2opt does not: it must not be paired with
    // 2opt's budget run, so 2opt's own row stays the only thing in those columns.
    await expect(section.locator('tfoot tr').nth(1)).toContainText(
      'Branch & Bound',
    )

    // Never measured on berlin52 — reported as such, not as a zero or a blank.
    const berlin = section.locator('tbody tr', { hasText: 'Berlin 52' })
    await expect(berlin).toContainText('not measured')
    // ...and the chart counts it rather than drawing it.
    await expect(section.locator('figcaption')).toContainText(
      /no value to plot for \d+ of these instances/,
    )
    // Both solvers are legended, and both provenances are shown as their own builds.
    await expect(section).toContainText('Lin-Kernighan')
    await expect(section).toContainText('Branch & Bound')
    await expect(section).toContainText(
      'measured from its own build, not this solver',
    )
  })

  test('restores a comparison from ?compare=', async ({ page }) => {
    await fulfilPerSolver(page, {
      '2opt': fixture('algorithm-2opt.json'),
      branch_bound: fixture('algorithm-branch-bound.json'),
    })
    await page.goto('/algorithms/2opt/?compare=branch_bound')
    await scrollToSection(page)

    const section = page.locator(
      'section[aria-labelledby="benchmarks-heading"]',
    )
    await expect(section.locator('tfoot tr')).toHaveCount(2)
    await expect(section.locator('select')).toHaveValue('branch_bound')
  })
})

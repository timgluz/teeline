import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { expect, test, type Page } from '@playwright/test'

// Routes are fulfilled from the committed fixtures, so this asserts what the leaderboard
// renders without depending on R2 being reachable.
function fixture(name: string): string {
  return readFileSync(
    fileURLToPath(new URL(`../src/lib/__fixtures__/${name}`, import.meta.url)),
    'utf8',
  )
}

const INDEX = JSON.stringify({
  schema_version: 2,
  generated_at: '2026-10-11T12:05:56Z',
  tier: 'a',
  budgets: {
    constructive: { seconds: 30, agreed: false },
    exact: { seconds: 60, agreed: false },
    local: { seconds: 300, agreed: false },
    meta: { seconds: 300, agreed: true },
    utility: { seconds: 60, agreed: false },
  },
  solvers: [],
  problems: [],
})

async function openLeaderboard(page: Page, shard: string): Promise<void> {
  await page.route('**/bench/v2/index.json', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: INDEX,
    }),
  )
  await page.route('**/bench/v2/problems/*.json', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: shard,
    }),
  )
  await page.goto('/problems/berlin52/')
  // A client:visible island at the foot of a docs page: it hydrates once it is in view, which
  // is when the placeholder is replaced by the table.
  await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight))
  await page.waitForSelector('#results tbody tr', { timeout: 30_000 })
}

test.describe('problem page leaderboard', () => {
  test('ranks the board, states every family allowance and links the data', async ({
    page,
  }) => {
    await openLeaderboard(page, fixture('problem-berlin52.json'))

    const section = page.locator('#results')
    await expect(
      section.getByRole('heading', { name: 'Published results' }),
    ).toBeVisible()
    await expect(section).toContainText(
      'Ranked on median gap from the known optimum',
    )

    // A leaderboard mixes families, so the allowances are stated rather than assumed equal.
    await expect(section).toContainText('Local search 300 s (provisional)')
    await expect(section).toContainText(
      'cross-family gaps are not like-for-like',
    )

    // Ranked from 1, and the best of the runs is beside the median.
    await expect(
      section.getByRole('columnheader', { name: 'Gap (best)' }),
    ).toBeVisible()
    const firstRow = section.locator('tbody tr').first()
    await expect(firstRow).toContainText('Lin-Kernighan')
    await expect(firstRow.locator('td').first()).toHaveText('1')

    // Every row links to its solver's page, which is where that solver's own build is stated.
    await expect(section.locator('a[href="/algorithms/lk/"]')).toBeVisible()

    // The graph is the same ranking, drawn: one bar per ranked row.
    const bars = section.locator('svg[role="img"] rect')
    await expect(bars).toHaveCount(await section.locator('tbody tr').count())

    // The data behind the page, and the contract it follows.
    await expect(
      section.locator('a[href$="/bench/v2/problems/berlin52.json"]'),
    ).toBeVisible()
    await expect(
      section.locator('a[href$="/bench/v2/index.json"]'),
    ).toBeVisible()
    await expect(section.locator('a[href$="schema.md"]')).toBeVisible()
  })

  test('keeps two configs apart and ranks a row with no completed run last', async ({
    page,
  }) => {
    await openLeaderboard(page, fixture('problem-synthetic.json'))

    const section = page.locator('#results')
    // branch_bound publishes `budget` and `exact`: two rows, so the config column appears.
    await expect(
      section.getByRole('columnheader', { name: 'Config' }),
    ).toBeVisible()
    const branchBound = section.locator('tbody tr', {
      hasText: 'Branch & Bound',
    })
    await expect(branchBound).toHaveCount(2)

    // The hand-written shard's only all-timeout row: unranked, last, and reporting its losses
    // rather than a fabricated result.
    const last = section.locator('tbody tr').last()
    await expect(last).toContainText('5 timeouts')
    await expect(last.locator('td').first()).toHaveText('—')
    await expect(section.locator('figcaption')).toContainText('not plotted')
  })

  test('ranks on tour cost, with no gap column, when no optimum is published', async ({
    page,
  }) => {
    await openLeaderboard(page, fixture('problem-no-optimum.json'))

    const section = page.locator('#results')
    await expect(section).toContainText(
      'No optimum is published for this instance',
    )
    await expect(
      section.getByRole('columnheader', { name: 'Gap (median)' }),
    ).toHaveCount(0)
    await expect(
      section.getByRole('columnheader', { name: 'Tour (median)' }),
    ).toBeVisible()
    await expect(section.locator('figcaption')).toContainText(
      'not with optimal',
    )

    // The page's own frontmatter claims an optimum the shard does not have. Both are real, and
    // the page says so instead of showing one of them silently.
    await expect(section).toContainText(
      'This page states an optimal tour of 7,542',
    )
    await expect(section).toContainText('ranked against no optimum')
  })

  test('renders no results section for an instance with no published shard', async ({
    page,
  }) => {
    await page.route('**/bench/v2/index.json', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: INDEX,
      }),
    )
    await page.route('**/bench/v2/problems/*.json', (route) =>
      route.fulfill({ status: 404, contentType: 'text/plain', body: 'none' }),
    )

    await page.goto('/problems/berlin52/')
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight))
    await page.waitForTimeout(500)
    await expect(page.locator('#results')).toHaveCount(0)
    // The page around it is intact — the layout states the title in the document head.
    await expect(page).toHaveTitle(/Berlin 52/)
  })
})

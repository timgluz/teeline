import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { expect, test, type Page } from '@playwright/test'

// Routes are fulfilled from the committed fixtures, so this asserts what the landing chart
// renders without depending on R2 being reachable.
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
    constructive: { seconds: 30, agreed: false },
    exact: { seconds: 60, agreed: false },
    local: { seconds: 300, agreed: false },
    meta: { seconds: 300, agreed: true },
    utility: { seconds: 60, agreed: false },
  },
  solvers: [],
  problems: [],
})

async function openLanding(page: Page, shard: string | null): Promise<void> {
  await page.route('**/bench/v2/index.json', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: INDEX,
    }),
  )
  await page.route('**/bench/v2/problems/*.json', (route) =>
    shard === null
      ? route.fulfill({ status: 404, contentType: 'text/plain', body: 'nope' })
      : route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: shard,
        }),
  )
  await page.goto('/')
  // The chart is a client:visible island, so it hydrates only once it is actually in the
  // viewport. Scrolling to the bottom would leave it *above* the viewport — it sits between the
  // algorithms list and the footer — and the observer would never fire.
  const section = page.locator('#benchmarks')
  if ((await section.count()) > 0) await section.scrollIntoViewIfNeeded()
}

test.describe('landing benchmark chart', () => {
  test('plots gaps against the optimum and states the budgets', async ({
    page,
  }) => {
    await openLanding(page, fixture('problem-berlin52.json'))

    const section = page.locator('#benchmarks')
    await expect(section).toBeVisible()
    // The headline is count-free precisely so it cannot go stale the way "Twenty algorithms"
    // did when the published set grew.
    await expect(
      section.getByRole('heading', {
        name: 'Every measured solver, one chart.',
      }),
    ).toBeVisible()
    await expect(section.locator('svg[role="img"]')).toBeVisible()
    await expect(section).toContainText('8 solvers')
    // berlin52 has a published optimum since the EUC_2D fix, so this is the gap axis.
    await expect(section).toContainText('gap from the known optimum')
    // Cross-family gaps are what this chart shows, so the allowances are stated.
    await expect(section).toContainText('Budgets differ by family')
    await expect(section).toContainText('Local search 300 s (provisional)')
  })

  test('says so when the axis is tour length, and claims no budget for it', async ({
    page,
  }) => {
    // Kept from the publish before the EUC_2D fix: `optimal_cost: null` is a documented
    // contract case, and the current corpus happens to exercise it nowhere.
    await openLanding(page, fixture('problem-no-optimum.json'))

    const section = page.locator('#benchmarks')
    await expect(section).toBeVisible()
    await expect(section).toContainText('8 solvers')
    await expect(section).toContainText('no optimum is published')
    // No gap is plotted, so no budget is claimed for one.
    await expect(section).not.toContainText('Budgets differ by family')
  })

  test('keeps two configs apart and counts a solver that only timed out', async ({
    page,
  }) => {
    await openLanding(page, fixture('problem-synthetic.json'))

    const section = page.locator('#benchmarks')
    await expect(section).toBeVisible()
    await expect(section).toContainText('gap from the known optimum')
    await expect(section).toContainText('Metaheuristic 300 s')
    // branch_bound publishes two configs, so it is two points but one solver; gsa only timed
    // out, so it is counted rather than plotted.
    await expect(section).toContainText('6 solvers · 7 results')
    await expect(section).toContainText('not plotted: 1 did not finish')
  })

  test('renders no chart for an instance with no published shard', async ({
    page,
  }) => {
    await openLanding(page, null)
    await page.waitForTimeout(500)
    await expect(page.locator('#benchmarks')).toHaveCount(0)
    // And the page around it is intact.
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible()
  })
})

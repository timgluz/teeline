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

  test('shades each family cluster and hides it with the family filter', async ({
    page,
  }) => {
    await openLanding(page, fixture('problem-berlin52.json'))

    const section = page.locator('#benchmarks')
    await expect(section).toBeVisible()
    const hulls = section.locator('svg[role="img"] path[data-family-cluster]')
    // Local search and Metaheuristic each have three or more points in this fixture;
    // Constructive has one, so it gets no hull rather than a fake area.
    await expect(hulls).toHaveCount(2)
    await expect(hulls.first()).toHaveAttribute('data-family-cluster', 'local')
    await expect(hulls.nth(1)).toHaveAttribute('data-family-cluster', 'meta')
    await expect(section).toContainText('Shaded areas group solvers by family.')

    await section.getByRole('button', { name: 'Local search' }).click()
    await expect(hulls).toHaveCount(1)
    await expect(hulls.first()).toHaveAttribute('data-family-cluster', 'meta')
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
    // Shuffle leaves a gap an order of magnitude larger than the rest, so it is named above
    // the axis instead of stretching it.
    await expect(section).toContainText('The axis stops at 15.0%')
    await expect(section).toContainText('Random shuffle, the sanity check')
    await expect(section.locator('svg[role="img"] path')).toHaveCount(1)
  })

  test('filters the chart when a family chip is clicked', async ({ page }) => {
    await openLanding(page, fixture('problem-synthetic.json'))

    const section = page.locator('#benchmarks')
    await expect(section).toBeVisible()
    const circles = section.locator('svg[role="img"] circle')
    await expect(circles).toHaveCount(6)

    const meta = section.getByRole('button', { name: 'Metaheuristic' })
    await expect(meta).toHaveAttribute('aria-pressed', 'true')
    await meta.click()

    await expect(meta).toHaveAttribute('aria-pressed', 'false')
    await expect(circles).toHaveCount(5)
    await expect(section).toContainText('1 hidden')

    await section.getByRole('button', { name: 'show all' }).click()
    await expect(meta).toHaveAttribute('aria-pressed', 'true')
    await expect(circles).toHaveCount(6)
    await expect(section).not.toContainText('hidden')
  })

  test('keeps the last family with results visible', async ({ page }) => {
    await openLanding(page, fixture('problem-synthetic.json'))

    const section = page.locator('#benchmarks')
    await expect(section).toBeVisible()
    for (const name of ['Exact', 'Local search', 'Metaheuristic']) {
      await section.getByRole('button', { name }).click()
    }

    // Utility is the last family with a point, so its chip cannot empty the plot.
    await expect(
      section.getByRole('button', { name: 'Baseline' }),
    ).toBeDisabled()
    // mystery_solver has no family, so it is never filtered away.
    await expect(section.locator('svg[role="img"] circle')).toHaveCount(1)
  })

  test('shows the solver, runtime and gap when a point is hovered', async ({
    page,
  }) => {
    await openLanding(page, fixture('problem-berlin52.json'))

    const section = page.locator('#benchmarks')
    await expect(section).toBeVisible()
    await section.locator('svg[role="img"] circle').first().hover()

    const tooltip = page.getByRole('tooltip')
    await expect(tooltip).toBeVisible()
    await expect(tooltip).toContainText('runtime')
    await expect(tooltip).toContainText('gap')
    await expect(tooltip).toContainText('Local search')
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

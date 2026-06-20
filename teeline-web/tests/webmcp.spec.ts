import { test, expect } from '@playwright/test'

const SMALL_TSP = `NAME: test4
TYPE: TSP
COMMENT: 4-city smoke test
DIMENSION: 4
EDGE_WEIGHT_TYPE: EUC_2D
NODE_COORD_SECTION
1 0.0 0.0
2 1.0 0.0
3 1.0 1.0
4 0.0 1.0
EOF`

// Simulate a WebMCP agent submit event.
// Chrome's real WebMCP agent adds `agentInvoked: true` and `respondWith()` to
// the SubmitEvent. We replicate that here so the wire functions in webmcp.ts
// are exercised without an actual AI agent present.
async function invokeWebMCPTool(
  page: import('@playwright/test').Page,
  formId: string,
  fields: Record<string, string> = {},
): Promise<unknown> {
  return page.evaluate(
    async ({ formId, fields }) => {
      const form = document.getElementById(formId) as HTMLFormElement | null
      if (!form) throw new Error(`form #${formId} not found`)

      // Fill fields by name
      for (const [name, value] of Object.entries(fields)) {
        const el = form.elements.namedItem(name) as HTMLInputElement | HTMLTextAreaElement | null
        if (el) el.value = value
      }

      return new Promise((resolve, reject) => {
        const event = new Event('submit', { bubbles: true, cancelable: true }) as SubmitEvent & {
          agentInvoked: boolean
          respondWith(p: Promise<unknown>): void
        }
        event.agentInvoked = true
        event.respondWith = (promise: Promise<unknown>) => {
          promise.then(resolve).catch(reject)
        }
        form.dispatchEvent(event)
      })
    },
    { formId, fields },
  )
}

// DOM test — forms are in the static HTML; no WASM init needed
test('4 WebMCP forms are present in the DOM with toolname attributes', async ({ page }) => {
  await page.goto('/')
  const forms = await page.locator('form[toolname]').all()
  expect(forms).toHaveLength(4)

  const names = await Promise.all(forms.map(f => f.getAttribute('toolname')))
  expect(names).toContain('solveTSP')
  expect(names).toContain('listAlgorithms')
  expect(names).toContain('parseProblem')
  expect(names).toContain('compareTours')
})

// Functional tests — WASM must be ready so initWebMCP has wired the forms
test.describe('WebMCP tool invocation', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    await page.waitForSelector('.status-dot--ready', { timeout: 20_000 })
  })

  test('listAlgorithms returns an array of algorithm objects', async ({ page }) => {
    const result = await invokeWebMCPTool(page, 'webmcp-list-algorithms') as Array<{ id: string; name: string }>
    expect(Array.isArray(result)).toBe(true)
    expect(result.length).toBeGreaterThan(0)
    const ids = result.map(a => a.id)
    expect(ids).toContain('nn')
    expect(ids).toContain('lk')
  })

  test('parseProblem returns city list for valid TSPLIB input', async ({ page }) => {
    const result = await invokeWebMCPTool(page, 'webmcp-parse', { problem: SMALL_TSP }) as {
      name: string
      cities: Array<{ id: number; x: number; y: number }>
    }
    expect(result.name).toBe('test4')
    expect(result.cities).toHaveLength(4)
    expect(result.cities[0]).toMatchObject({ id: 1, x: 0, y: 0 })
  })

  test('solveTSP returns tour and cost for valid input', async ({ page }) => {
    const result = await invokeWebMCPTool(page, 'webmcp-solve', {
      problem: SMALL_TSP,
      algorithm: 'nn',
    }) as { total: number; route: number[] }
    expect(typeof result.total).toBe('number')
    expect(result.total).toBeGreaterThan(0)
    expect(result.route).toHaveLength(4)
  })

  test('compareTours returns gap stats for matching city sets', async ({ page }) => {
    const cities = [
      { id: 1, x: 0, y: 0 },
      { id: 2, x: 1, y: 0 },
      { id: 3, x: 1, y: 1 },
      { id: 4, x: 0, y: 1 },
    ]
    const result = await invokeWebMCPTool(page, 'webmcp-compare', {
      solver_route: JSON.stringify([1, 2, 3, 4]),
      opt_route: JSON.stringify([1, 2, 3, 4]),
      cities: JSON.stringify(cities),
    }) as { gapPct: number; sharedEdges: number }
    expect(result.gapPct).toBe(0)
    expect(result.sharedEdges).toBe(4)
  })
})

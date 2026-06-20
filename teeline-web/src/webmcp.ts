import type {
  WebMCPSolveResult,
  WebMCPAlgorithmsResult,
  WebMCPParseResult,
  CompareToursResult,
} from './worker'
import type { SolveOptions } from './solver-options'

interface AgentSubmitEvent extends SubmitEvent {
  readonly agentInvoked: boolean
  respondWith(value: Promise<unknown>): void
}

export function initWebMCP(worker: Worker): void {
  const token = import.meta.env.WEBMCP_ORIGIN_TOKEN
  if (token) {
    const meta = document.createElement('meta')
    meta.httpEquiv = 'origin-trial'
    meta.content = token as string
    document.head.appendChild(meta)
  }
  wireSolveTSP(worker)
  wireListAlgorithms(worker)
  wireParseProblem(worker)
  wireCompareTours(worker)
}

function wireSolveTSP(worker: Worker): void {
  const form = document.getElementById('webmcp-solve') as HTMLFormElement | null
  if (!form) return
  form.addEventListener('submit', (e) => {
    const ae = e as AgentSubmitEvent
    if (typeof ae.respondWith !== 'function') return
    e.preventDefault()
    ae.respondWith(new Promise((resolve, reject) => {
      const data = new FormData(form)
      const id = crypto.randomUUID()
      const options: Partial<SolveOptions> = {}
      const epochs = data.get('epochs')
      if (epochs !== null && epochs !== '') options.epochs = parseInt(epochs as string, 10)
      const maxTemp = data.get('max_temperature')
      if (maxTemp !== null && maxTemp !== '') options.maxTemperature = parseFloat(maxTemp as string)
      const mutProb = data.get('mutation_probability')
      if (mutProb !== null && mutProb !== '') options.mutationProbability = parseFloat(mutProb as string)
      const nNearest = data.get('n_nearest')
      if (nNearest !== null && nNearest !== '') options.nNearest = parseInt(nNearest as string, 10)

      const handler = (msg: MessageEvent) => {
        const res = msg.data as WebMCPSolveResult
        if (res.type !== 'webmcp-result' || res.id !== id) return
        worker.removeEventListener('message', handler)
        if (res.error) reject(new Error(res.error))
        else resolve(res.solution)
      }
      worker.addEventListener('message', handler)
      worker.postMessage({
        type: 'webmcp-solve',
        id,
        solver: (data.get('algorithm') as string | null) ?? 'lk',
        input: data.get('problem') as string,
        options,
      })
    }))
  })
}

function wireListAlgorithms(worker: Worker): void {
  const form = document.getElementById('webmcp-list-algorithms') as HTMLFormElement | null
  if (!form) return
  form.addEventListener('submit', (e) => {
    const ae = e as AgentSubmitEvent
    if (typeof ae.respondWith !== 'function') return
    e.preventDefault()
    ae.respondWith(new Promise((resolve, reject) => {
      const id = crypto.randomUUID()
      const handler = (msg: MessageEvent) => {
        const res = msg.data as WebMCPAlgorithmsResult
        if (res.type !== 'webmcp-algorithms' || res.id !== id) return
        worker.removeEventListener('message', handler)
        if (res.error) reject(new Error(res.error))
        else resolve(res.algorithms)
      }
      worker.addEventListener('message', handler)
      worker.postMessage({ type: 'webmcp-list-algorithms', id })
    }))
  })
}

function wireParseProblem(worker: Worker): void {
  const form = document.getElementById('webmcp-parse') as HTMLFormElement | null
  if (!form) return
  form.addEventListener('submit', (e) => {
    const ae = e as AgentSubmitEvent
    if (typeof ae.respondWith !== 'function') return
    e.preventDefault()
    ae.respondWith(new Promise((resolve, reject) => {
      const data = new FormData(form)
      const id = crypto.randomUUID()
      const handler = (msg: MessageEvent) => {
        const res = msg.data as WebMCPParseResult
        if (res.type !== 'webmcp-parsed' || res.id !== id) return
        worker.removeEventListener('message', handler)
        if (res.error) reject(new Error(res.error))
        else resolve(res.problem)
      }
      worker.addEventListener('message', handler)
      worker.postMessage({ type: 'webmcp-parse', id, input: data.get('problem') as string })
    }))
  })
}

function wireCompareTours(worker: Worker): void {
  const form = document.getElementById('webmcp-compare') as HTMLFormElement | null
  if (!form) return
  form.addEventListener('submit', (e) => {
    const ae = e as AgentSubmitEvent
    if (typeof ae.respondWith !== 'function') return
    e.preventDefault()
    ae.respondWith(new Promise((resolve, reject) => {
      const data = new FormData(form)
      const id = crypto.randomUUID()
      const handler = (msg: MessageEvent) => {
        const res = msg.data as CompareToursResult
        if (res.type !== 'compare-tours-result' || res.id !== id) return
        worker.removeEventListener('message', handler)
        if (res.error) reject(new Error(res.error))
        else resolve(res.stats)
      }
      worker.addEventListener('message', handler)
      worker.postMessage({
        type: 'compare-tours',
        id,
        solverRoute: JSON.parse(data.get('solver_route') as string) as number[],
        optRoute: JSON.parse(data.get('opt_route') as string) as number[],
        cities: JSON.parse(data.get('cities') as string) as Array<{ id: number; x: number; y: number }>,
      })
    }))
  })
}

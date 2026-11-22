import { useState, useEffect, useMemo, useCallback } from 'preact/hooks'
import {
  CITIES,
  N_CITIES,
  PARENT_A,
  PARENT_B,
  PINNED_CYCLE,
  DEFAULT_SEED,
  applyESet,
  bestTwoOpt,
  edgeKey,
  makeEdge,
  selectRand,
  simulate,
  tourEdgeKeys,
  unionEdges,
  type Edge,
  type Frame,
  type Simulation,
  type TwoOptMove,
} from './eax-algo'

const STAGES = [
  { id: 1, name: 'Compare Parents' },
  { id: 2, name: 'Union Graph' },
  { id: 3, name: 'AB-Cycles' },
  { id: 4, name: 'Select E-set' },
  { id: 5, name: 'Apply E-set' },
  { id: 6, name: 'Patch Subtours' },
] as const

const STAGE_NAMES = Object.fromEntries(
  STAGES.map((s) => [s.id, s.name]),
) as Record<number, string>
const CYCLE_COLORS = ['#7c3aed', '#0891b2', '#ca8a04', '#db2777']

function cycleColor(id: number): string {
  return CYCLE_COLORS[id % CYCLE_COLORS.length]
}

function componentColor(id: number): string {
  // Deliberately distinct from the AB-cycle badge palette (design decision 9).
  return ['#0d9488', '#6366f1', '#f59e0b', '#ec4899'][id % 4]
}

function centroid(vertices: number[]): { x: number; y: number } {
  const sum = vertices.reduce(
    (acc, v) => ({ x: acc.x + CITIES[v].x, y: acc.y + CITIES[v].y }),
    { x: 0, y: 0 },
  )
  return { x: sum.x / vertices.length, y: sum.y / vertices.length }
}

function tourEdgesOf(tour: number[]): Edge[] {
  return tour.map((v, i) => makeEdge(v, tour[(i + 1) % tour.length]))
}

function parsePair(key: string): Edge {
  const [u, v] = key.split('-').map(Number)
  return [u, v]
}

interface EdgeDraw {
  edge: Edge
  cls: string
  color?: string
  offset?: number
}

function buildEdges(frame: Frame, sim: Simulation): EdgeDraw[] {
  const { cycles, selected } = sim
  const selectedA = new Set<string>()
  const selectedB = new Set<string>()
  for (const id of selected) {
    for (const e of cycles[id].aEdges) selectedA.add(edgeKey(e))
    for (const e of cycles[id].bEdges) selectedB.add(edgeKey(e))
  }

  switch (frame.kind) {
    case 'parents': {
      const aSet = tourEdgeKeys(PARENT_A)
      const bSet = tourEdgeKeys(PARENT_B)
      const out: EdgeDraw[] = []
      for (const edge of tourEdgesOf(PARENT_A)) {
        const shared = bSet.has(edgeKey(edge))
        out.push({
          edge,
          cls: `eax-edge eax-edge-a${shared ? ' eax-edge-dashed' : ''}`,
          offset: 1.6,
        })
      }
      for (const edge of tourEdgesOf(PARENT_B)) {
        const shared = aSet.has(edgeKey(edge))
        out.push({
          edge,
          cls: `eax-edge eax-edge-b${shared ? ' eax-edge-dashed' : ''}`,
          offset: -1.6,
        })
      }
      return out
    }
    case 'union':
      return unionEdges(PARENT_A, PARENT_B).map((e) => ({
        edge: e.edge,
        cls:
          e.kind === 'shared'
            ? 'eax-edge eax-edge-shared'
            : e.kind === 'a-only'
              ? 'eax-edge eax-edge-a'
              : 'eax-edge eax-edge-b',
      }))
    case 'cycle':
    case 'cycles-summary':
    case 'select': {
      const cycleOf = new Map<string, number>()
      for (const c of cycles) {
        for (const e of [...c.aEdges, ...c.bEdges])
          cycleOf.set(edgeKey(e), c.id)
      }
      return unionEdges(PARENT_A, PARENT_B).map((e) => {
        const key = edgeKey(e.edge)
        const id = cycleOf.get(key)
        if (id === undefined)
          return { edge: e.edge, cls: 'eax-edge eax-edge-shared' }
        if (frame.kind === 'cycle' && id !== frame.cycleId) {
          return { edge: e.edge, cls: 'eax-edge eax-edge-dim' }
        }
        const active = frame.kind === 'select' ? selected.includes(id) : true
        return {
          edge: e.edge,
          cls: `eax-edge${active ? ' eax-edge-selected' : ' eax-edge-dim'}`,
          color: cycleColor(id),
        }
      })
    }
    case 'remove-a':
      return tourEdgesOf(PARENT_A).map((edge) => ({
        edge,
        cls: selectedA.has(edgeKey(edge))
          ? 'eax-edge eax-edge-removed'
          : 'eax-edge eax-edge-a eax-edge-dim',
      }))
    case 'add-b': {
      const result = applyESet(PARENT_A, cycles, selected)
      return [...result].map((key) => ({
        edge: parsePair(key),
        cls: selectedB.has(key)
          ? 'eax-edge eax-edge-added'
          : 'eax-edge eax-edge-keep',
      }))
    }
    case 'bridge': {
      const state = sim.states[frame.bridgeIndex ?? 0]
      const out: EdgeDraw[] = []
      const colorOf = new Map<number, number>()
      state.forEach((comp, i) => comp.forEach((v) => colorOf.set(v, i)))
      for (const comp of state) {
        for (let i = 0; i < comp.length; i++) {
          out.push({
            edge: makeEdge(comp[i], comp[(i + 1) % comp.length]),
            cls: 'eax-edge eax-edge-component',
            color: componentColor(colorOf.get(comp[i]) ?? 0),
          })
        }
      }
      const step = frame.bridge!
      for (const edge of step.removed)
        out.push({ edge, cls: 'eax-edge eax-edge-removed' })
      for (const edge of step.added)
        out.push({ edge, cls: 'eax-edge eax-edge-bridge' })
      return out
    }
    case 'no-patch': {
      const tour = sim.states[0][0]
      return tour.map((v, i) => ({
        edge: makeEdge(v, tour[(i + 1) % tour.length]),
        cls: 'eax-edge eax-edge-keep',
      }))
    }
    case 'final':
      return sim.child.map((v, i) => ({
        edge: makeEdge(v, sim.child[(i + 1) % sim.child.length]),
        cls: 'eax-edge eax-edge-child',
      }))
  }
}

function subtourRings(
  frame: Frame,
  sim: Simulation,
): Array<{ vertices: number[]; color: string }> {
  if (frame.kind === 'add-b') {
    return sim.subtours.map((comp, i) => ({
      vertices: comp,
      color: componentColor(i),
    }))
  }
  if (frame.kind === 'bridge') {
    return sim.states[frame.bridgeIndex ?? 0].map((comp, i) => ({
      vertices: comp,
      color: componentColor(i),
    }))
  }
  return []
}

function frameCallout(frame: Frame, sim: Simulation): string {
  const allSelected = sim.selected.length === sim.cycles.length
  if (frame.stage >= 5 && allSelected) {
    return 'All AB-cycles selected: Parent A becomes Parent B exactly — nothing to patch.'
  }
  switch (frame.kind) {
    case 'parents':
      return `Both parents are crossing-free; ${sim.stats.sharedEdges} of ${N_CITIES} edges are shared.`
    case 'union':
      return 'The union graph fades the shared backbone and keeps A-only (blue) and B-only (amber) edges.'
    case 'cycle':
      return `Cycle #${(frame.cycleId ?? 0) + 1}: ${sim.cycles[frame.cycleId ?? 0].vertices.length} cities, alternating A-only and B-only edges.`
    case 'cycles-summary':
      return `${sim.cycles.length} AB-cycles partition the symmetric difference.`
    case 'select':
      return 'Pick the E-set: 1-AB pins one cycle, Rand selects a seeded subset, or click a cycle.'
    case 'remove-a':
      return 'Apply the E-set: remove the selected A-edges (dashed red).'
    case 'add-b':
      return 'Add the selected B-edges. Every city keeps degree two, but the tour can split into subtours.'
    case 'bridge':
      return `Patch the smallest subtour with the cheapest bridge (Δ ${(frame.bridge?.delta ?? 0).toFixed(1)}).`
    case 'no-patch':
      return 'No patching needed — the selected E-set already forms one tour.'
    case 'final':
      return `Final child: ${sim.stats.costChild.toFixed(1)} vs Parent A ${sim.stats.costA.toFixed(1)} and Parent B ${sim.stats.costB.toFixed(1)}.`
  }
}

function Graph({
  frame,
  sim,
  preview,
}: {
  frame: Frame
  sim: Simulation
  preview: TwoOptMove | null
}) {
  const draws = useMemo(() => buildEdges(frame, sim), [frame, sim])
  const rings = subtourRings(frame, sim)
  const badges = useMemo(() => {
    if (frame.kind === 'cycle') {
      const c = sim.cycles[frame.cycleId ?? 0]
      const p = centroid(c.vertices)
      return [
        { x: p.x, y: p.y, label: `C${c.id + 1}`, color: cycleColor(c.id) },
      ]
    }
    if (frame.kind === 'cycles-summary' || frame.kind === 'select') {
      return sim.cycles.map((c) => {
        const p = centroid(c.vertices)
        return {
          x: p.x,
          y: p.y,
          label: `C${c.id + 1}`,
          color: cycleColor(c.id),
        }
      })
    }
    return []
  }, [frame, sim])

  return (
    <svg
      viewBox="0 0 300 300"
      className="eax-canvas"
      aria-hidden="true"
      focusable="false"
    >
      <rect x={0} y={0} width={300} height={300} className="eax-bg" />

      {rings.map((ring, i) =>
        ring.vertices.map((v) => (
          <circle
            key={`ring-${i}-${v}`}
            cx={CITIES[v].x}
            cy={CITIES[v].y}
            r={12}
            fill={ring.color}
            className="eax-ring"
          />
        )),
      )}

      {draws.map((d, i) => {
        const pa = CITIES[d.edge[0]]
        const pb = CITIES[d.edge[1]]
        let x1 = pa.x
        let y1 = pa.y
        let x2 = pb.x
        let y2 = pb.y
        if (d.offset) {
          const dx = pb.x - pa.x
          const dy = pb.y - pa.y
          const len = Math.hypot(dx, dy) || 1
          const nx = (-dy / len) * d.offset
          const ny = (dx / len) * d.offset
          x1 += nx
          y1 += ny
          x2 += nx
          y2 += ny
        }
        return (
          <line
            key={`e-${i}-${edgeKey(d.edge)}`}
            x1={x1}
            y1={y1}
            x2={x2}
            y2={y2}
            className={d.cls}
            style={d.color ? { stroke: d.color } : undefined}
          />
        )
      })}

      {frame.kind === 'final' && preview && (
        <>
          {preview.removed.map((e, i) => (
            <line
              key={`pr-${i}`}
              x1={CITIES[e[0]].x}
              y1={CITIES[e[0]].y}
              x2={CITIES[e[1]].x}
              y2={CITIES[e[1]].y}
              className="eax-edge eax-edge-removed"
            />
          ))}
          {preview.added.map((e, i) => (
            <line
              key={`pa-${i}`}
              x1={CITIES[e[0]].x}
              y1={CITIES[e[0]].y}
              x2={CITIES[e[1]].x}
              y2={CITIES[e[1]].y}
              className="eax-edge eax-edge-bridge"
            />
          ))}
        </>
      )}

      {badges.map((b, i) => (
        <g key={`badge-${i}`}>
          <circle
            cx={b.x}
            cy={b.y}
            r={11}
            fill={b.color}
            className="eax-badge-bg"
          />
          <text x={b.x} y={b.y + 3} className="eax-badge-text">
            {b.label}
          </text>
        </g>
      ))}

      {CITIES.map((p, i) => (
        <g key={`city-${i}`}>
          <circle cx={p.x} cy={p.y} r={5.5} className="eax-city" />
          <text x={p.x + 7} y={p.y - 6} className="eax-city-label">
            {i}
          </text>
        </g>
      ))}
    </svg>
  )
}

function CycleLegend({
  sim,
  onToggle,
}: {
  sim: Simulation
  onToggle: (id: number) => void
}) {
  return (
    <div className="eax-legend" role="group" aria-label="AB-cycles">
      {sim.cycles.map((c) => {
        const active = sim.selected.includes(c.id)
        return (
          <button
            key={c.id}
            type="button"
            className={`eax-cycle-btn${active ? ' eax-cycle-active' : ''}`}
            aria-pressed={active}
            onClick={() => onToggle(c.id)}
          >
            <span
              className="eax-swatch"
              style={{ background: cycleColor(c.id) }}
            />
            Cycle #{c.id + 1} · {c.vertices.length} cities
          </button>
        )
      })}
    </div>
  )
}

function StatsPanel({ sim }: { sim: Simulation }) {
  const s = sim.stats
  return (
    <div className="eax-statgrid">
      <div>
        <div className="eax-statlabel">shared edges</div>
        <div className="eax-mono">
          {s.sharedEdges}/{N_CITIES} ({Math.round(s.sharedRatio * 100)}%)
        </div>
      </div>
      <div>
        <div className="eax-statlabel">AB-cycles</div>
        <div className="eax-mono">
          {s.cycleCount} · {s.cycleLengths.join(' / ')}
        </div>
      </div>
      <div>
        <div className="eax-statlabel">E-set</div>
        <div className="eax-mono">
          {s.eSetSize} cycle{s.eSetSize === 1 ? '' : 's'}
        </div>
      </div>
      <div>
        <div className="eax-statlabel">subtours</div>
        <div className="eax-mono">{s.subtours}</div>
      </div>
      <div>
        <div className="eax-statlabel">bridges</div>
        <div className="eax-mono">{s.bridges}</div>
      </div>
      <div>
        <div className="eax-statlabel">cost A / B / child</div>
        <div className="eax-mono">
          {s.costA.toFixed(1)} / {s.costB.toFixed(1)} / {s.costChild.toFixed(1)}
        </div>
      </div>
      <div className="eax-stat-wide">
        <div className="eax-statlabel">child edge inheritance</div>
        <div className="eax-mono">
          shared {s.inheritance.shared} · from A {s.inheritance.fromA} · from B{' '}
          {s.inheritance.fromB} · new {s.inheritance.new}
        </div>
      </div>
    </div>
  )
}

function SummaryTable({ sim }: { sim: Simulation }) {
  return (
    <table className="eax-table">
      <caption>AB-cycles and the selected E-set</caption>
      <thead>
        <tr>
          <th scope="col">Cycle</th>
          <th scope="col">Cities</th>
          <th scope="col">Selected</th>
        </tr>
      </thead>
      <tbody>
        {sim.cycles.map((c) => (
          <tr key={c.id}>
            <th scope="row">Cycle #{c.id + 1}</th>
            <td>{c.vertices.length}</td>
            <td>{sim.selected.includes(c.id) ? 'yes' : 'no'}</td>
          </tr>
        ))}
        <tr>
          <th scope="row">Subtours</th>
          <td>{sim.stats.subtours}</td>
          <td>
            {sim.stats.bridges} bridge{sim.stats.bridges === 1 ? '' : 's'}
          </td>
        </tr>
        <tr>
          <th scope="row">Child cost</th>
          <td>{sim.stats.costChild.toFixed(1)}</td>
          <td>vs A {sim.stats.costA.toFixed(1)}</td>
        </tr>
      </tbody>
    </table>
  )
}

export default function EaxExplainer() {
  const [selected, setSelected] = useState<number[]>([PINNED_CYCLE])
  const [strategy, setStrategy] = useState<'1-AB' | 'Rand' | 'manual'>('1-AB')
  const [seed, setSeed] = useState(DEFAULT_SEED)
  const [frameIndex, setFrameIndex] = useState(0)
  const [running, setRunning] = useState(false)
  const [previewOn, setPreviewOn] = useState(false)
  const [notice, setNotice] = useState('')

  const sim = useMemo(() => simulate(selected), [selected])
  const frames = sim.frames
  const safeIndex = Math.min(frameIndex, frames.length - 1)
  const frame = frames[safeIndex]
  const stage = frame.stage
  const preview = useMemo(() => bestTwoOpt(sim.child), [sim.child])

  const jumpToStage = useCallback(
    (stageId: number) => {
      const first = frames.findIndex((f) => f.stage === stageId)
      if (first >= 0) setFrameIndex(first)
    },
    [frames],
  )

  const pause = useCallback(() => setRunning(false), [])

  const applySelection = useCallback(
    (next: number[], nextStrategy: '1-AB' | 'Rand' | 'manual') => {
      setSelected(next)
      setStrategy(nextStrategy)
      setRunning(false)
      setNotice('')
      jumpToStage(4)
    },
    [jumpToStage],
  )

  const chooseOneAb = useCallback(
    () => applySelection([PINNED_CYCLE], '1-AB'),
    [applySelection],
  )

  const chooseRand = useCallback(
    (nextSeed: number = seed) => {
      setSeed(nextSeed)
      applySelection(selectRand(sim.cycles.length, nextSeed), 'Rand')
    },
    [applySelection, seed, sim.cycles.length],
  )

  const reshuffle = useCallback(
    () => chooseRand((seed + 1) % 1_000_000),
    [chooseRand, seed],
  )

  const toggleCycle = useCallback(
    (id: number) => {
      setRunning(false)
      jumpToStage(4)
      setStrategy('manual')
      setSelected((cur) => {
        if (cur.includes(id)) {
          if (cur.length === 1) {
            setNotice('At least one AB-cycle is required for a crossover.')
            return cur
          }
          setNotice('')
          return cur.filter((x) => x !== id)
        }
        setNotice('')
        return [...cur, id].sort((a, b) => a - b)
      })
    },
    [jumpToStage],
  )

  const stepForward = useCallback(() => {
    setFrameIndex((i) => Math.min(i + 1, frames.length - 1))
    setRunning(false)
  }, [frames.length])

  const stepBack = useCallback(() => {
    setFrameIndex((i) => Math.max(i - 1, 0))
    setRunning(false)
  }, [])

  const run = useCallback(() => {
    if (running) {
      setRunning(false)
      return
    }
    setFrameIndex((i) => (i >= frames.length - 1 ? 0 : i))
    setRunning(true)
  }, [running, frames.length])

  const reset = useCallback(() => {
    setRunning(false)
    setFrameIndex(0)
  }, [])

  useEffect(() => {
    if (!running) return
    if (frameIndex >= frames.length - 1) {
      setRunning(false)
      return
    }
    const reduced = window.matchMedia?.(
      '(prefers-reduced-motion: reduce)',
    ).matches
    if (reduced) {
      setFrameIndex(frames.length - 1)
      setRunning(false)
      return
    }
    const id = window.setTimeout(() => setFrameIndex((i) => i + 1), 900)
    return () => window.clearTimeout(id)
  }, [running, frameIndex, frames.length])

  const stageName = STAGE_NAMES[stage] ?? ''

  return (
    <div className="eax-root">
      <style>{CSS}</style>

      <header className="eax-header">
        <div className="eax-eyebrow">teeline · algorithms/eax</div>
        <h2 className="eax-title">Memetic EAX — one crossover event</h2>
        <p className="eax-sub">
          EAX keeps almost all of the parents' edges and only rewires the
          symmetric difference. This storyboard is simplified for teaching: the
          default <code>1-AB</code> pins one AB-cycle, while the solver picks a
          random one each child.
        </p>
      </header>

      <nav className="eax-stages" aria-label="Storyboard stages">
        {STAGES.map((s) => (
          <button
            key={s.id}
            type="button"
            className={`eax-stage-btn${stage === s.id ? ' eax-stage-active' : ''}`}
            aria-pressed={stage === s.id}
            onClick={() => {
              pause()
              jumpToStage(s.id)
            }}
          >
            {s.id}. {s.name}
          </button>
        ))}
      </nav>

      <div className="eax-stage-line">
        <span className="eax-stage-label" aria-live="polite">
          {stageName}
        </span>
        <span className="eax-hint">
          frame {safeIndex + 1}/{frames.length}
        </span>
      </div>

      <div className="eax-callout">{frameCallout(frame, sim)}</div>

      <div className="eax-viz-row">
        <Graph frame={frame} sim={sim} preview={previewOn ? preview : null} />
        <aside className="eax-side">
          <CycleLegend sim={sim} onToggle={toggleCycle} />
          <div
            className="eax-strategy"
            role="group"
            aria-label="E-set strategy"
          >
            <button
              type="button"
              className={`eax-strategy-1ab${strategy === '1-AB' ? ' eax-strategy-active' : ''}`}
              aria-pressed={strategy === '1-AB'}
              onClick={chooseOneAb}
            >
              1-AB
            </button>
            <button
              type="button"
              className={`eax-strategy-rand${strategy === 'Rand' ? ' eax-strategy-active' : ''}`}
              aria-pressed={strategy === 'Rand'}
              onClick={() => chooseRand()}
            >
              Rand
            </button>
            <button type="button" className="eax-reshuffle" onClick={reshuffle}>
              Reshuffle
            </button>
            <span className="eax-seed eax-mono">seed {seed}</span>
          </div>
          {notice && <div className="eax-notice">{notice}</div>}
        </aside>
      </div>

      <div className="eax-legend-row">
        <span>
          <span className="eax-swatch eax-swatch-a" /> Parent A
        </span>
        <span>
          <span className="eax-swatch eax-swatch-b" /> Parent B
        </span>
        <span>
          <span className="eax-swatch eax-swatch-shared" /> shared
        </span>
        <span>
          <span className="eax-swatch eax-swatch-removed" /> removed
        </span>
        <span>
          <span className="eax-swatch eax-swatch-bridge" /> bridge
        </span>
        <span>
          <span className="eax-swatch eax-swatch-child" /> child
        </span>
      </div>

      <div className="eax-controls">
        <button
          type="button"
          className="eax-back"
          onClick={stepBack}
          disabled={safeIndex === 0}
        >
          ◀ Back
        </button>
        <button
          type="button"
          className="eax-step"
          onClick={stepForward}
          disabled={safeIndex >= frames.length - 1}
        >
          Step ▶
        </button>
        <button
          type="button"
          className={`eax-run${running ? ' eax-run-active' : ''}`}
          onClick={run}
        >
          {running ? '⏸ Pause' : '▶ Run'}
        </button>
        <button type="button" className="eax-reset" onClick={reset}>
          ↺ Reset
        </button>
        <button
          type="button"
          className={`eax-preview-btn${previewOn ? ' eax-preview-active' : ''}`}
          aria-pressed={previewOn}
          onClick={() => setPreviewOn((v) => !v)}
          disabled={!preview}
        >
          {preview ? 'Preview 2-opt' : 'Already 2-opt local'}
        </button>
      </div>

      {previewOn && preview && (
        <div className="eax-preview-note">
          <span className="eax-preview-removed eax-mono">
            − {preview.removed.map((e) => edgeKey(e)).join(', ')}
          </span>{' '}
          <span className="eax-preview-added eax-mono">
            + {preview.added.map((e) => edgeKey(e)).join(', ')}
          </span>{' '}
          <span className="eax-preview-delta eax-mono">
            Δ {preview.delta.toFixed(1)} ({preview.oldLength.toFixed(1)} →{' '}
            {preview.newLength.toFixed(1)})
          </span>
        </div>
      )}

      <StatsPanel sim={sim} />

      <p className="eax-summary">
        Stage {stage} of 6: {stageName}. Parent A costs{' '}
        {sim.stats.costA.toFixed(1)}, Parent B {sim.stats.costB.toFixed(1)}. The
        pinned fixture shares {sim.stats.sharedEdges} edges and has{' '}
        {sim.stats.cycleCount} AB-cycles. The E-set selects {sim.stats.eSetSize}{' '}
        cycle
        {sim.stats.eSetSize === 1 ? '' : 's'}, producing {sim.stats.subtours}{' '}
        subtour
        {sim.stats.subtours === 1 ? '' : 's'} and {sim.stats.bridges} bridge
        {sim.stats.bridges === 1 ? '' : 's'}. The child costs{' '}
        {sim.stats.costChild.toFixed(1)}.
      </p>
      <SummaryTable sim={sim} />

      <footer className="eax-footer">
        <span className="eax-mono">cities: {N_CITIES}</span>
        <span className="eax-mono">
          shared: {sim.stats.sharedEdges}/{N_CITIES}
        </span>
        <span className="eax-mono">
          E-set: {sim.selected.map((id) => `C${id + 1}`).join(' + ') || 'none'}
        </span>
      </footer>
    </div>
  )
}

const CSS = `
.eax-root {
  --eax-a: #3b82f6;
  --eax-b: #d97706;
  --eax-shared: #94a3b8;
  --eax-removed: #f87171;
  --eax-bridge: #16a34a;
  --eax-child: #6366f1;
  --eax-child-ink: #4338ca;
  --eax-bg: #ffffff;
  --eax-panel: #f6f8fa;
  --eax-line: #d0d7de;
  --eax-text: #1f2328;
  --eax-muted: #656d76;
  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  background: var(--eax-bg);
  color: var(--eax-text);
  border: 1px solid var(--eax-line);
  border-radius: 12px;
  padding: 20px;
  max-width: 900px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.eax-root * { box-sizing: border-box; }
.eax-mono { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.85em; }
.eax-header { margin-bottom: 2px; }
.eax-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--eax-child-ink); margin-bottom: 6px;
}
.eax-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.eax-sub { margin: 0; color: var(--eax-muted); font-size: 0.88rem; line-height: 1.55; }
.eax-sub code { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; background: rgba(67,56,202,0.1); color: var(--eax-child-ink); padding: 1px 4px; border-radius: 4px; }

.eax-stages { display: flex; flex-wrap: wrap; gap: 6px; }
.eax-stage-btn {
  background: var(--eax-panel); color: var(--eax-text); border: 1px solid var(--eax-line);
  border-radius: 999px; padding: 4px 10px; font-size: 0.78rem; cursor: pointer; font-family: inherit;
}
.eax-stage-btn:hover { border-color: var(--eax-child); }
.eax-stage-active { border-color: var(--eax-child); color: var(--eax-child-ink); font-weight: 600; }
.eax-stage-line { display: flex; justify-content: space-between; align-items: baseline; }
.eax-stage-label { font-weight: 650; font-size: 1rem; }
.eax-hint { color: var(--eax-muted); font-size: 0.75rem; }
.eax-callout { font-size: 0.82rem; color: var(--eax-muted); background: var(--eax-panel); border-left: 3px solid var(--eax-child); border-radius: 4px; padding: 6px 10px; }

.eax-viz-row { display: flex; gap: 12px; align-items: stretch; }
.eax-canvas {
  flex: 1; min-width: 0; width: 100%; border-radius: 8px; border: 1px solid var(--eax-line);
  background: var(--eax-panel);
}
.eax-side { width: 190px; flex-shrink: 0; display: flex; flex-direction: column; gap: 8px; }

.eax-bg { fill: var(--eax-panel); }
.eax-edge { fill: none; stroke: var(--eax-shared); stroke-width: 2; stroke-linecap: round; }
.eax-edge-a { stroke: var(--eax-a); }
.eax-edge-b { stroke: var(--eax-b); }
.eax-edge-shared { stroke: var(--eax-shared); stroke-dasharray: 5 4; }
.eax-edge-dashed { stroke-dasharray: 5 4; }
.eax-edge-dim { opacity: 0.22; }
.eax-edge-cycle-hit { stroke-width: 3.4; }
.eax-edge-selected { stroke-width: 3.2; }
.eax-edge-removed { stroke: var(--eax-removed); stroke-dasharray: 5 4; stroke-width: 2.6; }
.eax-edge-added { stroke: var(--eax-b); stroke-width: 3; }
.eax-edge-keep { stroke: var(--eax-shared); stroke-width: 2; }
.eax-edge-component { stroke-width: 2.6; }
.eax-edge-bridge { stroke: var(--eax-bridge); stroke-width: 3.4; }
.eax-edge-child { stroke: var(--eax-child); stroke-width: 3; }
.eax-ring { opacity: 0.16; }
.eax-city { fill: #ffffff; stroke: #334155; stroke-width: 1.4; }
.eax-city-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 8px; fill: #374151; stroke: white; stroke-width: 2.5; paint-order: stroke fill;
  pointer-events: none; user-select: none;
}
.eax-badge-bg { opacity: 0.92; }
.eax-badge-text {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 8.5px; font-weight: 700; fill: #ffffff; text-anchor: middle;
  pointer-events: none; user-select: none;
}

.eax-legend { display: flex; flex-direction: column; gap: 4px; }
.eax-cycle-btn {
  display: flex; align-items: center; gap: 6px; text-align: left;
  background: var(--eax-panel); border: 1px solid var(--eax-line); border-radius: 6px;
  padding: 5px 8px; font-size: 0.76rem; cursor: pointer; font-family: inherit; color: var(--eax-text);
}
.eax-cycle-btn:hover { border-color: var(--eax-child); }
.eax-cycle-active { border-color: var(--eax-child); font-weight: 600; }
.eax-swatch { width: 14px; height: 4px; border-radius: 2px; display: inline-block; flex-shrink: 0; }

.eax-strategy { display: flex; flex-wrap: wrap; gap: 4px; align-items: center; }
.eax-strategy button {
  background: var(--eax-panel); border: 1px solid var(--eax-line); border-radius: 6px;
  padding: 4px 8px; font-size: 0.76rem; cursor: pointer; font-family: inherit; color: var(--eax-text);
}
.eax-strategy-active { border-color: var(--eax-child); color: var(--eax-child-ink); font-weight: 600; }
.eax-seed { color: var(--eax-muted); font-size: 0.72rem; }
.eax-notice { font-size: 0.75rem; color: #b45309; background: #fffbeb; border: 1px solid #fde68a; border-radius: 6px; padding: 6px 8px; }

.eax-legend-row { display: flex; flex-wrap: wrap; gap: 14px; font-size: 0.8rem; color: var(--eax-muted); align-items: center; }
.eax-swatch-a { background: var(--eax-a); }
.eax-swatch-b { background: var(--eax-b); }
.eax-swatch-shared { background: var(--eax-shared); }
.eax-swatch-removed { background: var(--eax-removed); }
.eax-swatch-bridge { background: var(--eax-bridge); }
.eax-swatch-child { background: var(--eax-child); }

.eax-controls { display: flex; flex-wrap: wrap; gap: 8px; }
.eax-controls button {
  background: var(--eax-panel); color: var(--eax-text); border: 1px solid var(--eax-line);
  border-radius: 6px; padding: 6px 14px; font-size: 0.85rem; cursor: pointer; font-family: inherit;
}
.eax-controls button:hover:not(:disabled) { border-color: var(--eax-child); }
.eax-controls button:disabled { opacity: 0.45; cursor: default; }
.eax-run-active, .eax-preview-active { border-color: var(--eax-child); color: var(--eax-child-ink); }

.eax-preview-note { font-size: 0.8rem; display: flex; flex-wrap: wrap; gap: 10px; }
.eax-preview-removed { color: #b91c1c; }
.eax-preview-added { color: #15803d; }
.eax-preview-delta { color: var(--eax-muted); }

.eax-statgrid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
.eax-stat-wide { grid-column: span 3; }
@media (max-width: 600px) { .eax-statgrid { grid-template-columns: repeat(2, 1fr); } .eax-stat-wide { grid-column: span 2; } }
.eax-statlabel { font-size: 0.65rem; color: var(--eax-muted); text-transform: uppercase; letter-spacing: 0.05em; }

.eax-summary {
  position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden;
  clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0;
}
.eax-table { width: 100%; border-collapse: collapse; font-size: 0.78rem; }
.eax-table caption { text-align: left; color: var(--eax-muted); font-size: 0.72rem; padding-bottom: 4px; }
.eax-table th, .eax-table td { text-align: left; border-bottom: 1px solid var(--eax-line); padding: 4px 6px; }
.eax-table thead th { color: var(--eax-muted); font-weight: 600; }

.eax-footer { display: flex; flex-wrap: wrap; gap: 14px; padding-top: 8px; border-top: 1px solid var(--eax-line); color: var(--eax-muted); }

@media (max-width: 720px) {
  .eax-viz-row { flex-direction: column; }
  .eax-side { width: 100%; flex-direction: row; flex-wrap: wrap; }
  .eax-legend { flex-direction: row; flex-wrap: wrap; }
}
`

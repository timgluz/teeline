import { useState, useRef, useEffect, useCallback } from "preact/hooks"
import type { SomState, Phase } from "./som-algo"
import {
  CITIES, ALPHA0, SIGMA0, MAX_STEPS,
  makeInitState, stepOnce, neighbourRadiusPx, phaseLabel,
} from "./som-algo"

function polyPts(pts: [number, number][]): string {
  return pts.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(' ')
}

const PHASE_COLOR: Record<Phase, string> = {
  'init':        '#6b7280',
  'expanding':   '#7c3aed',
  'converging':  '#a855f7',
  'fine-tuning': '#c084fc',
  'done':        '#16a34a',
}

export default function SomExplainer() {
  const simRef = useRef<SomState>(makeInitState())

  const [neurons, setNeurons] = useState(simRef.current.neurons)
  const [bmu, setBmu] = useState<number | null>(null)
  const [neighbors, setNeighbors] = useState<number[]>([])
  const [lastCityIdx, setLastCityIdx] = useState<number | null>(null)
  const [phase, setPhase] = useState<Phase>('init')
  const [step, setStep] = useState(0)
  const [alpha, setAlpha] = useState(ALPHA0)
  const [sigma, setSigma] = useState(SIGMA0)
  const [tour, setTour] = useState<number[] | null>(null)
  const [lastTourLength, setLastTourLength] = useState<number | null>(null)
  const [running, setRunning] = useState(false)
  const [speed, setSpeed] = useState(5)

  const syncState = (s: SomState) => {
    setNeurons(s.neurons)
    setBmu(s.bmu)
    setNeighbors(s.neighbors)
    setLastCityIdx(s.lastCityIdx)
    setPhase(s.phase)
    setStep(s.step)
    setAlpha(s.alpha)
    setSigma(s.sigma)
    setTour(s.tour)
    setLastTourLength(s.lastTourLength)
  }

  const stepFn = useCallback(() => {
    const next = stepOnce(simRef.current)
    simRef.current = next
    syncState(next)
    if (next.phase === 'done') setRunning(false)
  }, [])

  const reinit = useCallback(() => {
    const s = makeInitState()
    simRef.current = s
    syncState(s)
    setRunning(false)
  }, [])

  const delay = Math.max(50, 660 - speed * 60)
  useEffect(() => {
    if (!running) return
    const id = setInterval(stepFn, delay)
    return () => clearInterval(id)
  }, [running, stepFn, delay])

  const bmuPos = bmu !== null ? neurons[bmu] : null
  const nbRadiusPx = bmuPos ? neighbourRadiusPx(sigma, neurons) : 0

  // Tour polyline: cities in ring order, closed
  const tourPts = tour !== null
    ? polyPts([...tour.map(i => CITIES[i]), CITIES[tour[0]]])
    : null

  // Neuron ring polyline: close the ring back to first neuron
  const neuronRingPts = polyPts([...neurons, neurons[0]])

  const isDone = phase === 'done'

  return (
    <div class="som-root">
      <style>{CSS}</style>

      <header class="som-header">
        <span class="som-emoji">🧠</span>
        <div>
          <h2 class="som-title">Kohonen SOM</h2>
          <p class="som-subtitle">Self-Organising Map — topology-preserving TSP heuristic</p>
        </div>
      </header>

      <div class="som-viz-row">
        {/* 300×300 SVG canvas */}
        <svg class="som-canvas" viewBox="0 0 300 300" aria-label="SOM visualisation">

          {/* 1. Tour overlay (only when done) */}
          {tourPts && (
            <polyline points={tourPts} class="som-tour" />
          )}

          {/* 2. Neuron ring polyline */}
          <polyline points={neuronRingPts} class="som-neuron-ring" />

          {/* 3. Neighbourhood radius circle */}
          {bmuPos && !isDone && (
            <circle
              cx={bmuPos[0].toFixed(1)}
              cy={bmuPos[1].toFixed(1)}
              r={nbRadiusPx.toFixed(1)}
              class="som-nb-circle"
            />
          )}

          {/* 4. Attraction line: last city → BMU */}
          {lastCityIdx !== null && bmuPos && !isDone && (
            <line
              x1={CITIES[lastCityIdx][0].toFixed(1)}
              y1={CITIES[lastCityIdx][1].toFixed(1)}
              x2={bmuPos[0].toFixed(1)}
              y2={bmuPos[1].toFixed(1)}
              class="som-attraction"
            />
          )}

          {/* 5. Active neighbour circles */}
          {!isDone && neighbors.map(ni => (
            <circle
              key={ni}
              cx={neurons[ni][0].toFixed(1)}
              cy={neurons[ni][1].toFixed(1)}
              r="5.5"
              class="som-nb-neuron"
            />
          ))}

          {/* 6. All neuron circles (BMU rendered larger) */}
          {neurons.map(([nx, ny], ni) => (
            <circle
              key={ni}
              cx={nx.toFixed(1)}
              cy={ny.toFixed(1)}
              r={ni === bmu ? "8" : "4"}
              class={ni === bmu ? 'som-neuron som-neuron-bmu' : 'som-neuron'}
            />
          ))}

          {/* 7. City circles (always on top) */}
          {CITIES.map(([cx, cy], ci) => (
            <circle
              key={ci}
              cx={cx.toFixed(1)}
              cy={cy.toFixed(1)}
              r="7"
              class={ci === lastCityIdx && !isDone ? 'som-city som-city-active' : 'som-city'}
            />
          ))}
        </svg>

        {/* Right panel */}
        <div class="som-panel">

          {/* Phase chip */}
          <div class="som-phase-chip" style={`border-color: ${PHASE_COLOR[phase]}`}>
            <span class="som-phase-dot" style={`background: ${PHASE_COLOR[phase]}`} />
            <span class="som-phase-text">{phaseLabel(phase, lastTourLength)}</span>
          </div>

          {/* Stats grid */}
          <div class="som-statgrid">
            <div class="som-stat"><span>Step</span><strong>{step}/{MAX_STEPS}</strong></div>
            <div class="som-stat"><span>α</span><strong>{alpha.toFixed(3)}</strong></div>
            <div class="som-stat"><span>σ</span><strong>{sigma.toFixed(2)}</strong></div>
            <div class="som-stat"><span>Tour</span><strong>{lastTourLength !== null ? lastTourLength.toFixed(0) : '—'}</strong></div>
          </div>

          {/* Progress bar */}
          <div class="som-progress-track" role="progressbar" aria-valuenow={step} aria-valuemax={MAX_STEPS}>
            <div class="som-progress-bar" style={`width: ${(Math.min(step, MAX_STEPS) / MAX_STEPS * 100).toFixed(1)}%`} />
          </div>

          {/* Speed slider */}
          <div class="som-config">
            <label class="som-label">
              Speed
              <input
                type="range" min="1" max="10" value={speed}
                onInput={e => setSpeed(Number((e.target as HTMLInputElement).value))}
              />
              <span>{speed}</span>
            </label>
          </div>

          {/* Control buttons */}
          <div class="som-controls">
            <button
              class="som-btn som-btn-step"
              onClick={stepFn}
              disabled={running || isDone}
            >Step</button>
            <button
              class="som-btn som-btn-run"
              onClick={() => setRunning(r => !r)}
              disabled={isDone}
            >{running ? 'Pause' : 'Run'}</button>
            <button class="som-btn som-btn-reset" onClick={reinit}>Reset</button>
          </div>
        </div>
      </div>

      {/* Legend */}
      <div class="som-legend">
        <span class="som-leg-item">
          <svg width="14" height="14"><circle cx="7" cy="7" r="6" fill="#f97316" /></svg>
          City
        </span>
        <span class="som-leg-item">
          <svg width="14" height="14"><circle cx="7" cy="7" r="5" fill="#7c3aed" /></svg>
          Neuron
        </span>
        <span class="som-leg-item">
          <svg width="14" height="14"><circle cx="7" cy="7" r="6" fill="#4f46e5" /></svg>
          BMU
        </span>
        <span class="som-leg-item">
          <svg width="14" height="14"><circle cx="7" cy="7" r="6" fill="none" stroke="#7c3aed" stroke-width="1.5" stroke-dasharray="3,2" /></svg>
          σ radius
        </span>
        <span class="som-leg-item">
          <svg width="18" height="4"><line x1="0" y1="2" x2="18" y2="2" stroke="#f97316" stroke-width="1.5" stroke-dasharray="4,2" /></svg>
          Attraction
        </span>
        <span class="som-leg-item">
          <svg width="18" height="4"><line x1="0" y1="2" x2="18" y2="2" stroke="#16a34a" stroke-width="2" /></svg>
          Tour
        </span>
      </div>

      <footer class="som-footer">
        12 cities · 18 neurons · η₀={ALPHA0} · σ₀={SIGMA0} · {MAX_STEPS} steps
      </footer>
    </div>
  )
}

const CSS = `
.som-root {
  font-family: system-ui, sans-serif;
  max-width: 860px;
}

.som-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1.25rem;
}
.som-emoji { font-size: 2rem; line-height: 1; }
.som-title { margin: 0; font-size: 1.25rem; }
.som-subtitle { margin: 0; color: #6b7280; font-size: 0.875rem; }

.som-viz-row {
  display: flex;
  gap: 1.5rem;
  align-items: flex-start;
  flex-wrap: wrap;
}

.som-canvas {
  width: 300px;
  height: 300px;
  flex-shrink: 0;
  background: #f9fafb;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
}

.som-city { fill: #f97316; stroke: #fff; stroke-width: 1.5; }
.som-city-active { fill: #ea580c; stroke: #7c3aed; stroke-width: 2.5; }

.som-neuron-ring { fill: none; stroke: #a78bfa; stroke-width: 1.2; opacity: 0.7; }
.som-neuron { fill: #7c3aed; stroke: #fff; stroke-width: 1; }
.som-neuron-bmu { fill: #4f46e5; stroke: #fff; stroke-width: 2; }
.som-nb-neuron { fill: #a78bfa; stroke: none; opacity: 0.6; }

.som-nb-circle {
  fill: none;
  stroke: #7c3aed;
  stroke-width: 1.5;
  stroke-dasharray: 5,3;
  opacity: 0.5;
}

.som-attraction {
  stroke: #f97316;
  stroke-width: 1.5;
  stroke-dasharray: 4,3;
  opacity: 0.7;
}

.som-tour {
  fill: none;
  stroke: #16a34a;
  stroke-width: 2;
  stroke-linejoin: round;
  stroke-linecap: round;
}

.som-panel {
  flex: 1;
  min-width: 240px;
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
}

.som-phase-chip {
  border: 1.5px solid #7c3aed;
  border-radius: 8px;
  padding: 0.5rem 0.75rem;
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  font-size: 0.8125rem;
  line-height: 1.4;
}
.som-phase-dot {
  width: 8px; height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  margin-top: 3px;
}
.som-phase-text { color: #374151; }

.som-statgrid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 0.25rem;
}
.som-stat {
  background: #f3f4f6;
  border-radius: 6px;
  padding: 0.375rem 0.5rem;
  text-align: center;
  font-size: 0.75rem;
}
.som-stat span { display: block; color: #6b7280; }
.som-stat strong { display: block; font-size: 0.875rem; }

.som-progress-track {
  height: 4px;
  background: #e5e7eb;
  border-radius: 2px;
  overflow: hidden;
}
.som-progress-bar {
  height: 100%;
  background: #7c3aed;
  transition: width 0.1s linear;
}

.som-config { display: flex; flex-direction: column; gap: 0.5rem; }
.som-label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.8125rem;
}
.som-label input[type=range] { flex: 1; }
.som-label span { min-width: 1.5rem; text-align: right; }

.som-controls { display: flex; gap: 0.5rem; }
.som-btn {
  flex: 1;
  padding: 0.4rem 0.75rem;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.875rem;
  font-weight: 500;
}
.som-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.som-btn-step { background: #f3f4f6; color: #374151; }
.som-btn-reset { background: #f3f4f6; color: #374151; }
.som-btn-run { background: #7c3aed; color: #fff; }
.som-btn-run:hover:not(:disabled) { background: #6d28d9; }

.som-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  margin-top: 1rem;
  font-size: 0.8125rem;
  color: #374151;
  align-items: center;
}
.som-leg-item {
  display: flex;
  align-items: center;
  gap: 0.375rem;
}

.som-footer {
  margin-top: 0.75rem;
  font-size: 0.75rem;
  color: #9ca3af;
}

@media (max-width: 600px) {
  .som-viz-row { flex-direction: column; }
  .som-canvas { width: 100%; height: auto; }
  .som-statgrid { grid-template-columns: repeat(2, 1fr); }
}
`

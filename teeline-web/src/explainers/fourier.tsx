import { useState, useRef, useEffect, useMemo, useCallback } from "preact/hooks";
import type { ComponentChildren } from "preact";

/**
 * FourierExplainer
 * ------------------------------------------------------------
 * Interactive walkthrough of teeline's Fourier-basis constructive
 * solver (docs/algorithms/fourier.md).
 *
 * Zero extra dependencies: plain React + inline SVG + a single
 * <style> block. Drop this file into teeline-web and render
 * <FourierExplainer /> anywhere.
 * ------------------------------------------------------------
 */

// ---------------------------------------------------------------
// Fixed demo instance: 15 "cities" laid out on a 300x300 canvas.
// Small enough to animate smoothly in-browser.
// ---------------------------------------------------------------
const CITIES: [number, number][] = [
  [40, 60], [80, 30], [140, 20], [200, 40], [250, 70], [270, 130],
  [250, 200], [200, 250], [140, 270], [80, 250], [40, 200], [20, 130],
  [150, 150], [100, 100], [200, 100],
];
const K_MAX = 4;
const N_COEFFS = 2 * K_MAX + 1; // index ki -> k = ki - K_MAX
const M = 72; // curve sampling resolution
const SCALE = 300; // canvas size; coeffs are computed in [0,1] space, then scaled

// ---------------------------------------------------------------
// Core math: γ(s) = Σ_{k=-K}^{K} c_k · e^{2πiks}, with c_k = a_k + i·b_k
// ---------------------------------------------------------------
function curvePoints(a: number[], b: number[], kActiveMax = K_MAX, m = M): [number, number][] {
  const pts: [number, number][] = new Array(m);
  for (let j = 0; j < m; j++) {
    const s = j / m;
    let x = 0, y = 0;
    for (let ki = 0; ki < N_COEFFS; ki++) {
      const k = ki - K_MAX;
      if (Math.abs(k) > kActiveMax) continue;
      const theta = 2 * Math.PI * k * s;
      const c = Math.cos(theta), si = Math.sin(theta);
      x += a[ki] * c - b[ki] * si;
      y += a[ki] * si + b[ki] * c;
    }
    pts[j] = [x * SCALE, y * SCALE];
  }
  return pts;
}

// init in [0,1] city-coordinate space
function initCoeffsNormalized(): { a: number[]; b: number[] } {
  const a = new Array(N_COEFFS).fill(0) as number[];
  const b = new Array(N_COEFFS).fill(0) as number[];
  const cities01 = CITIES.map(([x, y]) => [x / SCALE, y / SCALE] as [number, number]);
  let cx = 0, cy = 0;
  cities01.forEach(([x, y]) => { cx += x; cy += y; });
  cx /= cities01.length; cy /= cities01.length;
  let r = 0;
  cities01.forEach(([x, y]) => { r += Math.hypot(x - cx, y - cy); });
  r /= cities01.length;
  a[K_MAX] = cx; b[K_MAX] = cy;     // c_0 = centroid
  a[K_MAX + 1] = r / 2;             // c_1 = radius / 2
  return { a, b };
}

function energy(a: number[], b: number[], lambda: number, kActiveMax: number, cities01: [number, number][]): { eAttr: number; eTen: number; total: number } {
  const pts = curvePoints(a, b, kActiveMax).map(([x, y]) => [x / SCALE, y / SCALE] as [number, number]);
  let eAttr = 0;
  for (const [cx, cy] of cities01) {
    let best = Infinity;
    for (const [px, py] of pts) {
      const d = (cx - px) ** 2 + (cy - py) ** 2;
      if (d < best) best = d;
    }
    eAttr += best;
  }
  let eTen = 0;
  for (let ki = 0; ki < N_COEFFS; ki++) {
    const k = ki - K_MAX;
    eTen += (2 * Math.PI * k) ** 2 * (a[ki] ** 2 + b[ki] ** 2);
  }
  return { eAttr, eTen: lambda * eTen, total: eAttr + lambda * eTen };
}

function gradStep(a: number[], b: number[], kActiveMax: number, lambda: number, lr: number, cities01: [number, number][]): { a: number[]; b: number[] } {
  const pts01 = curvePoints(a, b, kActiveMax).map(([x, y]) => [x / SCALE, y / SCALE] as [number, number]);
  const gradA = new Array(N_COEFFS).fill(0) as number[];
  const gradB = new Array(N_COEFFS).fill(0) as number[];
  // attraction: each city pulls its nearest curve sample
  for (const [cx, cy] of cities01) {
    let best = Infinity, bestJ = 0;
    for (let j = 0; j < pts01.length; j++) {
      const [px, py] = pts01[j];
      const d = (cx - px) ** 2 + (cy - py) ** 2;
      if (d < best) { best = d; bestJ = j; }
    }
    const s = bestJ / pts01.length;
    const [px, py] = pts01[bestJ];
    const dx = cx - px, dy = cy - py;
    for (let ki = 0; ki < N_COEFFS; ki++) {
      const k = ki - K_MAX;
      if (Math.abs(k) > kActiveMax) continue;
      const theta = 2 * Math.PI * k * s;
      const cth = Math.cos(theta), sth = Math.sin(theta);
      gradA[ki] += -2 * dx * cth - 2 * dy * sth;
      gradB[ki] += 2 * dx * sth - 2 * dy * cth;
    }
  }
  // tension: penalise high-frequency modes
  for (let ki = 0; ki < N_COEFFS; ki++) {
    const k = ki - K_MAX;
    if (Math.abs(k) > kActiveMax) continue;
    const w = 2 * lambda * (2 * Math.PI * k) ** 2;
    gradA[ki] += w * a[ki];
    gradB[ki] += w * b[ki];
  }
  const n = cities01.length;
  const nextA = a.slice(), nextB = b.slice();
  for (let ki = 0; ki < N_COEFFS; ki++) {
    const k = ki - K_MAX;
    if (Math.abs(k) > kActiveMax) continue;
    nextA[ki] -= (lr / n) * gradA[ki];
    nextB[ki] -= (lr / n) * gradB[ki];
  }
  return { a: nextA, b: nextB };
}

function decodeTour(a: number[], b: number[], kActiveMax: number, cities01: [number, number][]): { order: number[]; sVals: number[] } {
  const pts01 = curvePoints(a, b, kActiveMax).map(([x, y]) => [x / SCALE, y / SCALE] as [number, number]);
  const sVals = cities01.map(([cx, cy]) => {
    let best = Infinity, bestJ = 0;
    for (let j = 0; j < pts01.length; j++) {
      const [px, py] = pts01[j];
      const d = (cx - px) ** 2 + (cy - py) ** 2;
      if (d < best) { best = d; bestJ = j; }
    }
    return bestJ / pts01.length;
  });
  const order = cities01.map((_: [number, number], i: number) => i).sort((i: number, j: number) => sVals[i] - sVals[j]);
  return { order, sVals };
}

// ---------------------------------------------------------------
// Shared hyperparameters (mirroring docs/algorithms/fourier.md defaults,
// scaled for normalized [0,1] city coordinates)
// ---------------------------------------------------------------
const LAMBDA0 = 0.05;
const LAMBDA_DECAY = 0.5;
const LR = 0.05;
const EPOCHS_PER_STAGE = 90;
const STEPS_PER_TICK = 2;

const CITIES01 = CITIES.map(([x, y]) => [x / SCALE, y / SCALE] as [number, number]);

// ---------------------------------------------------------------
// SVG sub-components
// ---------------------------------------------------------------
interface CurveSVGProps {
  curve: [number, number][];
  showCities?: boolean;
  tour?: [number, number][] | null;
  highlightSample?: ComponentChildren;
  children?: ComponentChildren;
}

function CurveSVG({ curve, showCities = true, tour = null, highlightSample = null, children }: CurveSVGProps) {
  const path = curve.length
    ? "M " + curve.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" L ") + " Z"
    : "";
  return (
    <svg viewBox="0 0 300 300" className="fx-canvas" role="img" aria-label="Fourier curve and cities">
      <rect x="0" y="0" width="300" height="300" className="fx-bg" />
      {tour && tour.length > 1 && (
        <polyline
          className="fx-tour"
          points={tour.map(([x, y]) => `${x},${y}`).join(" ")}
        />
      )}
      {path && <path d={path} className="fx-curve" />}
      {showCities && CITIES.map(([x, y], i) => (
        <circle key={i} cx={x} cy={y} r="4.5" className="fx-city" />
      ))}
      {highlightSample}
      {children}
    </svg>
  );
}

interface SparklineProps {
  values: number[];
  max?: number;
}

function Sparkline({ values, max }: SparklineProps) {
  if (!values.length) return null;
  const w = 300, h = 60;
  const m = max ?? Math.max(...values, 1e-9);
  const pts = values.map((v, i) => {
    const x = (i / Math.max(values.length - 1, 1)) * w;
    const y = h - (v / m) * (h - 4) - 2;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  return (
    <svg viewBox={`0 0 ${w} ${h}`} className="fx-spark" preserveAspectRatio="none">
      <polyline points={pts.join(" ")} className="fx-spark-line" />
    </svg>
  );
}

// ---------------------------------------------------------------
// Tab 1: Idea — what the curve + decode actually do
// ---------------------------------------------------------------
function IdeaTab() {
  // a nicely-converged coefficient set, precomputed for illustration
  const a = [-0.02, -0.01, -0.0, -0.01, 0.44, 0.35, -0.03, -0.05, -0.04];
  const b = [-0.03, -0.03, -0.01, -0.01, 0.46, -0.01, -0.0, 0.03, 0.02];
  const curve = useMemo(() => curvePoints(a, b), []);
  const { order } = useMemo(() => decodeTour(a, b, K_MAX, CITIES01), []);
  const tour = order.map(i => CITIES[i]).concat([CITIES[order[0]]]);

  return (
    <div className="fx-tab">
      <CurveSVG curve={curve} tour={tour} />
      <div className="fx-prose">
        <p>
          Every tour is drawn as a single closed curve in the plane,
          <code> γ(s) = Σ c_k · e^{"{2πiks}"}</code>, sampled at <code>M</code> points.
          A small set of complex coefficients <code>c_k</code> (here <code>2K+1 = 9</code> of them)
          is the <em>entire</em> search space &mdash; gradient descent only ever moves these
          9 numbers.
        </p>
        <p>
          To turn the curve into a tour, every city snaps to its nearest sample point on
          the loop (<strong>argsort decode</strong>, dashed line above). Because a sort can
          never produce duplicates or skip a city, the result is <em>always</em> a valid
          Hamiltonian tour &mdash; even before training finishes.
        </p>
        <p className="fx-note">
          Try the other two tabs: <strong>Modes</strong> shows what each frequency
          contributes to the loop shape, and <strong>Train</strong> runs the actual
          coarse-to-fine gradient descent live in your browser.
        </p>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------
// Tab 2: Modes — coarse-to-fine intuition via partial sums
// ---------------------------------------------------------------
function ModesTab() {
  const targetA = [-0.02, -0.01, -0.0, -0.01, 0.44, 0.35, -0.03, -0.05, -0.04];
  const targetB = [-0.03, -0.03, -0.01, -0.01, 0.46, -0.01, -0.0, 0.03, 0.02];
  const [kActive, setKActive] = useState(0);

  const curve = useMemo(() => curvePoints(targetA, targetB, kActive), [kActive]);

  const descriptions = [
    "k = 0 only: a single point at the centroid c₀. The curve hasn't opened up yet.",
    "± k = 1 added: the curve becomes a single loop (an ellipse-ish ring) enclosing the cities.",
    "± k = 2 added: the loop can now bend into a rounder, asymmetric shape.",
    "± k = 3 added: finer wiggles let the loop hug clusters of cities more closely.",
    "± k = 4 added (k_max): the full default resolution — enough detail for berlin52-scale instances.",
  ];

  return (
    <div className="fx-tab">
      <CurveSVG curve={curve} />
      <div className="fx-prose">
        <p>
          The coarse-to-fine loop in <code>fourier.md</code> unlocks one frequency pair
          (<code>±k</code>) at a time. Slide through the stages below to see how each
          added mode refines the loop — exactly the order the optimiser itself follows.
        </p>
        <div className="fx-row">
          <input
            type="range" min={0} max={K_MAX} step={1} value={kActive}
            onChange={(e) => setKActive(Number((e.target as HTMLInputElement).value))}
            className="fx-slider"
            aria-label="active modes"
          />
          <span className="fx-mono">k_active = {kActive}</span>
        </div>
        <p className="fx-caption">{descriptions[kActive]}</p>
        <p className="fx-note">
          Why this order? Low modes set the overall loop shape; unlocking high modes too
          early creates competing gradients and saddle points. Coarse-to-fine avoids that
          by giving each frequency band its own optimisation stage.
        </p>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------
// Tab 3: Train & Decode — live coarse-to-fine gradient descent
// ---------------------------------------------------------------
function TrainTab() {
  const stateRef = useRef(initCoeffsNormalized());
  const [coeffs, setCoeffs] = useState(stateRef.current);
  const [kActive, setKActive] = useState(1);
  const [lambda, setLambda] = useState(LAMBDA0);
  const [epoch, setEpoch] = useState(0);
  const [history, setHistory] = useState<number[]>([]);
  const [playing, setPlaying] = useState(false);
  const [done, setDone] = useState(false);

  const reset = useCallback(() => {
    stateRef.current = initCoeffsNormalized();
    setCoeffs(stateRef.current);
    setKActive(1);
    setLambda(LAMBDA0);
    setEpoch(0);
    setHistory([]);
    setDone(false);
    setPlaying(false);
  }, []);

  const stepOnce = useCallback(() => {
    let { a, b } = stateRef.current;
    let curKActive = kActive, curLambda = lambda, curEpoch = epoch, curDone = done;
    if (curDone) return;
    for (let s = 0; s < STEPS_PER_TICK; s++) {
      ({ a, b } = gradStep(a, b, curKActive, curLambda, LR, CITIES01));
      curEpoch += 1;
      if (curEpoch >= EPOCHS_PER_STAGE) {
        curEpoch = 0;
        if (curKActive < K_MAX) {
          curKActive += 1;
          curLambda *= LAMBDA_DECAY;
        } else {
          curDone = true;
          break;
        }
      }
    }
    stateRef.current = { a, b };
    setCoeffs({ a, b });
    setKActive(curKActive);
    setLambda(curLambda);
    setEpoch(curEpoch);
    const e = energy(a, b, curLambda, curKActive, CITIES01);
    setHistory(h => [...h.slice(-149), e.total]);
    if (curDone) { setDone(true); setPlaying(false); }
  }, [kActive, lambda, epoch, done]);

  useEffect(() => {
    if (!playing) return;
    const id = setInterval(stepOnce, 60);
    return () => clearInterval(id);
  }, [playing, stepOnce]);

  const curve = useMemo(() => curvePoints(coeffs.a, coeffs.b, kActive), [coeffs, kActive]);
  const { order } = useMemo(
    () => decodeTour(coeffs.a, coeffs.b, kActive, CITIES01),
    [coeffs, kActive]
  );
  const tour = order.map(i => CITIES[i]).concat([CITIES[order[0]]]);
  const stageProgress = Math.round((epoch / EPOCHS_PER_STAGE) * 100);
  const totalEnergy = history.length ? history[history.length - 1] : null;

  return (
    <div className="fx-tab">
      <CurveSVG curve={curve} tour={tour} />
      <div className="fx-prose">
        <div className="fx-controls">
          <button className="fx-btn fx-btn-primary" onClick={() => setPlaying(p => !p)} disabled={done}>
            {playing ? "Pause" : done ? "Converged" : "Play"}
          </button>
          <button className="fx-btn" onClick={stepOnce} disabled={playing || done}>Step</button>
          <button className="fx-btn" onClick={reset}>Reset</button>
        </div>

        <div className="fx-stagebar" aria-hidden="true">
          {Array.from({ length: K_MAX }, (_, i) => i + 1).map(stage => (
            <div
              key={stage}
              className={"fx-stage" + (stage < kActive || done ? " fx-stage-done" : stage === kActive ? " fx-stage-active" : "")}
              style={stage === kActive && !done ? { "--p": `${stageProgress}%` } as preact.JSX.CSSProperties : undefined}
            >
              <span>k≤{stage}</span>
            </div>
          ))}
        </div>

        <div className="fx-statgrid">
          <div>
            <div className="fx-statlabel">stage</div>
            <div className="fx-mono">k_active = {kActive}{done ? " (done)" : ` / ${K_MAX}`}</div>
          </div>
          <div>
            <div className="fx-statlabel">λ (tension weight)</div>
            <div className="fx-mono">{lambda.toFixed(4)}</div>
          </div>
          <div>
            <div className="fx-statlabel">energy E(c)</div>
            <div className="fx-mono">{totalEnergy !== null ? totalEnergy.toFixed(4) : "—"}</div>
          </div>
        </div>

        <Sparkline values={history} />
        <p className="fx-caption">
          Energy decreases within each stage; small jumps at stage boundaries are expected
          &mdash; new modes (and a smaller λ) are unlocked, briefly changing the loss
          landscape before the optimiser re-settles.
        </p>
        <p className="fx-note">
          The dashed line is the <strong>decode</strong> step re-run on the current curve:
          each city snaps to its nearest sample, then cities are sorted by that sample's
          position <code>s ∈ [0,1)</code> to get the tour. It's always a valid tour, even
          mid-training.
        </p>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------
// Root component
// ---------------------------------------------------------------
type TabId = "idea" | "modes" | "train";

interface TabDef {
  id: TabId;
  label: string;
  Component: () => preact.JSX.Element;
}

const TABS: TabDef[] = [
  { id: "idea", label: "Idea", Component: IdeaTab },
  { id: "modes", label: "Modes", Component: ModesTab },
  { id: "train", label: "Train & decode", Component: TrainTab },
];

export default function FourierExplainer() {
  const [tab, setTab] = useState<TabId>("idea");
  const Active = TABS.find(t => t.id === tab)!.Component;

  return (
    <div className="fx-root">
      <style>{CSS}</style>
      <header className="fx-header">
        <div className="fx-eyebrow">teeline · algorithms/fourier.md</div>
        <h2 className="fx-title">Fourier-basis constructive solver</h2>
        <p className="fx-sub">
          A tour is a closed curve γ(s) = Σ c_k·e^{"{2πiks}"}; gradient descent shapes the
          curve, an argsort decodes it into a valid tour.
        </p>
      </header>

      <nav className="fx-tabs" role="tablist">
        {TABS.map(t => (
          <button
            key={t.id}
            role="tab"
            aria-selected={tab === t.id}
            className={"fx-tabbtn" + (tab === t.id ? " fx-tabbtn-active" : "")}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </nav>

      <Active />

      <footer className="fx-footer">
        <span className="fx-mono">cities: {CITIES.length}</span>
        <span className="fx-mono">k_max: {K_MAX}</span>
        <span className="fx-mono">M: {M}</span>
        <span className="fx-mono">lr: {LR}</span>
        <span className="fx-mono">λ₀: {LAMBDA0}</span>
      </footer>
    </div>
  );
}

// ---------------------------------------------------------------
// Styles — self-contained, scoped via fx- prefix
// ---------------------------------------------------------------
const CSS = `
.fx-root {
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --curve: #0969da;
  --city: #f2a154;
  --tour: #e0626b;
  --accent: #0969da;
  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 20px;
  max-width: 760px;
  box-sizing: border-box;
}
.fx-root * { box-sizing: border-box; }
.fx-mono { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.85em; }

.fx-header { margin-bottom: 14px; }
.fx-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.fx-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.fx-sub { margin: 0; color: var(--muted); font-size: 0.92rem; line-height: 1.5; }

.fx-tabs { display: flex; gap: 6px; margin-bottom: 14px; border-bottom: 1px solid var(--line); }
.fx-tabbtn {
  background: transparent; border: none; color: var(--muted);
  font-size: 0.9rem; padding: 8px 12px; cursor: pointer;
  border-bottom: 2px solid transparent; margin-bottom: -1px;
  font-family: inherit;
}
.fx-tabbtn:hover { color: var(--text); }
.fx-tabbtn-active { color: var(--text); border-bottom-color: var(--accent); }

.fx-tab { display: flex; flex-direction: column; gap: 14px; }
@media (min-width: 620px) {
  .fx-tab { flex-direction: row; align-items: flex-start; }
  .fx-canvas, .fx-spark { flex: 0 0 300px; }
  .fx-prose { flex: 1 1 auto; min-width: 0; }
}

.fx-canvas { width: 100%; max-width: 300px; border-radius: 8px; border: 1px solid var(--line); display: block; }
.fx-bg { fill: var(--panel); }
.fx-curve { fill: none; stroke: var(--curve); stroke-width: 2; stroke-linejoin: round; }
.fx-city { fill: var(--city); stroke: var(--bg); stroke-width: 1; }
.fx-tour { fill: none; stroke: var(--tour); stroke-width: 1.2; stroke-dasharray: 4 3; opacity: 0.85; }

.fx-prose p { margin: 0 0 10px; font-size: 0.92rem; line-height: 1.55; }
.fx-prose code { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.88em; color: var(--accent); background: rgba(9,105,218,0.08); padding: 1px 4px; border-radius: 4px; }
.fx-caption { color: var(--muted); font-size: 0.85rem; }
.fx-note { font-size: 0.85rem; color: var(--muted); border-left: 2px solid var(--line); padding-left: 10px; }

.fx-row { display: flex; align-items: center; gap: 10px; margin: 8px 0; }
.fx-slider { flex: 1; accent-color: var(--accent); }

.fx-controls { display: flex; gap: 8px; margin-bottom: 10px; }
.fx-btn {
  background: var(--panel); color: var(--text); border: 1px solid var(--line);
  border-radius: 6px; padding: 6px 14px; font-size: 0.85rem; cursor: pointer;
  font-family: inherit;
}
.fx-btn:hover:not(:disabled) { border-color: var(--accent); }
.fx-btn:disabled { opacity: 0.45; cursor: default; }
.fx-btn-primary { color: var(--accent); border-color: var(--accent); }

.fx-stagebar { display: flex; gap: 4px; margin-bottom: 10px; }
.fx-stage {
  flex: 1; height: 22px; border: 1px solid var(--line); border-radius: 5px;
  display: flex; align-items: center; justify-content: center;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.7rem;
  color: var(--muted); position: relative; overflow: hidden;
}
.fx-stage span { z-index: 1; }
.fx-stage-done { border-color: var(--curve); color: var(--curve); }
.fx-stage-active {
  color: var(--text); border-color: var(--accent);
}
.fx-stage-active::before {
  content: ""; position: absolute; inset: 0; width: var(--p, 0%);
  background: rgba(9,105,218,0.15); transition: width 60ms linear;
}

.fx-statgrid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin-bottom: 10px; }
.fx-statlabel { font-size: 0.72rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px; }

.fx-spark { width: 100%; max-width: 300px; height: 60px; border: 1px solid var(--line); border-radius: 6px; background: var(--panel); display: block; margin-bottom: 6px; }
.fx-spark-line { fill: none; stroke: var(--tour); stroke-width: 1.5; }

.fx-footer { display: flex; flex-wrap: wrap; gap: 14px; margin-top: 16px; padding-top: 10px; border-top: 1px solid var(--line); color: var(--muted); }
`;

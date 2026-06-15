// teeline-web/src/explainers/lk.ts

// Fixed 15-city layout, 300×300 canvas.
// Ring of 12 + 3 interior: NN tour creates crossing edges (2-opt fodder);
// ring structure gives clean 4-segment double-bridge cuts.
export const CITIES: [number, number][] = [
  [ 40,  60], [ 80,  30], [140,  20], [200,  40], [250,  70],
  [270, 130], [250, 200], [200, 250], [140, 270], [ 80, 250],
  [ 40, 200], [ 20, 130], [150, 150], [100, 100], [200, 100],
];

export function euclidDist(a: [number, number], b: [number, number]): number {
  return Math.hypot(a[0] - b[0], a[1] - b[1]);
}

export function buildDistMatrix(cities: [number, number][]): number[][] {
  const n = cities.length;
  return Array.from({ length: n }, (_, i) =>
    Array.from({ length: n }, (_, j) => euclidDist(cities[i], cities[j]))
  );
}

export function tourDist(tour: number[], dist: number[][]): number {
  const n = tour.length;
  let total = 0;
  for (let i = 0; i < n; i++) {
    total += dist[tour[i]][tour[(i + 1) % n]];
  }
  return total;
}

export function nnTour(cities: [number, number][], dist: number[][]): number[] {
  const n = cities.length;
  const visited = new Array<boolean>(n).fill(false);
  const tour = [0];
  visited[0] = true;
  for (let step = 1; step < n; step++) {
    const last = tour[step - 1];
    let bestJ = -1, bestD = Infinity;
    for (let j = 0; j < n; j++) {
      if (!visited[j] && dist[last][j] < bestD) {
        bestD = dist[last][j];
        bestJ = j;
      }
    }
    tour.push(bestJ);
    visited[bestJ] = true;
  }
  return tour;
}

// Linear congruential generator — seeded for reproducible animations.
export function lcgRand(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (Math.imul(1664525, s) + 1013904223) >>> 0;
    return s / 0x100000000;
  };
}

// Pre-computed constants used by both tabs.
export const DIST = buildDistMatrix(CITIES);
export const INIT_TOUR = nnTour(CITIES, DIST);

// Port of lin_kernighan.rs::double_bridge.
// Returns the kicked tour plus the three cut positions used,
// so callers can highlight the cut cities in the animation.
export function doubleBridge(
  tour: number[],
  rand: () => number
): { result: number[]; p1: number; p2: number; p3: number } {
  const n = tour.length;
  const p1 = 1 + Math.floor(rand() * Math.floor(n / 4));
  const p2 = p1 + 1 + Math.floor(rand() * Math.floor(n / 4));
  const p3 = p2 + 1 + Math.floor(rand() * Math.floor(n / 4));
  const result = [
    ...tour.slice(0, p1),
    ...tour.slice(p2, p3),
    ...tour.slice(p1, p2),
    ...tour.slice(p3),
  ];
  return { result, p1, p2, p3 };
}

// Each [from, to] pair is a city index pair forming one edge.
export type EdgePair = [[number, number], [number, number]];

export interface LSFrame {
  tour: number[];
  scanEdges: EdgePair | null;   // edges being considered (gray dashed in Step mode)
  swapEdges: EdgePair | null;   // new edges just accepted (green flash)
  dist: number;
  overlay: string | null;       // label overlaid on SVG ("Local optimum")
  isScan: boolean;              // true → skip this frame in Run mode
}

// Pre-computes all 2-opt animation frames (first-improvement, single restart per swap).
// scan events are interleaved so Step mode can show them; Run mode skips isScan frames.
// How many non-improving "miss" scan frames to show before each accepted swap.
const PREVIEW_SCANS = 2;

export function computeLocalSearchFrames(
  initTour: number[],
  dist: number[][]
): LSFrame[] {
  const frames: LSFrame[] = [];
  const tour = [...initTour];
  const n = tour.length;
  let currentDist = tourDist(tour, dist);

  let foundImprovement = true;
  while (foundImprovement) {
    foundImprovement = false;

    // First pass: collect up to PREVIEW_SCANS non-improving pairs, then find the winner.
    const misses: EdgePair[] = [];
    let winI = -1, winJ = -1;

    outer:
    for (let i = 0; i < n - 1; i++) {
      for (let j = i + 2; j < n; j++) {
        if (i === 0 && j === n - 1) continue;
        const removed = dist[tour[i]][tour[i + 1]] + dist[tour[j]][tour[(j + 1) % n]];
        const added   = dist[tour[i]][tour[j]]     + dist[tour[i + 1]][tour[(j + 1) % n]];
        if (removed - added > 1e-10) { winI = i; winJ = j; break outer; }
        if (misses.length < PREVIEW_SCANS) {
          misses.push([[tour[i], tour[i + 1]], [tour[j], tour[(j + 1) % n]]]);
        }
      }
    }

    if (winI === -1) break; // no improvement found in this pass

    // Emit miss scan frames (showing "still looking...")
    for (const edges of misses) {
      frames.push({ tour: [...tour], scanEdges: edges, swapEdges: null, dist: currentDist, overlay: null, isScan: true });
    }

    // Emit winning pair as scan frame (showing "found it!")
    frames.push({
      tour: [...tour],
      scanEdges: [[tour[winI], tour[winI + 1]], [tour[winJ], tour[(winJ + 1) % n]]],
      swapEdges: null, dist: currentDist, overlay: null, isScan: true,
    });

    // Apply 2-opt reversal and emit swap frame
    const newEdge1: [number, number] = [tour[winI],     tour[winJ]];
    const newEdge2: [number, number] = [tour[winI + 1], tour[(winJ + 1) % n]];
    tour.splice(winI + 1, winJ - winI, ...tour.slice(winI + 1, winJ + 1).reverse());
    currentDist = tourDist(tour, dist);
    frames.push({ tour: [...tour], scanEdges: null, swapEdges: [newEdge1, newEdge2], dist: currentDist, overlay: null, isScan: false });

    foundImprovement = true;
  }

  // Terminal frame
  frames.push({ tour: [...tour], scanEdges: null, swapEdges: null, dist: currentDist, overlay: "Local optimum", isScan: false });

  return frames;
}

export interface ILSFrame {
  tour: number[];
  bestTour: number[];
  swapEdges: EdgePair | null;
  bridgePoints: [number, number, number] | null; // city indices at cut positions
  phase: string;
  currentDist: number;
  bestDist: number;
  restarts: number;
  plateauCount: number;
  overlay: string | null;
  highlight: 'swap' | 'bridge' | 'best' | null;
}

// Number of duplicate frames for visual hold on bridge_cut and new_best events.
const ILS_HOLD = 3;

// Pre-computes all ILS animation frames: NN → 2-opt → [kick → 2-opt → accept/reject]*.
// No scan events — Tab 2 shows only accepted swaps and ILS events.
export function computeILSFrames(
  initTour: number[],
  dist: number[][],
  maxEpochs: number,
  plateauLimit: number,
  rand: () => number
): ILSFrame[] {
  const frames: ILSFrame[] = [];

  let currentTour = [...initTour];
  let bestTour = [...initTour];
  let currentDist = tourDist(currentTour, dist);
  let bestDist = currentDist;
  let restarts = 0;
  let plateauCount = 0;
  const n = currentTour.length;

  // Helper: run first-improvement 2-opt on `t` in place, emitting swap frames.
  function runPass(t: number[], phase: string): void {
    let improved = true;
    while (improved) {
      improved = false;
      outer:
      for (let i = 0; i < n - 1; i++) {
        for (let j = i + 2; j < n; j++) {
          if (i === 0 && j === n - 1) continue;
          const removed = dist[t[i]][t[i + 1]] + dist[t[j]][t[(j + 1) % n]];
          const added   = dist[t[i]][t[j]]     + dist[t[i + 1]][t[(j + 1) % n]];
          if (removed - added > 1e-10) {
            const e1: [number, number] = [t[i],     t[j]];
            const e2: [number, number] = [t[i + 1], t[(j + 1) % n]];
            t.splice(i + 1, j - i, ...t.slice(i + 1, j + 1).reverse());
            currentDist = tourDist(t, dist);
            frames.push({
              tour: [...t], bestTour: [...bestTour],
              swapEdges: [e1, e2], bridgePoints: null,
              phase, currentDist, bestDist, restarts, plateauCount,
              overlay: null, highlight: 'swap',
            });
            improved = true;
            break outer;
          }
        }
      }
    }
  }

  // Initial pass
  frames.push({
    tour: [...currentTour], bestTour: [...bestTour],
    swapEdges: null, bridgePoints: null,
    phase: 'LK pass', currentDist, bestDist, restarts, plateauCount,
    overlay: null, highlight: null,
  });
  runPass(currentTour, 'LK pass');
  currentDist = tourDist(currentTour, dist);

  if (currentDist < bestDist) {
    bestDist = currentDist;
    bestTour = [...currentTour];
    plateauCount = 0;
  }
  for (let h = 0; h < ILS_HOLD; h++) {
    frames.push({
      tour: [...currentTour], bestTour: [...bestTour],
      swapEdges: null, bridgePoints: null,
      phase: 'New best!', currentDist, bestDist, restarts, plateauCount,
      overlay: null, highlight: 'best',
    });
  }

  // ILS loop
  for (let epoch = 0; epoch < maxEpochs; epoch++) {
    // Kick
    const { result: kicked, p1, p2, p3 } = doubleBridge(bestTour, rand);
    currentTour = kicked;
    currentDist = tourDist(currentTour, dist);
    restarts++;
    const bridgePoints: [number, number, number] = [bestTour[p1], bestTour[p2], bestTour[p3]];

    for (let h = 0; h < ILS_HOLD; h++) {
      frames.push({
        tour: [...currentTour], bestTour: [...bestTour],
        swapEdges: null, bridgePoints,
        phase: 'Double-bridge kick', currentDist, bestDist, restarts, plateauCount,
        overlay: null, highlight: 'bridge',
      });
    }

    // Optimise kicked tour
    frames.push({
      tour: [...currentTour], bestTour: [...bestTour],
      swapEdges: null, bridgePoints: null,
      phase: 'LK pass', currentDist, bestDist, restarts, plateauCount,
      overlay: null, highlight: null,
    });
    runPass(currentTour, 'LK pass');
    currentDist = tourDist(currentTour, dist);

    // Local optimum label
    frames.push({
      tour: [...currentTour], bestTour: [...bestTour],
      swapEdges: null, bridgePoints: null,
      phase: 'Local optimum', currentDist, bestDist, restarts, plateauCount,
      overlay: 'Local optimum', highlight: null,
    });

    // Accept / reject
    if (currentDist < bestDist - 1e-10) {
      bestDist = currentDist;
      bestTour = [...currentTour];
      plateauCount = 0;
      for (let h = 0; h < ILS_HOLD; h++) {
        frames.push({
          tour: [...currentTour], bestTour: [...bestTour],
          swapEdges: null, bridgePoints: null,
          phase: 'New best!', currentDist, bestDist, restarts, plateauCount,
          overlay: null, highlight: 'best',
        });
      }
    } else {
      plateauCount++;
      frames.push({
        tour: [...currentTour], bestTour: [...bestTour],
        swapEdges: null, bridgePoints: null,
        phase: 'Plateau', currentDist, bestDist, restarts, plateauCount,
        overlay: null, highlight: null,
      });
      if (plateauCount >= plateauLimit) {
        frames.push({
          tour: [...bestTour], bestTour: [...bestTour],
          swapEdges: null, bridgePoints: null,
          phase: 'Done', currentDist: bestDist, bestDist, restarts, plateauCount,
          overlay: 'Done — plateau limit reached', highlight: null,
        });
        return frames;
      }
    }
  }

  frames.push({
    tour: [...bestTour], bestTour: [...bestTour],
    swapEdges: null, bridgePoints: null,
    phase: 'Done', currentDist: bestDist, bestDist, restarts, plateauCount,
    overlay: 'Done — max epochs', highlight: null,
  });
  return frames;
}

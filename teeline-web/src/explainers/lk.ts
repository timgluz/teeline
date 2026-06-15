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
    outer:
    for (let i = 0; i < n - 1; i++) {
      for (let j = i + 2; j < n; j++) {
        if (i === 0 && j === n - 1) continue; // would reverse entire tour

        // Scan frame: show the two candidate edges (old edges that might be removed)
        frames.push({
          tour: [...tour],
          scanEdges: [[tour[i], tour[i + 1]], [tour[j], tour[(j + 1) % n]]],
          swapEdges: null,
          dist: currentDist,
          overlay: null,
          isScan: true,
        });

        const removed = dist[tour[i]][tour[i + 1]] + dist[tour[j]][tour[(j + 1) % n]];
        const added   = dist[tour[i]][tour[j]]     + dist[tour[i + 1]][tour[(j + 1) % n]];
        const gain = removed - added;

        if (gain > 1e-10) {
          // Record new edges before applying reversal
          const newEdge1: [number, number] = [tour[i],     tour[j]];
          const newEdge2: [number, number] = [tour[i + 1], tour[(j + 1) % n]];

          // Apply 2-opt reversal: reverse segment [i+1 .. j]
          tour.splice(i + 1, j - i, ...tour.slice(i + 1, j + 1).reverse());
          currentDist = tourDist(tour, dist);

          frames.push({
            tour: [...tour],
            scanEdges: null,
            swapEdges: [newEdge1, newEdge2],
            dist: currentDist,
            overlay: null,
            isScan: false,
          });

          foundImprovement = true;
          break outer; // restart scan from i=0 after each swap
        }
      }
    }
  }

  // Terminal frame
  frames.push({
    tour: [...tour],
    scanEdges: null,
    swapEdges: null,
    dist: currentDist,
    overlay: "Local optimum",
    isScan: false,
  });

  return frames;
}

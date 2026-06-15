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

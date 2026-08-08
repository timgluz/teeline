// Real-bindings integration tests — imports the actual teeline-wasm
// bindings (same file teeline-wasm/js-bindings/smoke.mjs exercises), not a
// mock. Proves the numeric core is correct without needing Excel at all.
import { describe, it, expect } from "vitest";
import { solvers, tspSolve, solveDistance, distanceEuc } from "../src/functions/functions";

function closedLoopDistance(coords: number[][]): number {
  const n = coords.length;
  let total = 0;
  for (let i = 0; i < n; i++) {
    const [ax, ay] = coords[i];
    const [bx, by] = coords[(i + 1) % n];
    total += Math.hypot(ax - bx, ay - by);
  }
  return total;
}

function sortedRows(rows: number[][]): string[] {
  return rows.map((r) => r.join(",")).sort();
}

const FIVE_CITIES: number[][] = [
  [565, 575],
  [25, 185],
  [345, 750],
  [945, 685],
  [845, 655],
];

// A square entered in a self-crossing order (0-2-1-3 instead of 0-1-2-3),
// so there's a concrete, solver-independent improvement available — a
// reliable case to prove solving actually helps, without assuming any
// particular heuristic beats an arbitrary natural order in general.
const CROSSED_SQUARE: number[][] = [
  [0, 0],
  [10, 10],
  [10, 0],
  [0, 10],
];

describe("SOLVERS", () => {
  it("returns known solver ids", () => {
    const ids = solvers().map((row) => row[0]);
    expect(ids).toContain("nn");
    expect(ids).toContain("2opt");
    expect(ids.length).toBeGreaterThan(10);
  });
});

describe("SOLVE", () => {
  it("returns a permutation of the input rows", () => {
    const result = tspSolve(FIVE_CITIES, "nn");
    expect(sortedRows(result)).toEqual(sortedRows(FIVE_CITIES));
  });

  it("improves a deliberately self-crossing route", () => {
    const solved = tspSolve(CROSSED_SQUARE, "2opt");
    expect(closedLoopDistance(solved)).toBeLessThan(closedLoopDistance(CROSSED_SQUARE));
  });
});

describe("SOLVE_DISTANCE", () => {
  it("matches the closed-loop distance of SOLVE's own reordered output", () => {
    const solved = tspSolve(FIVE_CITIES, "nn");
    const reported = solveDistance(FIVE_CITIES, "nn");
    expect(reported).toBeCloseTo(closedLoopDistance(solved), 3);
  });
});

describe("DISTANCE_EUC", () => {
  it("matches a manually computed closed-loop distance of the input as given", () => {
    expect(distanceEuc(FIVE_CITIES)).toBeCloseTo(closedLoopDistance(FIVE_CITIES), 3);
  });

  it("does not reorder — measures the crossed square as worse than its solved distance", () => {
    const asGiven = distanceEuc(CROSSED_SQUARE);
    const solved = solveDistance(CROSSED_SQUARE, "2opt");
    expect(asGiven).toBeGreaterThan(solved);
  });
});

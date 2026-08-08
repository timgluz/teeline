/* global CustomFunctions */

import { listAlgorithms, solve, tourDistance } from "teeline-wasm";
import type { City, SolveOptions } from "teeline-wasm";

const DEFAULT_SOLVER = "nn";

// Matches teeline-web's defaultSolveOptions() (teeline-web/src/solver-options.ts)
// so behavior is consistent across subprojects for solvers that use them.
const DEFAULT_OPTIONS: SolveOptions = {
  epochs: 500,
  platooEpochs: 50,
  coolingRate: 0.0001,
  maxTemperature: 1000.0,
  minTemperature: 0.001,
  mutationProbability: 0.05,
  nElite: 3,
  nNearest: 5,
};

function invalidValue(message: string): CustomFunctions.Error {
  return new CustomFunctions.Error(CustomFunctions.ErrorCode.invalidValue, message);
}

/**
 * Converts an Excel range into City records with sequential ids matching
 * input row order. Requires at least 2 rows and exactly 2 numeric columns.
 * Non-numeric cells (e.g. a header row) arrive here as NaN — Excel coerces
 * the range to numbers before this function runs, it doesn't reject them
 * itself, so this is where that check has to happen.
 */
function citiesFromRange(range: number[][]): City[] {
  if (!Array.isArray(range) || range.length < 2) {
    throw invalidValue("range must contain at least 2 rows of (x, y) coordinates");
  }
  return range.map((row, index) => {
    if (
      !Array.isArray(row) ||
      row.length !== 2 ||
      !Number.isFinite(row[0]) ||
      !Number.isFinite(row[1])
    ) {
      throw invalidValue(
        `row ${index + 1} must contain exactly 2 numeric values (x, y) — check for a header row or non-numeric cell`
      );
    }
    return { id: index, x: row[0], y: row[1] };
  });
}

/**
 * Lists the ids of every available TSP solver (e.g. "nn", "2opt", "sa").
 *
 * This is also the scaffolding smoke test: if this function returns real
 * solver ids, the WASM component is loading correctly inside Excel.
 * @customfunction SOLVERS
 * @returns A single-column list of solver ids.
 */
export function solvers(): string[][] {
  try {
    return listAlgorithms().map((algo) => [algo.id]);
  } catch (err) {
    throw invalidValue(String(err));
  }
}

/**
 * Solves the traveling salesman problem for a range of (x, y) coordinates
 * and returns the same coordinates reordered into the optimal visiting
 * sequence. Distance is Euclidean and closed-loop (last city back to the
 * first). This is a synchronous WASM call under an async wrapper — it will
 * visibly block Excel for its duration; the "nn" default keeps that fast in
 * practice, but heavier solvers or very large ranges will noticeably freeze
 * Excel rather than queuing in the background.
 * @customfunction SOLVE
 * @param range A range of at least 2 rows, each with exactly 2 numeric columns (x, y).
 * @param solver Solver id from TSPSOLVER.SOLVERS(), e.g. "nn" or "2opt". Defaults to "nn".
 * @returns The input rows reordered into the solved route.
 */
export function tspSolve(range: number[][], solver?: string): number[][] {
  const cities = citiesFromRange(range);
  try {
    const result = solve(solver ?? DEFAULT_SOLVER, cities, DEFAULT_OPTIONS);
    // result.route is a Uint32Array — TypedArray#map coerces its callback's
    // return value back to a number (ToNumber), so mapping straight to
    // [x, y] pairs on it silently collapses every pair to NaN -> 0. Convert
    // to a plain Array first so .map() can return arbitrary values.
    return Array.from(result.route).map((id) => {
      const city = cities[id];
      return [city.x, city.y];
    });
  } catch (err) {
    throw invalidValue(String(err));
  }
}

/**
 * Solves the traveling salesman problem for a range of (x, y) coordinates
 * and returns only the total closed-loop tour distance.
 * @customfunction SOLVE_DISTANCE
 * @param range A range of at least 2 rows, each with exactly 2 numeric columns (x, y).
 * @param solver Solver id from TSPSOLVER.SOLVERS(), e.g. "nn" or "2opt". Defaults to "nn".
 * @returns The total distance of the solved tour.
 */
export function solveDistance(range: number[][], solver?: string): number {
  const cities = citiesFromRange(range);
  try {
    return solve(solver ?? DEFAULT_SOLVER, cities, DEFAULT_OPTIONS).total;
  } catch (err) {
    throw invalidValue(String(err));
  }
}

/**
 * Measures the closed-loop Euclidean distance of a range of (x, y)
 * coordinates as given — no solving, no reordering. Useful for comparing
 * against TSPSOLVER.SOLVE_DISTANCE to see how much a solver improves on the
 * as-entered order.
 * @customfunction DISTANCE_EUC
 * @param range A range of at least 2 rows, each with exactly 2 numeric columns (x, y).
 * @returns The total distance of the route as given.
 */
export function distanceEuc(range: number[][]): number {
  const cities = citiesFromRange(range);
  const route = Uint32Array.from(cities, (city) => city.id);
  try {
    return tourDistance(route, cities);
  } catch (err) {
    throw invalidValue(String(err));
  }
}

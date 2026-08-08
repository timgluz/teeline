// Pure input-validation edge cases — every one of these throws before any
// WASM call happens, so no solver actually runs here.
import { describe, it, expect } from "vitest";
import { tspSolve, solveDistance, distanceEuc } from "../src/functions/functions";

describe("range validation", () => {
  it("rejects an empty range", () => {
    expect(() => distanceEuc([])).toThrow(/at least 2 rows/);
  });

  it("rejects a single-row range", () => {
    expect(() => distanceEuc([[1, 2]])).toThrow(/at least 2 rows/);
  });

  it("rejects a non-numeric (header) row", () => {
    // Excel coerces non-numeric cells to NaN before this function runs.
    expect(() => distanceEuc([[NaN, NaN], [1, 2], [3, 4]])).toThrow(/numeric/);
  });

  it("rejects a row with the wrong number of columns", () => {
    expect(() => distanceEuc([[1, 2, 3], [4, 5, 6]] as unknown as number[][])).toThrow(/numeric/);
  });

  it("applies the same validation to SOLVE", () => {
    expect(() => tspSolve([[1, 2]])).toThrow(/at least 2 rows/);
  });

  it("applies the same validation to SOLVE_DISTANCE", () => {
    expect(() => solveDistance([[1, 2]])).toThrow(/at least 2 rows/);
  });
});

describe("solver name validation", () => {
  it("surfaces an unknown solver name as an error rather than throwing an uncaught exception", () => {
    expect(() => solveDistance([[0, 0], [1, 1], [2, 2]], "does_not_exist")).toThrow();
  });
});

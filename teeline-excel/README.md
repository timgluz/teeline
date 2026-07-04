# teeline-excel

Excel Custom Functions add-in — select a range of `(x, y)` coordinates and solve the traveling salesman problem without leaving the spreadsheet. Runs the same `teeline-wasm` Component Model build `teeline-web` uses, loaded into Excel's shared runtime (Edge WebView2 on Windows, WebKit on Mac/web).

## Functions

All functions live under the `TSPSOLVER` namespace.

| Function | Signature | Returns |
| --- | --- | --- |
| `TSPSOLVER.SOLVERS` | `()` | Single-column list of solver ids (e.g. `"nn"`, `"2opt"`, `"sa"`) — also doubles as the scaffolding smoke test |
| `TSPSOLVER.SOLVE` | `(range, [solver])` | The input rows spilled back out, reordered into the solved route |
| `TSPSOLVER.SOLVE_DISTANCE` | `(range, [solver])` | Total closed-loop distance of the solved tour |
| `TSPSOLVER.DISTANCE_EUC` | `(range)` | Closed-loop distance of `range` exactly as entered — no solving, no reordering |

`range` must be at least 2 rows with exactly 2 numeric columns (x, y). `solver` defaults to `"nn"` if omitted; pass any id returned by `TSPSOLVER.SOLVERS()`. Distance is Euclidean and closed-loop (the tour returns from the last city to the first) — consistent with every other teeline subproject. GEO distance (`teeline`'s core lib already supports it for TSPLIB input) is not exposed here yet.

Example:

```text
=TSPSOLVER.SOLVE(A1:B5)
=TSPSOLVER.SOLVE_DISTANCE(A1:B5, "2opt")
=TSPSOLVER.DISTANCE_EUC(A1:B5)
```

## Local Development

Requires the `teeline-wasm` component to already be built with jco bindings generated at `../teeline-wasm/js-bindings` (see [`teeline-wasm/README.md`](../teeline-wasm/README.md)) — this add-in consumes that shared package via a `file:` dependency, the same way `teeline-web` does, rather than a private re-transpiled copy.

```bash
cd teeline-wasm
cargo component build --target wasm32-wasip2 --release
npx jco transpile target/wasm32-wasip2/release/teeline_wasm.wasm --name teeline_wasm -o js-bindings

cd ../teeline-excel
npm install
npm start          # builds, sideloads, and opens Excel with the add-in loaded
```

`npm start` (via `office-addin-debugging`) handles trusting the local dev certificate and sideloading `manifest.xml` for you. See [Microsoft's sideloading guide](https://learn.microsoft.com/office/dev/add-ins/testing/test-debug-office-add-ins#sideload-an-office-add-in-for-testing) if it doesn't launch automatically.

### The WASI-preview2-shim gotcha

`teeline-wasm`'s jco-transpiled bindings target WASI Preview 2, which needs `@bytecodealliance/preview2-shim` to polyfill host imports (clocks, random). That package ships separate `node/` and `browser/` variants selected via `package.json` "exports" conditions — webpack resolves the **node** variant by default here (it's a transitive dependency nested under `teeline-wasm/js-bindings/node_modules`, not a direct dependency of this project), which pulls in `node:fs/promises` and fails to bundle. `webpack.config.js` works around this two ways:

- A `resolve.alias` entry that points each `@bytecodealliance/preview2-shim/*` subpath straight at its `browser/` file — mirrors `teeline-web`'s `force-preview2-shim-browser` vite plugin (`teeline-web/vite.config.ts`), just expressed as a webpack alias instead of a vite plugin.
- A `webpack.IgnorePlugin` for `node:fs/promises` — jco's generated glue code (`fetchCompile`) has a dynamic `import('node:fs/promises')` gated behind a runtime `isNode` check that's always false in a browser, but webpack still tries to statically resolve the `node:` scheme and fails without this.

If either of these ever needs adjusting for a future jco/webpack version bump, that's where to look.

## Testing

```bash
npm test        # vitest: real-bindings integration tests + pure-JS validation tests
npm run lint    # office-addin-lint (eslint + prettier)
npx tsc --noEmit
```

`tests/solving.test.ts` imports the actual `teeline-wasm` bindings (same ones `teeline-wasm/js-bindings/smoke.mjs` exercises) — no mocks — and checks that `SOLVE` returns a genuine permutation of the input, that `SOLVE_DISTANCE` matches the closed-loop distance of `SOLVE`'s own output, and that solving demonstrably improves a deliberately self-crossing route. `tests/validation.test.ts` covers range-shape edge cases (empty range, single row, non-numeric cells, wrong column count) that throw before any WASM call happens.

Manual sideload testing in real Excel is not covered by this suite and has to be done by hand — `npm test` proves the numeric core is correct; only sideloading proves the Excel-specific mechanics (spill behavior, error display, manifest/runtime loading).

## Semantics & Edge Cases

- **Header rows**: a range with a text header row throws (non-numeric cells arrive as `NaN` after Excel's own coercion). Select data rows only — auto-detecting and skipping a header row is deferred, since it's ambiguous when only one of the two columns has one.
- **`#SPILL!`**: `SOLVE`'s output is a dynamic array. If any target cell is occupied, Excel shows `#SPILL!` on the formula cell — that's Excel's own behavior, not something this add-in controls.
- **f32/f64 precision**: coordinates cross the WASM boundary as `f32`; Excel cells are `f64`. `SOLVE`'s spilled output re-emits the *original* Excel values, so there's no precision loss there — but `SOLVE_DISTANCE`/`DISTANCE_EUC` return an f32-computed distance widened to f64. It won't bit-for-bit match a distance you compute manually from the spilled coordinates in Excel; expect the usual float rounding, not a bug.
- **Blocking**: custom functions on a shared runtime run on the same JS context as the task pane — there's no Worker in this v1. Even wrapped in `async`, a `solve()` call visibly blocks Excel for its duration. The `"nn"` default keeps this fast in practice; a heavier solver or a very large range will noticeably freeze Excel rather than running quietly in the background.
- **No city-count limit** is enforced by this add-in itself in v1.

## Architecture

```text
teeline-excel/
├── manifest.xml           # Office Add-in manifest — TSPSOLVER namespace, SharedRuntime requirement
├── webpack.config.js      # bundling + the preview2-shim/node:fs workarounds above
├── src/
│   ├── functions/
│   │   └── functions.ts   # the four TSPSOLVER.* functions
│   ├── commands/          # unused ribbon-command boilerplate from the yo office scaffold
│   └── taskpane/          # minimal task pane required by the shared runtime; no custom UI built for v1
└── tests/
    ├── setup.ts           # stubs the global CustomFunctions.Error API for Node/Vitest
    ├── solving.test.ts    # real-bindings integration tests
    └── validation.test.ts # pure-JS range validation tests
```

Scaffolded with `yo office` (project type "Excel Custom Functions using a Shared Runtime", TypeScript).

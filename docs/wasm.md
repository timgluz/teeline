# Using Teeline via WebAssembly Component Model

Teeline exposes all its TSP solvers as a [WebAssembly Component Model](https://component-model.bytecodealliance.org/) library. The component can be called from Go, Python, JavaScript, Rust, and any runtime that supports the Component Model (including [Spin](https://spinframework.io/)).

---

## Interface

The WIT interface lives in [`teeline-wasm/wit/world.wit`](../teeline-wasm/wit/world.wit):

```wit
package teeline:solver@0.1.0;

world solver {
    export solve: func(
        solver: string,
        cities: list<city>,
        options: solve-options,
    ) -> result<solution, string>;
}
```

**`solver`** — algorithm name (see table below)  
**`cities`** — list of `{ id: u32, x: f32, y: f32 }` records  
**`options`** — tuning parameters (all have sensible defaults — see table below)  
**returns** — `{ total: f32, route: list<u32> }` or an error string

### Solver names

| Name(s) | Algorithm |
|---|---|
| `sa`, `simulated_annealing` | Simulated Annealing |
| `2opt`, `two_opt` | 2-opt local search |
| `nn`, `nearest_neighbor` | Nearest Neighbor |
| `ga`, `genetic_algorithm` | Genetic Algorithm |
| `pso`, `particle_swarm` | Particle Swarm Optimisation |
| `cs`, `cuckoo_search` | Cuckoo Search |
| `fpa`, `flower_pollination` | Flower Pollination Algorithm |
| `tabu_search` | Tabu Search |
| `stochastic_hill` | Stochastic Hill Climbing |
| `bhk`, `bellman_karp` | Bellman–Held–Karp (**exact — ≤ 20 cities**) |
| `branch_bound` | Branch and Bound (**exact — ≤ 20 cities**) |

### Default solve-options

| Field | Default | Notes |
|---|---|---|
| `epochs` | 10 000 | Maximum iterations |
| `platoo-epochs` | 500 | Iterations without improvement before restart |
| `cooling-rate` | 0.0001 | SA temperature decay per step |
| `max-temperature` | 1000.0 | SA starting temperature |
| `min-temperature` | 0.001 | SA stopping temperature |
| `mutation-probability` | 0.001 | GA/CS/FPA mutation or switch probability |
| `n-elite` | 3 | GA elite individuals preserved per generation |
| `n-nearest` | 3 | PSO/CS/FPA population size |

---

## Building the component

Prerequisites: Rust toolchain, [`cargo-component`](https://github.com/bytecodealliance/cargo-component), `wasm-tools`.

```bash
# Install tools (one-time)
cargo install cargo-component
cargo install wasm-tools
rustup target add wasm32-wasip2

# Debug build
cargo component build --manifest-path teeline-wasm/Cargo.toml

# Release build (recommended for production)
cargo component build --manifest-path teeline-wasm/Cargo.toml --release
```

Artifacts:
- Debug: `target/wasm32-wasip1/debug/teeline_wasm.wasm`
- Release: `target/wasm32-wasip1/release/teeline_wasm.wasm`

Inspect the component's exported interface:

```bash
wasm-tools component wit target/wasm32-wasip1/debug/teeline_wasm.wasm
```

---

## JavaScript / Node.js

Use [`jco`](https://github.com/bytecodealliance/jco) to transpile the component to a native ES module:

```bash
npm install -g @bytecodealliance/jco
jco transpile target/wasm32-wasip1/release/teeline_wasm.wasm \
    -o teeline-wasm/js-bindings/
cd teeline-wasm/js-bindings && npm install @bytecodealliance/preview2-shim
```

The transpiled module exports `solve` directly:

```js
import { solve } from './teeline_wasm.js';

const cities = [
    { id: 0, x: 565.0, y: 575.0 },
    { id: 1, x: 25.0,  y: 185.0 },
    { id: 2, x: 345.0, y: 750.0 },
    { id: 3, x: 945.0, y: 685.0 },
    { id: 4, x: 845.0, y: 655.0 },
];

const options = {
    epochs: 5000,
    platooEpochs: 200,
    coolingRate: 0.0001,
    maxTemperature: 1000.0,
    minTemperature: 0.001,
    mutationProbability: 0.001,
    nElite: 3,
    nNearest: 3,
};

const result = solve('sa', cities, options);
console.log(`distance: ${result.total.toFixed(2)}`);
console.log(`route: ${result.route.join(' → ')}`);
```

> **Note:** jco maps WIT kebab-case field names to camelCase — `platoo-epochs` becomes `platooEpochs`, `cooling-rate` becomes `coolingRate`, etc.

Pre-transpiled JS bindings are included in `teeline-wasm/js-bindings/` for convenience but should be regenerated when the component is rebuilt.

---

## Python

Install [`wasmtime-py`](https://github.com/bytecodealliance/wasmtime-py):

```bash
pip install wasmtime
```

Generate Python bindings from the WIT file using [`wit-bindgen`](https://github.com/bytecodealliance/wit-bindgen):

```bash
cargo install wit-bindgen-cli
wit-bindgen python --out-dir teeline-wasm/py-bindings teeline-wasm/wit/
```

Then call the component:

```python
import wasmtime
from teeline_wasm import Root, RootImports
from teeline.solver.types import City, SolveOptions

config = wasmtime.Config()
config.wasm_component_model = True
engine = wasmtime.Engine(config)

with open("target/wasm32-wasip1/release/teeline_wasm.wasm", "rb") as f:
    component = wasmtime.Component(engine, f.read())

store = wasmtime.Store(engine)
imports = RootImports()  # no host imports required for this component
instance = Root(store, component, imports)

cities = [
    City(id=0, x=565.0, y=575.0),
    City(id=1, x=25.0,  y=185.0),
    City(id=2, x=345.0, y=750.0),
    City(id=3, x=945.0, y=685.0),
    City(id=4, x=845.0, y=655.0),
]

options = SolveOptions(
    epochs=5000,
    platoo_epochs=200,
    cooling_rate=0.0001,
    max_temperature=1000.0,
    min_temperature=0.001,
    mutation_probability=0.001,
    n_elite=3,
    n_nearest=3,
)

result = instance.solve(store, "sa", cities, options)
print(f"distance: {result.total:.2f}")
print(f"route: {' → '.join(str(c) for c in result.route)}")
```

> **Note:** Python bindings map WIT kebab-case to snake_case — `platoo-epochs` → `platoo_epochs`, `cooling-rate` → `cooling_rate`, etc.

Alternatively, use [`componentize-py`](https://github.com/bytecodealliance/componentize-py) if you need to embed Python logic inside a WASM component that calls Teeline.

---

## Go

Install [`wasmtime-go`](https://github.com/bytecodealliance/wasmtime-go):

```bash
go get github.com/bytecodealliance/wasmtime-go/v27
```

Generate Go bindings from the WIT file using [`wit-bindgen-go`](https://github.com/bytecodealliance/wit-bindgen/releases):

```bash
wit-bindgen go --out-dir teeline-wasm/go-bindings teeline-wasm/wit/
```

Then call the component:

```go
package main

import (
    "fmt"
    "os"

    "github.com/bytecodealliance/wasmtime-go/v27"
    // generated bindings:
    teeline "path/to/teeline-wasm/go-bindings"
)

func main() {
    config := wasmtime.NewConfig()
    config.SetWasmComponentModel(true)
    engine := wasmtime.NewEngineWithConfig(config)

    wasm, err := os.ReadFile("target/wasm32-wasip1/release/teeline_wasm.wasm")
    if err != nil {
        panic(err)
    }

    component, err := wasmtime.NewComponent(engine, wasm)
    if err != nil {
        panic(err)
    }

    linker := wasmtime.NewLinker(engine)
    wasmtime.WasiConfigNew() // adds WASI imports required by rand
    store := wasmtime.NewStore(engine, nil)

    instance, err := teeline.NewSolver(store, component, linker)
    if err != nil {
        panic(err)
    }

    cities := []teeline.TeelineSolverTypesCity{
        {Id: 0, X: 565.0, Y: 575.0},
        {Id: 1, X: 25.0,  Y: 185.0},
        {Id: 2, X: 345.0, Y: 750.0},
        {Id: 3, X: 945.0, Y: 685.0},
        {Id: 4, X: 845.0, Y: 655.0},
    }

    options := teeline.TeelineSolverTypesSolveOptions{
        Epochs:              5000,
        PlatooEpochs:        200,
        CoolingRate:         0.0001,
        MaxTemperature:      1000.0,
        MinTemperature:      0.001,
        MutationProbability: 0.001,
        NElite:              3,
        NNearest:            3,
    }

    result, err := instance.Solve(store, "sa", cities, options)
    if err != nil {
        panic(err)
    }

    fmt.Printf("distance: %.2f\n", result.Total)
    fmt.Printf("route: %v\n", result.Route)
}
```

> **Note:** Go bindings map WIT kebab-case to PascalCase — `platoo-epochs` → `PlatooEpochs`, `cooling-rate` → `CoolingRate`, etc.

---

## Rust

The integration tests in [`tests/wasm_component.rs`](../tests/wasm_component.rs) show the full Rust host setup using `wasmtime` with `wasmtime_wasi`. See those for a complete working example.

Quick summary:

```rust
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtxBuilder, WasiView};

wasmtime::component::bindgen!({ world: "solver", path: "teeline-wasm/wit" });

// Engine needs component model enabled
let mut config = Config::new();
config.wasm_component_model(true);
let engine = Engine::new(&config)?;

// WASI linker is required — rand imports wasi:random/random
let mut linker: Linker<HostState> = Linker::new(&engine);
wasmtime_wasi::add_to_linker_sync(&mut linker)?;

let component = Component::from_file(&engine, "target/wasm32-wasip1/debug/teeline_wasm.wasm")?;
let instance = Solver::instantiate(&mut store, &component, &linker)?;

let result = instance.call_solve(&mut store, "sa", &cities, options)?;
```

---

## Spin (Fermyon)

Teeline's component targets `wasm32-wasip2`, which is compatible with the [Spin](https://spinframework.io/) runtime. You can call the Teeline component from a Spin handler using Spin's component composition support.

```toml
# spin.toml
[[component]]
id = "tsp-handler"
source = "handler.wasm"
[component.dependencies]
solver = { path = "target/wasm32-wasip1/release/teeline_wasm.wasm" }
```

The Teeline component exposes no HTTP surface — it is a pure library component. Your Spin handler imports `teeline:solver/types` and calls `solve` as a linked dependency.

---

## Notes

- **WASI required:** The component imports `wasi:random/random` (used by `rand` for seeding). Any host that instantiates this component must link a WASI implementation.
- **Exact solvers:** `bhk` and `branch_bound` find optimal solutions but have exponential/factorial complexity. They work correctly on small inputs (≤ 20 cities) but will run for a very long time on large ones — there is no built-in timeout.
- **Panic safety:** All solver panics are caught inside the component and returned as `Err(string)` so they cannot trap the host.

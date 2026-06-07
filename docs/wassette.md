# Using teeline via Wassette in Claude

[Wassette](https://microsoft.github.io/wassette/latest/) is an open-source MCP server from Microsoft that runs WebAssembly components in a sandboxed environment. It exposes WASM Component Model exports as native MCP tools inside Claude conversations.

## Prerequisites

- Claude Desktop or Claude.ai (Pro or Team)
- Wassette MCP server — see the [Wassette getting started guide](https://microsoft.github.io/wassette/latest/getting-started/)

## Loading the teeline component

In a Claude conversation, ask Claude to load the teeline component from GHCR:

```
Load component from oci://ghcr.io/timgluz/teeline/wassette:latest
```

Claude will confirm when the following tools are available:

| Tool | Description |
|------|-------------|
| `list-algorithms` | List all TSP solvers with descriptions and recommendations |
| `solve` | Solve from a list of city coordinate structs |
| `parse-and-solve` | Parse a raw TSPLIB/JSON string and solve in one call |
| `parse` | Parse a TSPLIB file and return structured city data |
| `compare` | Run multiple solvers side-by-side and return timing + distance |

## Example workflow

1. Attach a `.tsp` file or paste TSPLIB content:
   ```
   @berlin52.tsp list the available algorithms and recommend the best one
   ```

2. Claude calls `list-algorithms` and presents the options with recommendations

3. Pick a solver and solve:
   ```
   solve it with 2opt
   ```

4. Compare two solvers:
   ```
   compare nn and 2opt on this dataset
   ```
   Claude calls `compare` and shows side-by-side results including solve time in milliseconds.

## Component permissions

The teeline component runs with **zero host permissions** — no filesystem, no network, no environment variables. All computation is sandboxed inside the WASM runtime (`teeline-wasm/policy.yaml` declares this explicitly).

## Input formats

**TSPLIB format:**
```
NAME: my_problem
TYPE: TSP
DIMENSION: 5
EDGE_WEIGHT_TYPE: EUC_2D
NODE_COORD_SECTION
1 0.0 0.0
2 1.0 0.0
3 1.0 1.0
4 0.0 1.0
5 0.5 0.5
EOF
```

**JSON format:**
```json
[{"id": 1, "x": 0.0, "y": 0.0}, {"id": 2, "x": 1.0, "y": 0.0}]
```

## Solver options

| Option | Example value | Used by |
|--------|--------------|---------|
| `epochs` | 200 | all iterative solvers |
| `platoo-epochs` | 50 | stagnation threshold |
| `cooling-rate` | 0.0001 | SA |
| `max-temperature` | 1000.0 | SA |
| `min-temperature` | 0.001 | SA |
| `mutation-probability` | 0.001 | GA, CS, FPA |
| `n-elite` | 3 | GA |
| `n-nearest` | 3 | NN |

## Publishing a new version

**Automatically:** Every versioned release tag (e.g. `v1.1.0`) triggers the `publish-wassette` CI job, which pushes both `:latest` and `:v1.1.0` to GHCR.

**Manually:** Trigger `workflow_dispatch` on release.yml via the [Actions tab](https://github.com/timgluz/teeline/actions/workflows/release.yml) to build and push `:latest` without creating a full release. Note that `workflow_dispatch` only pushes `:latest` — a versioned tag (e.g. `:v1.1.0`) is only pushed when the workflow runs from a version tag (`GITHUB_REF_TYPE == "tag"`).

## Developing and testing locally

Build the wasip2 component locally and load it directly:

```bash
# Build
cargo component build --manifest-path teeline-wasm/Cargo.toml --target wasm32-wasip2

# Load in Claude via Wassette
# In Claude: Load component from file:///path/to/teeline/target/wasm32-wasip2/debug/teeline_wasm.wasm
```

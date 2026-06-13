# Fourier-basis Constructive Solver

| | |
|---|---|
| **Alias** | `fourier` |
| **Type** | Heuristic — constructive |
| **Complexity** | O(K_max · epochs · n · M) per run |

## Description

Encodes a TSP tour as a **closed curve in the complex plane** and optimises the Fourier
coefficients of that curve with gradient descent. Decoding is a pure argsort: no penalty,
no repair step, and no possibility of producing an invalid tour.

### Curve representation

The tour is the closed curve

```
γ(s) = Σ_{k=-K}^{K} c_k · exp(2πi · k · s),   s ∈ [0, 1)
```

sampled at M evenly-spaced points `s_j = j/M`. The `2K+1` complex coefficients `c_k` are
the only free variables; gradient descent moves them so the curve passes near each city.

### Energy

```
E(c) = Σ_i  min_j |city_i − γ(s_j)|²   +   λ Σ_k (2πk)² |c_k|²
         attraction                              tension
```

- **Attraction**: each city pulls its nearest curve sample toward it.
- **Tension**: a smoothness regulariser that weights high-frequency modes proportionally to
  their squared Fourier frequency, preventing jagged curves that visit no city cleanly.

### Coarse-to-fine optimisation loop

```
initialise c[0] = centroid, c[1] = radius/2, all others = 0
λ ← opts.lambda

for k_active = 1 … K_max:
    basis[k][j] = exp(2πi · ks[k] · j/M)   // pre-computed; constant within stage

    repeat opts.epochs times:
        γ = eval_curve(c, ks, M)
        grad = attraction_gradient + tension_gradient
        for k where |ks[k]| ≤ k_active:
            c[k] -= (lr / n) * grad[k]

    λ *= lambda_decay
```

Unlocking modes one stage at a time lets the optimiser set the overall loop shape (low
modes) before refining local detail (high modes), avoiding the saddle points that occur
when all modes compete simultaneously.

### Decode (always valid)

```
s_i = argmin_j |city_i − γ(s_j)|     // nearest curve sample per city
tour = argsort(s_i)                    // sort cities by their curve position
```

The argsort gives **array positions** in `cities[]`, which are then mapped to their `.id`
fields — the same pattern used by Christofides. This guarantees a valid Hamiltonian tour
regardless of convergence quality.

## Options

| Field | Default | Range | Description |
|-------|---------|-------|-------------|
| `k_max` | 4 | ≥ 1 | Maximum Fourier mode (number of frequency stages) |
| `m` | 200 | ≥ 2 | Curve sampling resolution (points on γ) |
| `lambda` | 0.05 | > 0 | Initial tension weight |
| `lambda_decay` | 0.5 | (0, 1) | Tension multiplier applied at each stage |
| `lr` | 0.05 | > 0 | Gradient descent learning rate |
| `epochs` | 400 | ≥ 1 | Gradient steps per k_active stage |

`epochs` follows the same vocabulary as all other solvers in this codebase.

## Usage

```bash
# standalone
teeline solve fourier -i ./data/tsplib/berlin52.tsp

# as warm-start for 2-opt (recommended)
teeline pipeline --steps=fourier,2opt -i ./data/tsplib/berlin52.tsp

# as warm-start for LK
teeline pipeline --steps=fourier,lk -i ./data/tsplib/berlin52.tsp
```

## Notes

- **`init_tour` is ignored**: as a purely constructive solver, Fourier does not accept or
  benefit from a seed tour; the `--init-tour` pipeline option has no effect on this solver.
- **f64 internally**: all gradient computation uses `f64` for numerical stability; coordinates
  are converted from `f32` at input and the final tour is `Vec<usize>` city IDs.
- **WASM-compatible**: uses `num-complex = "0.4"` (no_std-compatible, no libm dependency).

## Relationship to the Elastic Net

This algorithm is a Fourier-parameterised variant of the **Elastic Net** (Durbin & Willshaw
1987). Both share the same two-term energy (city attraction + curve smoothness/tension) and
optimise via gradient descent. The key differences:

| | Elastic Net | This implementation |
|---|---|---|
| Curve representation | M explicit node positions | 2K+1 Fourier coefficients |
| Tension term | Sum of squared edge lengths | `λ(2πk)²|c_k|²` (diagonal in coefficient space) |
| Mode schedule | Simultaneous + temperature annealing | Coarse-to-fine frequency unlocking |
| Tour decode | Explicit node ordering | Argsort of nearest-sample parameter `s_i` |

## References

- Durbin, R. & Willshaw, D. (1987) — "An analogue approach to the travelling salesman problem
  using an elastic net method", *Nature* **326**, 689–691 (original Elastic Net)
- Meijer, H. & Imai, H. (1992) — "The discrete Fourier transform approach to the travelling
  salesman problem", *Oper. Res. Lett.* (Fourier-basis parameterisation)

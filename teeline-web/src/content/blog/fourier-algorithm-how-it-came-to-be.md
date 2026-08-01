---
title: "Fourier algorithm — how it came to be"
description: "A ~50-line TSP solver that encodes a tour as a closed curve in the complex plane, decodes with a plain argsort, and was designed together with Claude Opus."
pubDate: 2026-08-01
tags: ["algorithms", "fourier", "ai-collaboration", "tsp"]
draft: false
---

[teeline](/) ships 18 TSP algorithms. Most of them look the way you'd expect a heuristic to look: a permutation, a neighbourhood move, a repair step for when the move breaks validity. One of them doesn't. The [Fourier-basis constructive solver](/algorithms/fourier/) never touches a permutation until the very last line — it optimises a handful of complex numbers instead, and the tour falls out as a side effect of sorting them.

It's also the shortest solver in the codebase by a wide margin, and it started life as a brainstorming session with Claude Opus rather than a paper I was implementing. This post is about the idea, and honestly, about that session too.

## The idea: a tour as a closed curve

The starting point is the [Elastic Net](https://doi.org/10.1038/326689a0) (Durbin & Willshaw, 1987): stretch a loop of points across the city map and let it get pulled toward every city while a tension term keeps it smooth. Read off the tour by walking the loop in order. It's an elegant idea, but the loop itself is represented the obvious way — as M explicit 2-D points that all have to move, all have to stay smooth relative to their neighbours, and all have to be annealed together.

The Fourier version keeps the "curve pulled toward cities" idea and throws out the explicit points. Instead, the curve is a truncated Fourier series in the complex plane:

```text
γ(s) = Σ_{k=-K}^{K} c_k · exp(2πi · k · s),   s ∈ [0, 1)
```

The only free variables are the `2K+1` complex coefficients `c_k` — typically 9 of them (`K=4`). Move the coefficients, and the whole curve moves as a unit; there's no separate smoothness constraint to enforce because smoothness is just "don't put weight on high-frequency modes."

## Energy: attraction and a one-line tension term

Fitting the curve to the cities is gradient descent on a two-term energy, same shape as the Elastic Net's:

```text
E(c) = Σ_i  min_j |city_i − γ(s_j)|²   +   λ Σ_k (2πk)² |c_k|²
         attraction                              tension
```

Attraction pulls each city's nearest curve sample toward it, same as the Elastic Net. But the tension term is where the Fourier basis pays off: because `(2πk)²|c_k|²` only depends on `c_k` itself, the tension gradient for coefficient `k` is just `λ(2πk)²c_k` — no coupling between coefficients, no matrix, closed form. In the Elastic Net's explicit-point representation the equivalent term is a sum over edge lengths between neighbouring points, which needs the whole loop to update coherently. Here it's diagonal.

## Coarse-to-fine: unlocking frequencies like a synth patch

Rather than optimise all 9 coefficients from a random start, the solver unlocks them one frequency at a time:

```rust
for k_active in 1..=opts.k_max {
    for _ in 0..opts.epochs {
        gradient_step(&mut c, &ks, &basis, &cities_cx, lambda, opts.lr, k_active);
    }
    lambda *= opts.lambda_decay;
}
```

`gradient_step` only updates coefficients where `|k| ≤ k_active`, so stage 1 can only move `c_0` (the centroid) and `c_{±1}` (the fundamental — literally a circle). Stage 2 unlocks the next harmonic, and so on up to `K_max`. The [interactive explainer's](/algorithms/fourier/explainer/) "Modes" tab shows this directly: the curve starts as a single point at the centroid, becomes a circle once `k=1` is unlocked, and picks up progressively more detail as higher modes come online.

This isn't just a nice animation — it's why the optimiser doesn't get stuck. Trying to fit all 9 coefficients simultaneously means the gross shape and the fine detail are fighting for the same gradient step, which is exactly the kind of setup that produces saddle points. Deciding the loop shape first, then refining it, sidesteps that.

## Decode: argsort, always valid

```rust
fn decode_tour(gamma: &[Complex<f64>], cities: &[KDPoint]) -> Vec<usize> {
    let sample_indices: Vec<usize> = cities
        .iter()
        .map(|c| nearest_sample(gamma, Complex::new(c.coords[0] as f64, c.coords[1] as f64)))
        .collect();
    let mut order: Vec<usize> = (0..cities.len()).collect();
    order.sort_by_key(|&i| sample_indices[i]);
    order.iter().map(|&pos| cities[pos].id).collect()
}
```

Each city finds its nearest point on the curve, then cities get sorted by where on the curve they landed. That's the entire decode step. It always produces a valid Hamiltonian tour — every city appears exactly once, because every city contributes exactly one sort key — regardless of how well the curve has converged. Compare that to a Hopfield-Tank network, the other classic "physics metaphor for TSP": it needs fragile penalty terms in the energy function just to discourage a neuron activation pattern that isn't a valid tour at all, and even then isn't guaranteed to avoid one.

## Designing it with Opus

I didn't derive this from a paper — I brainstormed it with Claude Opus, working from the Elastic Net as a starting point and asking what a Fourier reparameterisation would look like. What came out of that session was a validated Python prototype (exact 0.00% gap on both a 5-city circle and 8 random uniform cities) plus a handover document to bring into Claude Code for the Rust implementation.

My own reaction, reading the finished prototype:

> "Wow, the core algorithm is much shorter than I expected, less than 50 lines"

Opus's read on why:

> "When an algorithm is fighting the problem structure you end up with pages of bookkeeping — penalty terms, validity repairs, tabu lists, population management. The Fourier formulation has almost none of that because the encoding does the work."

The line I keep coming back to, though, is the comparison to teeline's other continuous-optimisation solvers — PSO, Cuckoo Search, Flower Pollination — which all have to bolt velocity caps, swap-sequence arithmetic, or Lévy-flight perturbations onto a fundamentally discrete permutation just to make a continuous algorithm work there at all:

> "The PSO/CS/FPA family is 'continuous algorithm awkwardly adapted to discrete space.' This is the opposite: 'discrete problem naturally lifted into continuous space.' That inversion is probably why it's short."

That's not a line I'd have arrived at on my own mid-implementation — it took stepping back from the code to see the pattern across solvers I'd already written. Worth being honest about what this transcript actually is, too: it's the handover point, not a blow-by-blow of every dead end along the way. What it captures well is the reasoning behind the design, and a prototype that was already proven correct before a single line of Rust existed.

## Where it sits among neighbours

Against the Elastic Net specifically:

| | Elastic Net | Fourier solver |
| --- | --- | --- |
| Curve representation | M explicit node positions | 2K+1 Fourier coefficients |
| Tension term | Sum of squared edge lengths | `λ(2πk)²\|c_k\|²` (diagonal) |
| Mode schedule | Simultaneous + temperature annealing | Coarse-to-fine frequency unlocking |
| Tour decode | Explicit node ordering | Argsort of nearest-sample parameter |

But the closer sibling, inside teeline itself, is the [Kohonen Self-Organizing Map](/algorithms/som/). Both are constructive, both are neural/topology-inspired, and both share the same decode guarantee: SOM sorts cities by their neuron's ring index, Fourier sorts by curve parameter — an argsort either way, over a different index, always valid regardless of convergence. Where they actually differ is where the free parameters live. SOM's neurons are explicit 2-D points on a ring, trained one random city at a time via best-matching-unit search and a Gaussian neighbourhood pull — structurally closer to the Elastic Net, just discretised into neurons. The Fourier solver has no online per-city training loop at all: every city pulls on every coefficient, every step, in plain batch gradient descent.

## Does it actually work?

On berlin52 (52 cities, optimal cost 7 544.37), re-run just now against the current release build:

| Solver | Gap from optimal | Wall time |
| --- | ---: | ---: |
| Nearest Neighbour | +19.0 % | 0.00 s |
| **Fourier** (standalone, `--no-seed`) | **+13.3 %** | 0.17 s |
| **Fourier + 2-opt** | **+5.4 %** | 0.18 s |
| 3-opt (best deterministic) | +2.6 % | 0.3 s |
| Lin-Kernighan (default) | 0.0 % | 0.15 s |

Those gap numbers match [`docs/benchmarks.md`](https://github.com/timgluz/teeline/blob/master/docs/benchmarks.md) almost exactly (13.3% / 5.4% / 19.0%, originally measured on v1.0.1). The wall time doesn't — the original benchmark clocked Fourier at 2.5s standalone and 1.9s piped into 2-opt; today's release build does both in under 0.2s, thanks to an `OnceLock`-cached basis matrix added after that benchmark was written. Same math, ~14x faster.

On quality, let's be direct about it: Fourier alone isn't a leaderboard-topper. 3-opt, Cuckoo Search, and Lin-Kernighan all beat it. What it does offer: it's fully deterministic (the initial coefficients come straight from the city centroid and mean radius, no RNG anywhere), so unlike SA/GA/PSO/CS it returns the exact same tour on every run. Alone, it already beats a nearest-neighbour tour. Piped into 2-opt, it lands at 5.4% — comparable to what teeline's SOM+2-opt table lists as its typical range (~3–7%, not yet re-measured against the current build). It's a good, principled *starting point* for local search, not a solver you'd reach for on its own.

## Try it yourself

- Read the full spec: [teeline algorithms → Fourier-basis Constructive Solver](/algorithms/fourier/)
- Watch it converge: the [interactive explainer](/algorithms/fourier/explainer/) — step through the coarse-to-fine schedule frequency by frequency, or run live gradient descent in the browser
- Solve something on [tspsolver.com](/): upload a `.tsp` file and pick `fourier` from the algorithm list (or `fourier` → `2opt` in the pipeline builder)
- CLI:

  ```bash
  teeline solve fourier -i ./data/tsplib/berlin52.tsp --optimal-tour ./data/tsplib/berlin52.opt.tour
  teeline pipeline --steps=fourier,2opt -i ./data/tsplib/berlin52.tsp
  ```

- REST API ([get a key first](/api-key/)):

  ```bash
  curl -X POST https://api.tspsolver.com/api/v1/solve \
    -H "Authorization: Bearer ak_your_key_here" \
    -H "Content-Type: application/json" \
    -d '{
      "solver": "fourier",
      "input": { "tsplib": "<TSPLIB text>" },
      "configs": { "fourier": { "k_max": 4, "epochs": 400 } }
    }'
  ```

- MCP, via [Wassette](/webmcp/): `Load component from oci://ghcr.io/timgluz/teeline/wassette:latest`, then just say "solve it with fourier"
- WebMCP, straight from the browser: any agent connected per the [AI agent access page](/webmcp/) can call `solveTSP` with `algorithm: "fourier"` — same mechanism as the [WebMCP post](/blog/webmcp-tsp-solver/), different algorithm.

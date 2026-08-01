---
title: "Fourier algorithm: how it came to be"
description: "A TSP solver that encodes a tour as a closed curve in the complex plane and decodes with a plain argsort, designed together with Claude Opus, starting from a 37-line Python prototype."
pubDate: 2026-08-01
tags: ["algorithms", "fourier", "ai-collaboration", "tsp"]
draft: false
---

[teeline](/) ships 18 TSP algorithms. Most of them look the way you'd expect a heuristic to look: a permutation, a neighbourhood move, a repair step for when the move breaks validity. One of them doesn't. The [Fourier-basis constructive solver](/algorithms/fourier/) never touches a permutation until the very last line. It optimises a handful of complex numbers instead, and the tour falls out as a side effect of sorting them.

It's also the only solver in the codebase with no penalty term, no validity repair, and no population bookkeeping anywhere in it. I didn't derive it from a paper; I brainstormed it with Claude Opus, and I'll get to that. First, the idea itself.

## The idea: a tour as a closed curve

The starting point is the [Elastic Net](https://doi.org/10.1038/326689a0) (Durbin & Willshaw, 1987): stretch a loop of points across the city map and let it get pulled toward every city while a tension term keeps it smooth. Read off the tour by walking the loop in order. It's an elegant idea, but the loop itself is represented the obvious way: as M explicit 2-D points that all have to move, all have to stay smooth relative to their neighbours, and all have to be annealed together.

The Fourier version keeps the "curve pulled toward cities" idea and throws out the explicit points. Instead, the curve is a truncated Fourier series in the complex plane:

```text
γ(s) = Σ_{k=-K}^{K} c_k · exp(2πi · k · s),   s ∈ [0, 1)
```

The only free variables are the `2K+1` complex coefficients `c_k`, typically 9 of them (`K=4`). Move the coefficients, and the whole curve moves as a unit; there's no smoothness coupling between neighbouring points to maintain, because truncating at K already bounds how wiggly the curve can get.

## Energy: attraction and a one-line tension term

Fitting the curve to the cities is gradient descent on a two-term energy, same shape as the Elastic Net's:

```text
E(c) = Σ_i  min_j |city_i − γ(s_j)|²   +   λ Σ_k (2πk)² |c_k|²
         attraction                              tension
```

Attraction pulls each city's nearest curve sample toward it, same as the Elastic Net. But the tension term is where the Fourier basis pays off: because `(2πk)²|c_k|²` only depends on `c_k` itself, the tension gradient for coefficient `k` is just `λ(2πk)²c_k`: no coupling between coefficients, no matrix, closed form. In the Elastic Net's explicit-point representation the equivalent term is a sum over edge lengths between neighbouring points, which needs the whole loop to update coherently. Here it's diagonal.

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

`gradient_step` only updates coefficients where `|k| ≤ k_active`, so stage 1 can only move `c_0` (the centroid) and `c_{±1}` (the fundamental, literally a circle). Stage 2 unlocks the next harmonic, and so on up to `K_max`. It's the same idea as building a sound on a synthesizer by setting the fundamental frequency first and layering harmonics on top, rather than twisting every oscillator at once. The [interactive explainer's](/algorithms/fourier/explainer/) "Modes" tab shows the visual version: the curve starts as a single point at the centroid, becomes a circle once `k=1` is unlocked, and picks up progressively more detail as higher modes come online.

This isn't just a nice animation: it's why the optimiser doesn't get stuck. Trying to fit all 9 coefficients simultaneously means the gross shape and the fine detail are fighting for the same gradient step, which is exactly the kind of setup that produces saddle points. Deciding the loop shape first, then refining it, sidesteps that.

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

Each city finds its nearest point on the curve, then cities get sorted by where on the curve they landed. That's the entire decode step. It always produces a valid Hamiltonian tour (every city appears exactly once, because every city contributes exactly one sort key), regardless of how well the curve has converged. Compare that to a Hopfield-Tank network, the other classic "physics metaphor for TSP": it needs fragile penalty terms in the energy function just to discourage a neuron activation pattern that isn't a valid tour at all, and even then isn't guaranteed to avoid one.

## Designing it with Opus

That argsort decode is also the right place to explain how this came to exist. I brainstormed the whole thing with Claude Opus, working from the Elastic Net above and asking what a Fourier reparameterisation of it would look like. What came out of that session was a Python prototype (37 lines, including the brute-force check used to validate it against the true optimum) and a handover document to bring into Claude Code for the Rust implementation. The prototype matched brute force on the two toy instances I could check exhaustively, a 5-city circle and 8 random uniform cities, before a single line of Rust existed.

Opus's read on why it stayed that short:

> "When an algorithm is fighting the problem structure you end up with pages of bookkeeping — penalty terms, validity repairs, tabu lists, population management. The Fourier formulation has almost none of that because the encoding does the work."

I was genuinely surprised reading that prototype: I'd expected something closer to the length of teeline's other constructive solvers. The production Rust port, once it's wearing real types and integrated with the rest of the codebase, comes to about 140 lines outside the test suite: one orchestrating `solve()` function plus seven small named helpers (`ks_array`, `init_coefficients`, `compute_basis`, `gradient_step`, `eval_curve`, `nearest_sample`, `decode_tour`). Longer than the prototype, as you'd expect, and it turns out not the shortest solver in the codebase either (a few of the simpler local-search solvers are shorter), but the no-bookkeeping shape survived the translation intact. See for yourself in [the source](https://github.com/timgluz/teeline/blob/master/src/tsp/fourier.rs) (`src/tsp/fourier.rs`): no penalty terms, no validity repair, no population bookkeeping anywhere in it.

The line I keep coming back to, though, is the comparison Opus drew to teeline's other continuous-optimisation solvers (PSO, Cuckoo Search, Flower Pollination), which all have to bolt velocity caps, swap-sequence arithmetic, or Lévy-flight perturbations onto a fundamentally discrete permutation just to make a continuous algorithm work there at all:

> "The PSO/CS/FPA family is 'continuous algorithm awkwardly adapted to discrete space.' This is the opposite: 'discrete problem naturally lifted into continuous space.' That inversion is probably why it's short."

The inversion is the real insight, even though the "it's short" part didn't hold up: PSO and Cuckoo Search, two of the three solvers being contrasted with here, turn out to be shorter in Rust than the Fourier solver is. What's actually true, and checkable, is the no-bookkeeping part: no velocity caps, no swap-sequence arithmetic, no Lévy-flight perturbations bolted onto a permutation, because the discretisation only happens once, at decode time. That's the property worth crediting to "lifting the problem into continuous space", not line count.

Worth being honest about what the handover document actually is, too: it's the point where the design was handed off for implementation, not a blow-by-blow of every dead end along the way. What it captures well is the reasoning behind the design, and a prototype that already matched brute force before implementation started.

As far as I know, this specific combination (parameterising a TSP tour's curve directly as Fourier coefficients rather than explicit points, with coarse-to-fine mode unlocking) hasn't been published elsewhere. If you know of prior art doing exactly this, I'd like to hear about it.

## Where it sits among neighbours

Against the Elastic Net specifically:

| | Elastic Net | Fourier solver |
| --- | --- | --- |
| Curve representation | M explicit node positions | 2K+1 Fourier coefficients |
| Tension term | Sum of squared edge lengths | `λ(2πk)²\|c_k\|²` (diagonal) |
| Mode schedule | Simultaneous + temperature annealing | Coarse-to-fine frequency unlocking |
| Tour decode | Explicit node ordering | Argsort of nearest-sample parameter |

But the closer sibling, inside teeline itself, is the [Kohonen Self-Organizing Map](/algorithms/som/). Both are constructive, both are neural/topology-inspired, and both share the same decode guarantee: SOM sorts cities by their neuron's ring index, Fourier sorts by curve parameter: an argsort either way, over a different index, always valid regardless of convergence. Where they actually differ is where the free parameters live. SOM's neurons are explicit 2-D points on a ring, trained one random city at a time via best-matching-unit search and a Gaussian neighbourhood pull. That's structurally closer to the Elastic Net, just discretised into neurons. The Fourier solver has no online per-city training loop at all: every city pulls on every coefficient, every step, in plain batch gradient descent.

On berlin52, Fourier lands at 13.3% standalone and 5.4% with 2-opt, both deterministic. SOM is stochastic (it picks a random city each training epoch), so it's a range rather than a fixed number: across several runs I saw roughly 10-21% standalone and 5-10% with 2-opt. Fourier sits inside SOM's spread on both counts, which makes "comparable" the fair word; on a different run, SOM could just as easily come out ahead.

## Where it struggles

The clearest failure case I found is [a280](https://comopt.ifi.uni-heidelberg.de/software/TSPLIB95/tsp.html) (280 cities): Fourier standalone lands at **+103.4%**, more than double the optimal tour length. Nearest-neighbour on the same instance is rough too, and varies run to run (I saw roughly 22-38% across five runs), but even its worst run is still less than half of Fourier's gap. The default `k_max=4` gives only 9 coefficients, and the default curve resolution `m=200` gives only 200 sample points to decode against; neither scales with city count, and at 280 cities both limits show. Neither is exposed on the `solve` CLI or the pipeline TOML config (`fourier` isn't a recognised per-stage override there), so right now the REST API is the only way to raise them past their defaults.

Piped into 2-opt, a280 recovers a lot of that gap, the same pattern that shows up on the two smaller instances below.

I also tested [pr76](https://comopt.ifi.uni-heidelberg.de/software/TSPLIB95/tsp.html) (76 cities, Padberg/Rinaldi) on a theory the design conversation raised: the projection/decode step (finding each city's nearest curve sample) was expected to degrade on layouts less amenable to a smooth loop. Standalone gap jumps to **+26.4%**, against 13.3% on berlin52. Piped into 2-opt, though, pr76 actually comes out *better* than berlin52: **+0.9%** versus 5.4%.

I also tried a GEO-distance instance ([gr96](https://comopt.ifi.uni-heidelberg.de/software/TSPLIB95/tsp.html), an Africa subproblem measured in latitude/longitude) on the theory that fitting a curve directly against raw lat/long coordinates (rather than true great-circle distance) should hurt more. It landed at +22.75% standalone, +6.65% with 2-opt: degraded from berlin52, but not measurably worse than pr76's plain-Euclidean case. So within what I tested, GEO coordinates aren't a distinct failure mode of their own; the real risk factor is city count outrunning `k_max` and `m`, which a280 shows clearly.

## Does it actually work?

On berlin52 (52 cities, optimal cost 7 544.37), re-run just now against the current release build:

| Solver | Gap from optimal |
| --- | ---: |
| Nearest Neighbour | +19.0 % |
| **Fourier** (standalone, `--no-seed`) | **+13.3 %** |
| **Fourier + 2-opt** | **+5.4 %** |
| 3-opt (best deterministic) | +2.6 % |
| Lin-Kernighan (default) | 0.0 % |

Those gap numbers match [`docs/benchmarks.md`](https://github.com/timgluz/teeline/blob/master/docs/benchmarks.md) exactly, down to the tour cost (8 549.14 standalone, 7 948.88 with 2-opt), despite that table being recorded against v1.0.1. I'm leaving wall time out of this table on purpose: it's not settled enough to publish yet, since a proper cross-release benchmarking setup is still in progress.

Fourier alone isn't a leaderboard-topper. Six of teeline's solvers beat it standalone on berlin52: 3-opt, Lin-Kernighan, Cuckoo Search, Simulated Annealing, Or-opt, and the Genetic Algorithm. What it does offer: it's fully deterministic (the initial coefficients come straight from the city centroid and mean radius, no RNG anywhere), so unlike SA/GA/PSO/CS it returns the exact same tour on every run. Alone, it already beats a nearest-neighbour tour on instances up to roughly 100 cities; see "Where it struggles" above for where that stops holding. Piped into 2-opt, it lands at 5.4%, in the same tier as SOM+2-opt (roughly 5-10%, above) and SA. It's a good, principled *starting point* for local search, not a solver you'd reach for on its own.

## Try it yourself

The best place to start is the [interactive explainer](/algorithms/fourier/explainer/): step through the coarse-to-fine schedule frequency by frequency, or run live gradient descent right in the browser and watch the curve shape itself around the cities.

From there:

- Read the full spec: [teeline algorithms → Fourier-basis Constructive Solver](/algorithms/fourier/)
- Solve something on [tspsolver.com](/): upload a `.tsp` file and pick `fourier` from the algorithm list (or `fourier` → `2opt` in the pipeline builder)
- CLI (run `./download_data.sh` once first to fetch the TSPLIB instances used here):

  ```bash
  teeline solve fourier -i ./data/tsplib/berlin52.tsp --optimal-tour ./data/tsplib/berlin52.opt.tour
  teeline pipeline --steps=fourier,2opt -i ./data/tsplib/berlin52.tsp
  ```

- REST API ([get a key first](/api-key/)), currently the only place `k_max` and the other tuning knobs are exposed:

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

- It's also reachable from Claude via [Wassette](/webmcp/) (`Load component from oci://ghcr.io/timgluz/teeline/wassette:latest`, then just say "solve it with fourier"), and from any WebMCP-connected agent using the same `solveTSP` tool as the [WebMCP post](/blog/webmcp-tsp-solver/), just with `algorithm: "fourier"`.

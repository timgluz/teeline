---
id: "eax"
name: "Memetic EAX"
typeBadge: "Heuristic — memetic evolutionary search"
description: "Memetic Edge Assembly Crossover: a population of locally optimised tours recombined through alternating A/B edge cycles, with subtour patching and 2-opt polish."
hasExplainer: false
---

# Memetic EAX

<!-- The #benchmarks anchor is rendered by AlgorithmBenchmarks, not by a heading in this file. -->
<!-- markdownlint-disable MD051 -->
| | |
| --- | --- |
| **Alias** | `eax`, `memetic`, `memetic_eax` |
| **Type** | Heuristic — memetic evolutionary search |
| **Complexity** | O(epochs · pop · children · n²) |
| **Warm start** | Consumes the previous pipeline stage's tour as one population member |
| **Benchmarks** | [published results ↓](#benchmarks) |

## Description

Memetic EAX keeps a population of locally optimised tours and recombines two parents by
**Edge Assembly Crossover (EAX)**. The union of the two parent edge sets is decomposed into
alternating A/B cycles: each city pairs its A-only edges with its B-only edges, which yields
disjoint even cycles. Applying a selected E-set to parent A as a symmetric difference gives
every city exactly one new neighbour, so degree two is preserved. Any subtour the exchange
creates is patched back into a single cycle by greedily merging the smallest component with
candidate-list edges. Offspring are polished with a candidate-restricted 2-opt pass seeded by
the disturbed vertices, and a parent is replaced only by its own strictly cheaper child
(family-based replacement), which keeps population niches instead of letting one global best
take over.

The solver is symmetric-only: `TYPE: ATSP` is rejected by the TSPLIB parser, and every move
here assumes an undirected edge.

```text
procedure MemeticEAX(cities, epochs, pop_size, children_per_pair):
    candidates ← k-nearest-neighbour lists
    population ← polished randomised constructive starts (+ warm-start seed)
    best ← cheapest member
    for epoch in 1..epochs:
        for each distinct pair (A, B) in a shuffled ring:
            child ← ∅
            for _ in 1..children_per_pair:
                cycles ← AB-cycles(A, B)
                E      ← select(cycles, strategy)
                tour   ← patch(A ⊕ E, candidates)
                tour   ← dirty_2opt(tour, candidates)
                child  ← cheaper(child, tour)
            if child < A: replace A with child
        if best improved: reset stagnation else stagnate++
    return best
```

## Options

| Field | CLI flag | Default | Range | Description |
| ------- | ------- | --------- | ------- | ------------- |
| `epochs` | `--epochs` | 500 | ≥ 0 | Maximum generations; `0` means no epoch cap |
| `pop_size` | `--pop-size` | 100 | ≥ 2 | Population size |
| `children_per_pair` | `--children-per-pair` | 15 | ≥ 1 | Offspring considered per parent |
| `strategy` | `--strategy` | `hybrid` | `single_ab`, `rand`, `hybrid` | E-set selection |
| `stall_epochs` | `--stall-epochs` | 50 | ≥ 1 | `hybrid` switches to `single_ab` after this many stagnant generations; the solver stops at twice this count |
| `candidate_k` | `--candidate-k` | 20 | ≥ 1 | Nearest-neighbour list size for patching and local search |
| `seed` | `--seed` | system entropy | any `u64` | Fixed RNG seed for reproducible runs |

`--stall-epochs` falls back to `--stagnation_epochs` and `--candidate-k` to `--n_nearest`
when the EAX-specific flag is omitted. All fields are also reachable through the REST API's
`configs.memetic_eax`, a TOML `[memetic_eax]`/`[stage.memetic_eax]` table, or the WASM
`SolveOptions` fields (`eaxPopSize`, `eaxChildrenPerPair`, `eaxStrategy` with
`0 = single_ab, 1 = rand, 2 = hybrid`, `eaxStallEpochs`, `eaxCandidateK`).

## Usage

```bash
# Standalone
teeline solve eax -i data/tsplib/berlin52.tsp --seed 1

# Warm-started after a constructive stage
teeline pipeline --steps=nn,eax -i data/tsplib/berlin52.tsp --seed 1

# TOML pipeline stage
cat > eax.toml <<'EOF'
[[stage]]
solver = "nn"

[[stage]]
solver = "memetic_eax"

[stage.memetic_eax]
pop_size = 50
children_per_pair = 8
strategy = "hybrid"
seed = 1
EOF
teeline pipeline --config eax.toml -i data/tsplib/berlin52.tsp
```

## Notes

EAX is heavier per epoch than Lin-Kernighan or 2-opt: each child runs a crossover, a
patching pass, and a local 2-opt polish. On berlin52 and kroA100 the default budget reaches
the published optimum; a280 and dsj1000 approach it. For a tighter wall-clock budget, lower
`pop_size` and `children_per_pair` first — both scale the per-epoch cost linearly, while
`strategy = "single_ab"` reduces the crossover work further.

The published scores for this algorithm are indexed under `eax` in
[`bench/solvers.json`](../../bench/solvers.json); the TSPLIB optimum is the ground truth,
never the solver's own output.

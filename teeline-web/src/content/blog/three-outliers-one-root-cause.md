---
title: "Three outliers, one root cause: fixing PSO, FPA and GSA"
description: "How three swarm solvers — flocking birds, pollinating flowers, Newton's gravity — were stranded at 150–250% median gap on the TSP benchmark, and how one fix brought them into the pack."
pubDate: 2026-11-22
tags: ["algorithms", "benchmarking", "tsp", "heuristics"]
draft: false
---

On the 12-map Tier A benchmark, most local-search and constructive solvers finish in the single digits or low twenties. Three did not: PSO at 150.35% median gap, FPA at 253.94%, GSA at 190.52%. On the largest map, dsj1000 with 1,000 cities, those gaps ran to 2,340%, 2,655% and 2,504%.

They were not slow. All three finish in about a second or two on average, against a 300-second budget. They were fast and bad.

The landing chart tells a milder version of the same story: it plots berlin52 alone, and there the trio sat at 110–199% while the rest of the field clustered lower. The percentages in this post are the 12-map medians, where the large instances dominate.

PSO is modelled on a flock of birds, FPA on pollinating flowers, GSA on Newton's gravity. Different metaphors, same failure — and, it turned out, the same fix.

## The paper gives the dynamics; the TSP grounding was ours

Each paper defines its update in continuous space: a particle moves by a velocity, a flower takes a Lévy-scaled step toward the best, an agent is pulled by a gravitational force. On a map, a solution is a permutation of cities. We translated "move toward the best" into "swap two city positions" — the obvious textbook move, and the wrong one.

In continuous space, a small step is small. On a permutation, the nearest change in *list* distance can be a huge change in *tour* cost, and the two are not the same thing.

A tour is a cycle of edges. One swap can break four of them at once:

```text
tour:      A B C D E F
swap B, E: A E C D B F
```

The edges `A–B`, `B–C`, `D–E` and `E–F` are gone. Four edges replaced for one "small" move. Worse, the same cycle can be written six ways — `A B C D E F`, `B C D E F A`, and so on — and a rank-by-rank comparison sees those as completely different tours. So a move "toward the best" spends its budget undoing a rotation that costs nothing. The trio also started from random permutations and had no local search to recover.

## The fix, in one sentence

A tour is made of edges, so change edges, not list positions.

The shared playbook was:

- **Order crossover.** Copy a contiguous segment of a good tour, then fill the remaining cities in the current tour's order. The child inherits the target's internal edges and breaks only the junctions.
- **A nearest-neighbour seed.** Half the population starts from constructive tours; the other half stays random for diversity.
- **A final 2-opt polish.** Uncross edges until no simple swap shortens the loop.

Each solver kept its own character. PSO still pulls toward personal and global bests; FPA still has its Lévy-scaled global move and ε-scaled local move; GSA still weights attraction by mass, and now shrinks its Kbest set from N to 1 the way Rashedi's paper does. What changed is the move underneath: an order crossover instead of a rank swap.

The shared crossover, seeding and polish also meant the second and third fixes were mostly wiring — the first one built the plumbing. This is a **memetic adaptation**, not a new published algorithm: a population method for exploration, a local search for exploitation. Each solver's page says so.

## The numbers

![A dumbbell chart of median gap before and after: PSO 150.4% to 6.9%, FPA 253.9% to 4.1%, GSA 190.5% to 2.6%.](/blog/swarm-retrospective.svg)

| Solver | Median gap before | After | dsj1000 before → after |
| --- | ---: | ---: | ---: |
| PSO | 150.35% | **6.88%** | 2,340.27% → 11.17% |
| FPA | 253.94% | **4.07%** | 2,654.97% → 9.18% |
| GSA | 190.52% | **2.59%** | 2,503.87% → 10.13% |

Median across the 12 Tier A maps, five runs each. The headline median is helped by the small maps, where all three now hit the optimum; on the larger maps they land at roughly 5–10%, alongside the other metaheuristics.

## What actually solved the 1,000-city maps

On 52–100 cities, the swarm does real work: the population starts around 25–50% and the loop pulls it down to single digits. By 280–1,000 cities, the same loop stalls near its nearest-neighbour seed, and the final 2-opt polish supplies most of the gain.

That is not a hidden failure; it is the normal shape of a memetic algorithm. The population explores, the local search finishes. It is the same reason Memetic EAX polishes every child with 2-opt, and each solver's changelog states it plainly.

## What this taught us

The neighbourhood dominates. The population dynamics in all three papers were fine — the swarm, the pollination, the gravity. The move was what made them look broken. Any metaphor collapses if its move destroys the thing it is supposed to optimize.

The benchmark chart was a diagnostic, not a scoreboard. It did not just rank the solvers; it asked why three dots were stranded, and the answer was the same for all three. That question only became visible after the code worked and after the cleanup: measurement is what turned "produces valid tours" into "good or bad".

## Go deeper

- [PSO](/algorithms/pso/), [FPA](/algorithms/fpa/) and [GSA](/algorithms/gsa/) — each with a `2.0.0` changelog and its benchmark table.
- The [EAX post](/blog/eax-np-hard-tsp/) makes the same "edges, not order" argument from the other direction.
- Run any of them on [the solver](/solve/).

The next time a chart looks wrong, the algorithm may be fine. Check the move.

---
title: "The Traveling Salesman Problem Is NP-Hard. This Algorithm Still Nails 24,978 Cities."
description: "TSP is famously NP-hard, yet Edge Assembly Crossover finds optimal routes on 24,978-city maps by evolving roads instead of city lists."
pubDate: 2026-11-22
tags: ["algorithms", "eax", "tsp", "complexity"]
draft: false
---

Ask for the poster child of an impossible problem and you'll get the Traveling Salesman Problem: given a set of cities and the distances between them, find the shortest loop that visits every city exactly once and returns home.

The reason it's famous is arithmetic. With 60 cities on a symmetric map there are 59!/2 possible tours: about 7 × 10⁷⁹, a 7 followed by 79 zeros. That is roughly the number of atoms in the observable universe. Start checking tours at one per second at the Big Bang and you would have made no meaningful progress by today. Ten cities is a puzzle. Twenty is out of reach for brute force. A few dozen is hopeless. This is also why TSP sits at the centre of the P vs NP question: checking a proposed route is trivial, but nobody knows an algorithm that is guaranteed to find the shortest one quickly on every possible map. Find one and you have settled P vs NP, along with a Millennium Prize.

The escape hatch is that "no guaranteed fast algorithm" is a statement about the worst case, not about every map. You give up the guarantee and buy speed with cleverness. The resulting methods are called heuristics, and after decades of work they are astonishingly good — often finding tours that later turn out to be optimal, on maps where brute force would never finish.

The algorithm I want to tell you about is a genetic algorithm. That is the surprising part. Genetic algorithms have a bad reputation on TSP, for a good reason I'll get to, and they are not what you picture when you picture the state of the art. Yet in 2006 Yuichi Nagata [reported](https://doi.org/10.1007/11844297_38) that his Edge Assembly Crossover found optimal tours for benchmark instances of up to 24,978 cities within a day, on a single 1.3 GHz processor. The [2013 paper](https://doi.org/10.1287/ijoc.1120.0506) with Shigenobu Kobayashi pushed top-tier results to instances of 200,000 cities — without Lin–Kernighan, the local-search engine behind essentially every other state-of-the-art TSP heuristic.

## A tour is roads, not a list

Most genetic algorithms treat a tour as a list of cities in visiting order, and crossover as a list operation: cut two parent lists at a point, swap the tails, repair the result.

That is reasonable for a list. It is a bad way to handle a route, because the list is only a notation for the thing that actually matters — the set of roads between cities.

Two excellent tours can be written in completely different orders and still agree on most of their roads. Those shared roads are the reason both tours are good. A list crossover has no idea they exist. It cuts wherever the list happens to be cut:

```text
parent 1:  A B | C D E F
parent 2:  A D | C B E F
child:     A B + C B E F   →   B appears twice, D is gone
```

Before that child is even a valid tour, a repair step has to remove the duplicate and fill the hole — and the repair is free to tear out exactly the roads both parents agreed on. The offspring ends up worse than either parent, and the population spends its generations rediscovering what it already had. This is why "just use a GA" so often disappoints on TSP.

EAX asks the opposite question: what if crossover never touched the order at all, and only ever exchanged edges?

## The move, in three beats

**Overlay the parents.** Put both tours on the same map and keep every road they share. At each city where the tours disagree, pair up the edges that exist only in tour A with those that exist only in tour B, then follow those pairings. They close into loops that alternate A-edge, B-edge, A-edge, B-edge. The papers call them AB-cycles; think of them as alternating loops. Each loop is a place where the two parents genuinely disagree.

**Swap a few loops.** Choose some of those loops and exchange them, keeping exactly the edges that belong to one parent and not the other. At every city one edge leaves and one arrives, so each city still has exactly two roads — the tour never loses its shape. What it usually loses is being a single loop: the result is several disconnected mini-loops.

**Stitch and polish.** Patch the mini-loops back into one tour, merging the smallest first and preferring short candidate edges. Here is the quiet magic of the whole method: the stitch is allowed to use a road that was in neither parent. That is mutation for free, delivered precisely where the crossover broke the tour. A quick 2-opt pass — uncross edges until no simple swap shortens the loop — then cleans up the seams. A genetic algorithm with a local search bolted on like this is called memetic, which is where the solver's name comes from.

The [interactive explainer](/algorithms/eax/explainer/) steps through all of this in six stages, with actual tours drawn on the map. If the three beats above feel abstract, scrub through it once — the picture does more than another paragraph would.

## Quietly extraordinary — and the catch

The usual currency for a TSP heuristic is gap: how much longer its tour is than the best known one, as a percentage. Zero means it tied the optimum.

teeline ships 23 algorithms. On the 12 TSPLIB instances in its published benchmark, Memetic EAX's median gap is 0.00% on every single one, over five runs each. The largest map, dsj1000 with 1,000 cities, is the only one where any run missed at all: the worst was 0.04% above optimal, and the median run took about 30 seconds. The benchmark allows it 300 seconds.

I did not expect that when I added the solver. I expected an interesting algorithm that was too slow to matter. It is slow. It is also, in the median run, exactly optimal on every map in the benchmark.

Now the catches, because there are two.

The first is permanent: EAX is a heuristic. A gap of 0.00% means "matched a route someone else proved optimal", on maps whose optimum is already known. On a brand-new map it will give you a remarkably short tour, not a certificate that no shorter tour exists. That is the honest answer to the P vs NP question. Heuristics do not defeat NP-hardness; they route around it for the inputs people actually have.

The second is the price of all that machinery: speed. On berlin52, 2-opt finishes in about two hundredths of a second; Memetic EAX takes about three seconds. Every child pays for a crossover, a patch, and a local-search polish. On a tiny map that trade is silly. On a real budget it is the whole point: you are buying quality per run, not tours per second.

## Try it

The [interactive explainer](/algorithms/eax/explainer/) is the best place to see the move. The [algorithm page](/algorithms/eax/) has the full description, the options, and the benchmark table, with the references at the bottom.

If you would rather feel the gap close, open [the solver](/solve/?dataset=berlin52). It loads berlin52 and its optimal tour, so the gap column is live. Run Nearest Neighbor and note the gap, run 2-opt and watch it shrink, then choose Memetic EAX from the "All solvers…" menu and watch it land on 0.00%.

Or from the command line:

```bash
teeline solve eax -i berlin52.tsp
```

TSP is NP-hard in the worst case. Your map is probably not the worst case.

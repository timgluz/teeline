#!/usr/bin/env python3
"""Generate a deterministic synthetic TSPLIB EUC_2D instance.

The benchmark scaling run needs coordinate sets far larger than any tracked fixture. Points
come from a shuffled grid plus a small jitter, so they are distinct, spatially spread, and
reproducible from `--seed` alone. A plain uniform random draw could produce duplicate points,
which would make distances collapse and quietly change what the run measures.

    scripts/gen-synthetic-tsp.py --n 10000 --seed 42 > /tmp/10000.tsp
"""

from __future__ import annotations

import argparse
import math
import random
import sys


def parse_args() -> argparse.Namespace:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--n", type=int, required=True, help="number of cities (>= 2)")
    ap.add_argument("--seed", type=int, default=42, help="PRNG seed (default: 42)")
    ap.add_argument(
        "--spacing",
        type=float,
        default=10.0,
        help="grid spacing; jitter is added within one cell (default: 10.0)",
    )
    args = ap.parse_args()
    if args.n < 2:
        ap.error("--n must be at least 2")
    if args.spacing <= 0:
        ap.error("--spacing must be positive")
    return args


def main() -> int:
    args = parse_args()
    rng = random.Random(args.seed)

    side = math.ceil(math.sqrt(args.n))
    cells = list(range(side * side))
    rng.shuffle(cells)

    print(f"NAME: synthetic_{args.n}_{args.seed}")
    print("TYPE: TSP")
    print(f"DIMENSION: {args.n}")
    print("EDGE_WEIGHT_TYPE: EUC_2D")
    print("NODE_COORD_SECTION")
    for city_id, cell in enumerate(cells[: args.n], start=1):
        x = (cell % side) * args.spacing + rng.random() * args.spacing
        y = (cell // side) * args.spacing + rng.random() * args.spacing
        print(f"{city_id} {x:.6f} {y:.6f}")
    print("EOF")
    return 0


if __name__ == "__main__":
    sys.exit(main())

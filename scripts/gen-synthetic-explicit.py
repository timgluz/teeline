#!/usr/bin/env python3
"""Generate a deterministic synthetic TSPLIB EXPLICIT instance in UPPER_ROW layout.

The compact-storage measurements need explicit files far larger than any tracked fixture.
Weights are deterministic from the city pair and `--seed`, so a run is reproducible without
storing the multi-gigabyte input. Values stay in `0..max_weight`, which lets a caller choose a
`--max-weight` that exercises the `u16` storage path; wider values exercise the `f32` fallback.

    scripts/gen-synthetic-explicit.py --n 5000 --max-weight 60000 > /tmp/explicit5000.tsp
"""

from __future__ import annotations

import argparse
import sys


def parse_args() -> argparse.Namespace:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--n", type=int, required=True, help="number of cities (>= 2)")
    ap.add_argument(
        "--max-weight",
        type=int,
        default=60000,
        help="largest generated weight; <= 65535 exercises u16 (default: 60000)",
    )
    ap.add_argument("--seed", type=int, default=42, help="weight seed (default: 42)")
    args = ap.parse_args()
    if args.n < 2:
        ap.error("--n must be at least 2")
    if args.max_weight < 1:
        ap.error("--max-weight must be positive")
    return args


def main() -> int:
    args = parse_args()

    print(f"NAME: synthetic_explicit_{args.n}_{args.max_weight}_{args.seed}")
    print("TYPE: TSP")
    print(f"DIMENSION: {args.n}")
    print("EDGE_WEIGHT_TYPE: EXPLICIT")
    print("EDGE_WEIGHT_FORMAT: UPPER_ROW")
    print("EDGE_WEIGHT_SECTION")

    buffer: list[str] = []
    for i in range(args.n):
        for j in range(i + 1, args.n):
            weight = (i * 31 + j * 17 + args.seed) % args.max_weight + 1
            buffer.append(str(weight))
        if len(buffer) >= 4000:
            print(" ".join(buffer))
            buffer.clear()
    if buffer:
        print(" ".join(buffer))
    print("EOF")
    return 0


if __name__ == "__main__":
    sys.exit(main())

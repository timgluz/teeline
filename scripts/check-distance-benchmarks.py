#!/usr/bin/env python3
"""Check the matrix-vs-on-the-fly comparison against its documented thresholds.

Reads the JSONL produced by `scripts/bench-matrix.sh bench/matrix/tier-distance.toml`,
groups rows by (solver, dataset) and backend, and fails when the on-the-fly backend's median
wall-clock or peak RSS exceeds the configured tolerance.

The comparison is only meaningful for rows with `status: ok`; timeouts and errors are ignored
rather than compared, because they carry no comparable observation.

    scripts/check-distance-benchmarks.py
    scripts/check-distance-benchmarks.py --jsonl bench/runs/tier-distance-compare.jsonl
    scripts/check-distance-benchmarks.py --runtime-tolerance 2.0
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

# Median wall-clock tolerance per solver family. The on-the-fly backend computes each distance
# instead of looking it up, so the tolerance is looser than the matrix's noise floor; the exact
# and constructive values are dominated by process startup at corpus sizes.
FAMILY_TOLERANCES = {
    "exact": 2.0,
    "constructive": 2.0,
    "local": 1.5,
    "meta": 1.5,
    "utility": 1.5,
}
DEFAULT_MEMORY_RATIO = 1.25


def parse_args() -> argparse.Namespace:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--jsonl",
        type=Path,
        default=Path("bench/runs/tier-distance-compare.jsonl"),
        help="comparison run log (default: bench/runs/tier-distance-compare.jsonl)",
    )
    ap.add_argument(
        "--solvers-json",
        type=Path,
        default=Path("bench/solvers.json"),
        help="solver catalogue used for family lookup",
    )
    ap.add_argument(
        "--memory-ratio",
        type=float,
        default=DEFAULT_MEMORY_RATIO,
        help=f"maximum on-the-fly/matrix median peak-RSS ratio (default: {DEFAULT_MEMORY_RATIO})",
    )
    ap.add_argument(
        "--runtime-tolerance",
        type=float,
        help="override every family's median wall-clock tolerance",
    )
    return ap.parse_args()


def backend_of(config: str) -> str | None:
    if config.startswith("matrix"):
        return "matrix"
    if config.startswith("on-the-fly"):
        return "on-the-fly"
    return None


def main() -> int:
    args = parse_args()
    if not args.jsonl.is_file():
        print(f"ERROR: {args.jsonl} not found", file=sys.stderr)
        return 2

    solvers = json.loads(args.solvers_json.read_text(encoding="utf-8"))
    family_of = {solver["id"]: solver["family"] for solver in solvers["solvers"]}

    cells: dict[tuple[str, str], dict[str, list[dict]]] = defaultdict(
        lambda: defaultdict(list)
    )
    for line in args.jsonl.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        row = json.loads(line)
        if row.get("status") != "ok":
            continue
        backend = backend_of(str(row.get("config", "")))
        if backend is None:
            continue
        cells[(row["solver"], row["dataset"])][backend].append(row)

    violations: list[str] = []
    unpaired = 0

    print(
        f"{'solver':24}{'instance':12}{'family':14}"
        f"{'matrix wall':>12}{'otf wall':>10}{'wall x':>8}"
        f"{'matrix rss':>12}{'otf rss':>10}{'rss x':>8}"
    )
    for (solver, dataset), by_backend in sorted(cells.items()):
        if "matrix" not in by_backend or "on-the-fly" not in by_backend:
            unpaired += 1
            continue

        matrix = by_backend["matrix"]
        onfly = by_backend["on-the-fly"]
        matrix_wall = statistics.median(row["wall_s"] for row in matrix)
        onfly_wall = statistics.median(row["wall_s"] for row in onfly)
        matrix_rss = statistics.median(row["peak_rss_kb"] for row in matrix)
        onfly_rss = statistics.median(row["peak_rss_kb"] for row in onfly)

        family = family_of.get(solver, "unknown")
        tolerance = (
            args.runtime_tolerance
            if args.runtime_tolerance is not None
            else FAMILY_TOLERANCES.get(family, 1.5)
        )
        wall_ratio = onfly_wall / matrix_wall if matrix_wall else float("inf")
        rss_ratio = onfly_rss / matrix_rss if matrix_rss else float("inf")

        print(
            f"{solver:24}{dataset:12}{family:14}"
            f"{matrix_wall:12.4f}{onfly_wall:10.4f}{wall_ratio:8.2f}"
            f"{matrix_rss:12.0f}{onfly_rss:10.0f}{rss_ratio:8.2f}"
        )

        if wall_ratio > tolerance:
            violations.append(
                f"{solver}/{dataset}: wall {wall_ratio:.2f}x > {tolerance:.2f}x"
            )
        if rss_ratio > args.memory_ratio:
            violations.append(
                f"{solver}/{dataset}: rss {rss_ratio:.2f}x > {args.memory_ratio:.2f}x"
            )

    print()
    if unpaired:
        print(
            f"WARN: {unpaired} cell(s) had fewer than two backends and were skipped",
            file=sys.stderr,
        )
    if violations:
        print("VIOLATIONS:")
        for violation in violations:
            print(f" - {violation}")
        return 1
    print("OK: all paired cells are within the documented runtime and memory tolerances")
    return 0


if __name__ == "__main__":
    sys.exit(main())

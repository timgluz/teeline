#!/usr/bin/env python3
"""Plan the cells of a benchmark tier.

Reads a tier config (TierConfig), the canonical solver map, and whatever the output file
already contains, and prints one JSON object per cell still to run — so `bench-matrix.sh` is
only a runner and `--dry-run` is the same plan without the running.

A "cell" is one `(solver, instance, config, run)`. The sweep is resumable, so a cell already
present in the output is skipped unless `--force`; that is what makes an interrupted run safe
to restart.
"""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

# Where an instance's TSPLIB file may live. The tracked fixtures come first so a run does not
# depend on the git-ignored download directory.
INSTANCE_DIRS = ("tests/fixtures", "data/tsplib")


def fail(message: str) -> "NoReturn":  # noqa: F821 - simple exit helper
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_config(path: Path) -> dict:
    try:
        with path.open("rb") as fh:
            return tomllib.load(fh)
    except FileNotFoundError:
        fail(f"tier config not found: {path}")
    except tomllib.TOMLDecodeError as exc:
        fail(f"{path} is not valid TOML: {exc}")


def load_solvers(path: Path) -> tuple[dict[str, dict], dict[str, str]]:
    """Return (by id, cli name -> id)."""
    try:
        doc = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        fail(f"solver map not found: {path}")
    except json.JSONDecodeError as exc:
        fail(f"{path} is not valid JSON: {exc}")
    by_id = {s["id"]: s for s in doc["solvers"]}
    by_cli = {s["cli"]: s["id"] for s in doc["solvers"]}
    return by_id, by_cli


def resolve_instance(name: str) -> Path:
    """The TSPLIB file for an instance, or a hard failure.

    A missing or unparseable instance must stop the sweep rather than be skipped: a silently
    absent instance is a hole in the published data that nothing downstream can detect.
    """
    for directory in INSTANCE_DIRS:
        candidate = Path(directory) / f"{name}.tsp"
        if candidate.is_file():
            return candidate
    fail(f"no TSPLIB file for instance '{name}' (looked in {', '.join(INSTANCE_DIRS)})")


def completed_cells(path: Path) -> set[tuple[str, str, str, int]]:
    """Cells already recorded, so a resumed run does not repeat them.

    A half-written final line is ignored rather than fatal — the runner truncates it — but an
    unreadable line anywhere else means the file is not what it claims to be, and planning
    from it would duplicate or lose cells.
    """
    if not path.is_file():
        return set()
    done: set[tuple[str, str, str, int]] = set()
    lines = [ln for ln in path.read_text(encoding="utf-8").splitlines() if ln.strip()]
    for index, line in enumerate(lines):
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            if index == len(lines) - 1:
                print(f"WARN: ignoring an incomplete final row in {path}", file=sys.stderr)
                continue
            fail(f"{path}:{index + 1} is not valid JSON; refusing to plan from it")
        done.add(
            (
                str(row.get("solver", "")),
                str(row.get("dataset", "")),
                str(row.get("config") or "default"),
                int(row.get("run", 0)),
            )
        )
    return done


def config_applies(cfg: dict, solver_id: str, solver_family: str, instance: str) -> bool:
    """Whether a named config applies to this solver on this instance.

    A config may restrict itself with `solvers` and/or `instances`; omitting either means "all".
    """
    solvers = cfg.get("solvers")
    if solvers is not None and solver_id not in solvers:
        return False
    instances = cfg.get("instances")
    if instances is not None and instance not in instances:
        return False
    del solver_family  # families gate coverage, not configs
    return True


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--tier", required=True, type=Path)
    ap.add_argument("--solvers-json", required=True, type=Path)
    ap.add_argument("--existing", type=Path)
    ap.add_argument("--solver", action="append", default=[])
    ap.add_argument("--runs", type=int)
    ap.add_argument("--force", action="store_true")
    args = ap.parse_args()

    cfg = load_config(args.tier)
    by_id, by_cli = load_solvers(args.solvers_json)

    tier = cfg.get("tier")
    if not tier:
        fail(f"{args.tier} has no `tier` name")
    runs = args.runs if args.runs is not None else cfg.get("runs", 1)
    if runs < 1:
        fail(f"runs must be >= 1, got {runs}")
    budgets = cfg.get("budgets") or {}
    coverage = cfg.get("coverage") or {}
    instances = cfg.get("instances") or {}
    configs = cfg.get("configs") or {}
    default_timeout = cfg.get("default_timeout_s", 300)

    if not instances:
        fail(f"{args.tier} declares no [instances]")
    if "default" not in configs:
        # Every published cell carries a config; `default` is the one a solver does out of the
        # box and is what the contract's examples show.
        fail(f"{args.tier} must declare [configs.default]")

    # --- which solvers ---
    if args.solver:
        selected: list[str] = []
        for raw in args.solver:
            sid = raw if raw in by_id else by_cli.get(raw)
            if sid is None:
                fail(
                    f"unknown solver '{raw}'; not in {args.solvers_json} as either an id or a CLI name"
                )
            if sid not in selected:
                selected.append(sid)
    else:
        selected = sorted(by_id)

    # --- resolve the instance files up front, so a missing one fails before any running ---
    instance_paths: dict[str, Path] = {}
    for name in instances:
        instance_paths[name] = resolve_instance(name)

    done = set() if args.force else completed_cells(args.existing) if args.existing else set()

    cells: list[dict] = []
    for sid in selected:
        solver = by_id[sid]
        family = solver.get("family")
        if family not in coverage:
            fail(f"solver '{sid}' has family '{family}', which [coverage] does not mention")
        groups = set(coverage[family])
        timeout_s = budgets.get(family, default_timeout)

        for instance, group in sorted(instances.items()):
            if group not in groups:
                # Capability tiering: an exponential solver is not measured on instances it
                # cannot finish, rather than recording a timeout that was never in doubt.
                continue
            for config_name, config_cfg in sorted(configs.items()):
                if not config_applies(config_cfg or {}, sid, family, instance):
                    continue
                for run_no in range(1, runs + 1):
                    if (sid, instance, config_name, run_no) in done:
                        continue
                    cells.append(
                        {
                            "solver": sid,
                            "cli": solver["cli"],
                            "instance": instance,
                            "path": str(instance_paths[instance]),
                            "group": group,
                            "config": config_name,
                            "timeout_s": timeout_s,
                            "run": run_no,
                            "args": list((config_cfg or {}).get("args", [])),
                        }
                    )

    if args.existing and done:
        print(f"    resuming: {len(done)} cell(s) already in {args.existing}", file=sys.stderr)

    for cell in cells:
        print(json.dumps(cell, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

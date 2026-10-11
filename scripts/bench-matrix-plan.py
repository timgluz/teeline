#!/usr/bin/env python3
"""Plan the cells of a benchmark tier.

Reads a tier config, the canonical solver map, and whatever the output file already contains,
and prints one JSON object per cell still to run — so `bench-matrix.sh` is only a runner and
`--dry-run` is the same plan without the running.

A "cell" is one `(solver, instance, config, run)`. The sweep is resumable, so a cell already
present in the output is skipped unless `--force`.

Almost every check here exists for the same reason: a tier that quietly measures less than it
appears to is a hole in the published corpus that nothing downstream can detect. A family left
out of `[coverage]`, a group no family names, a config naming a solver that does not exist —
each would silently produce fewer rows and still look like success. They are all hard errors.
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


def fail(message: str) -> "NoReturn":  # noqa: F821 - exit helper
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

    A missing instance must stop the sweep rather than be skipped: an absent instance is a hole
    in the published data that nothing downstream can detect.
    """
    for directory in INSTANCE_DIRS:
        candidate = Path(directory) / f"{name}.tsp"
        if candidate.is_file():
            return candidate
    fail(f"no TSPLIB file for instance '{name}' (looked in {', '.join(INSTANCE_DIRS)})")


def read_existing(path: Path) -> tuple[set[tuple[str, str, str, int]], dict[str, int]]:
    """Cells already recorded, plus a count of each status.

    A half-written final line is ignored rather than fatal — the runner repairs it — but an
    unreadable line anywhere else means the file is not what it claims to be, and planning from
    it would duplicate or lose cells. A malformed row is reported with its file and line rather
    than raised as a traceback that says neither.
    """
    done: set[tuple[str, str, str, int]] = set()
    counts: dict[str, int] = {}
    if not path.is_file():
        return done, counts

    lines = [ln for ln in path.read_text(encoding="utf-8").splitlines() if ln.strip()]
    for index, line in enumerate(lines, start=1):
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            if index == len(lines):
                print(f"WARN: ignoring an incomplete final row in {path}", file=sys.stderr)
                continue
            fail(f"{path}:{index} is not valid JSON; refusing to plan from it")
        if not isinstance(row, dict):
            fail(f"{path}:{index} is not a JSON object; refusing to plan from it")
        run = row.get("run")
        if not isinstance(run, int) or isinstance(run, bool):
            fail(f"{path}:{index} has a non-integer 'run' ({run!r}); refusing to plan from it")
        solver, dataset = row.get("solver"), row.get("dataset")
        if not isinstance(solver, str) or not isinstance(dataset, str):
            fail(f"{path}:{index} has no usable 'solver'/'dataset'; refusing to plan from it")
        counts[str(row.get("status"))] = counts.get(str(row.get("status")), 0) + 1
        done.add((solver, dataset, str(row.get("config") or "default"), run))
    return done, counts


def validate_groups(coverage: dict, instances: dict) -> None:
    """Both directions of the family/group relation must line up.

    A group named by `[instances]` but by no family's `[coverage]` list is never measured — a
    typo like `upto42` would silently drop that instance from every board. A group a family names
    but `[instances]` never uses matches nothing, for the same reason.
    """
    named_by_a_family = {str(g) for groups in coverage.values() for g in groups}
    declared = {str(g) for g in instances.values()}
    unmeasured = sorted(declared - named_by_a_family)
    if unmeasured:
        fail(
            f"[coverage] names no family for instance group(s) {unmeasured}; those instances "
            f"would never be measured. Name a family, or drop them from [instances]."
        )
    unknown = sorted(named_by_a_family - declared)
    if unknown:
        fail(f"[coverage] refers to instance group(s) {unknown}, which [instances] never uses")


def validate_configs(configs: dict, by_id: dict, by_cli: dict, instances: dict) -> None:
    """A config naming an unknown solver or instance matches nothing and is silently ignored."""
    for name, cfg in configs.items():
        for raw in (cfg or {}).get("solvers", []) or []:
            if raw not in by_id and raw not in by_cli:
                fail(
                    f"[configs.{name}] names solver '{raw}', which is not in bench/solvers.json "
                    f"as either an id or a CLI name"
                )
        for raw in (cfg or {}).get("instances", []) or []:
            if raw not in instances:
                fail(f"[configs.{name}] names instance '{raw}', which [instances] does not declare")


def config_applies(cfg: dict, solver_id: str, instance: str) -> bool:
    """Whether a named config applies to this solver on this instance.

    A config may restrict itself with `solvers` and/or `instances`; omitting either means "all".
    """
    solvers = (cfg or {}).get("solvers")
    if solvers is not None and solver_id not in solvers:
        return False
    instances = (cfg or {}).get("instances")
    if instances is not None and instance not in instances:
        return False
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

    if not instances:
        fail(f"{args.tier} declares no [instances]")
    if "default" not in configs:
        # Every published cell carries a config; `default` is what a solver does out of the box
        # and is what the contract's examples show.
        fail(f"{args.tier} must declare [configs.default]")

    validate_groups(coverage, instances)
    validate_configs(configs, by_id, by_cli, instances)

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

    # A family the tier leaves out of [coverage] is refused: silence is indistinguishable from
    # having forgotten it, and the result would be a corpus missing solvers with nothing to show
    # for it. A family with no [budgets] entry is refused for the sibling reason — it would be
    # measured under some fallback while the published budgets state something else, so the page
    # would report an allowance the numbers were not taken under.
    for sid in selected:
        family = by_id[sid].get("family")
        if family not in coverage:
            fail(
                f"solver '{sid}' has family '{family}', which [coverage] does not mention. "
                f"Naming it with an empty group list is how a tier says 'not measured here'."
            )
        if family not in budgets:
            fail(f"family '{family}' has no [budgets] entry, so its timeout is undefined")

    # --- resolve the instance files up front, so a missing one fails before any running ---
    instance_paths = {name: resolve_instance(name) for name in instances}

    if args.force or not args.existing:
        done, counts = set(), {}
    else:
        done, counts = read_existing(args.existing)
    if counts:
        summary = ", ".join(f"{n} {status}" for status, n in sorted(counts.items()))
        print(f"    existing: {summary} in {args.existing}", file=sys.stderr)

    cells: list[dict] = []
    for sid in selected:
        solver = by_id[sid]
        family = solver.get("family")
        groups = set(coverage[family])
        timeout_s = budgets[family]

        for instance, group in sorted(instances.items()):
            if group not in groups:
                # Capability tiering: an exponential solver is not measured on instances it
                # cannot finish, rather than recording a timeout that was never in doubt.
                continue
            for config_name, config_cfg in sorted(configs.items()):
                if not config_applies(config_cfg, sid, instance):
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

    for cell in cells:
        print(json.dumps(cell, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

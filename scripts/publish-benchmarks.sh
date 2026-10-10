#!/usr/bin/env bash
#
# publish-benchmarks.sh — turn a benchmark TSV into the published JSON contract
# and upload it to the R2 bucket behind static.tspsolver.com.
#
# Usage:
#   scripts/publish-benchmarks.sh --tier a --dry-run
#   scripts/publish-benchmarks.sh --tier a --only r2
#   scripts/publish-benchmarks.sh --tier a --only criteriondb
#   scripts/publish-benchmarks.sh --tsv bench/runs/custom.tsv --prefix bench/v1 --dry-run
#
# Reads:  bench/runs/<tier>.tsv   (or --tsv), bench/solvers.json, docs/problems/*.md
# Writes: build/bench-json/v1/**  (local staging), then R2
#
# The JSON contract is documented in docs/benchmarks/schema.md — read that first.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# ---------------------------------------------------------------------------
# Defaults / arguments
# ---------------------------------------------------------------------------

TIER=""
TSV=""
DRY_RUN=0
ONLY="both"
REQUIRE_CLEAN=0
PREFIX="bench/v1"
STAGE="build/bench-json"
CRITERIONDB_URL="${CRITERIONDB_URL:-http://127.0.0.1:8080}"
CRITERIONDB_PROJECT="${CRITERIONDB_PROJECT:-default}"

usage() {
  sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  cat <<'EOF'

Options:
  --tier <a|b|c>       Read bench/runs/tier-<tier>.tsv (lowercase)
  --tsv <path>         Read an explicit TSV instead of a tier file
  --prefix <key>       R2 key prefix (default: bench/v1)
  --stage <dir>        Local staging directory (default: build/bench-json)
  --only <sink>        r2 | criteriondb | both (default: both)
  --bucket <name>      R2 bucket name (default: $CF_R2_BUCKET)
  --require-clean      Refuse to publish unless the working tree is clean (CI use)
  --dry-run            Build and validate everything, upload nothing
  -h, --help           Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tier) TIER="${2:?}"; shift 2 ;;
    --tsv) TSV="${2:?}"; shift 2 ;;
    --prefix) PREFIX="${2:?}"; shift 2 ;;
    --stage) STAGE="${2:?}"; shift 2 ;;
    --only) ONLY="${2:?}"; shift 2 ;;
    --bucket) CF_R2_BUCKET="${2:?}"; shift 2 ;;
    --require-clean) REQUIRE_CLEAN=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

case "$ONLY" in
  r2|criteriondb|both) ;;
  *) echo "--only must be r2, criteriondb or both (got '$ONLY')" >&2; exit 2 ;;
esac

if [[ -z "$TSV" ]]; then
  if [[ -z "$TIER" ]]; then
    echo "either --tier or --tsv is required" >&2
    usage >&2
    exit 2
  fi
  TIER_LOWER="$(echo "$TIER" | tr '[:upper:]' '[:lower:]')"
  TSV="bench/runs/tier-${TIER_LOWER}.tsv"
  TIER="$TIER_LOWER"
fi

if [[ ! -f "$TSV" ]]; then
  echo "ERROR: benchmark TSV not found: $TSV" >&2
  echo "Run the benchmark first: scripts/bench-matrix.sh bench/matrix/tier-a.toml" >&2
  exit 1
fi

CF_R2_BUCKET="${CF_R2_BUCKET:-}"
if [[ "$ONLY" != "criteriondb" && "$DRY_RUN" -eq 0 && -z "$CF_R2_BUCKET" ]]; then
  echo "ERROR: CF_R2_BUCKET is not set (or pass --bucket <name>)." >&2
  echo "Find it with: cf r2 buckets list" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Provenance — a published number must be attributable to a commit
# ---------------------------------------------------------------------------

DIRTY=0
if [[ -n "$(git status --porcelain)" ]]; then
  DIRTY=1
fi
# A dirty tree is reported, not fatal.
#
# The original hard failure was wrong: the script doing the publishing is itself
# usually the thing that is uncommitted, so "commit first, then publish" was
# unsatisfiable for any change to this pipeline. Worse, "is the tree dirty *now*"
# is the wrong question — what matters is what commit the measured binary came
# from, and that is recorded in each TSV row's own provenance, not by the tree
# state at publish time.
#
# So: warn loudly, record `dirty: true` in index.json so consumers can see it, and
# let CI opt into the stricter behaviour with --require-clean.
if [[ "$DIRTY" -eq 1 ]]; then
  if [[ "$REQUIRE_CLEAN" -eq 1 ]]; then
    echo "ERROR: working tree is dirty and --require-clean was given." >&2
    git status --porcelain | sed 's/^/    /' >&2
    exit 1
  fi
  echo "WARN: working tree is dirty — index.json will record dirty=true." >&2
  echo "      The measured binary's provenance is what matters; see bench/runs/*.tsv." >&2
fi

GIT_COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
TEELINE_VERSION="$(grep -m1 '^pub const VERSION' src/tsp/mod.rs | sed 's/.*"\(.*\)".*/\1/' || echo unknown)"
RUST_VERSION="$(rustc --version 2>/dev/null | awk '{print $2}' || echo unknown)"
GENERATED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
BUILD_PROFILE="release"

# ---------------------------------------------------------------------------
# The R2 client.
#
# `cf` (the agentic Cloudflare CLI) is always preferred, and is used whenever it
# is on PATH — per this repo's Cloudflare guidance and an explicit preference for
# `cf` over `wrangler`. `wrangler` is only a fallback for machines that have no
# `cf` binary at all (for example an older CI image), since `cf` is still an open
# beta. Both take the object body as a file.
# ---------------------------------------------------------------------------

R2_CLIENT=""
if command -v cf >/dev/null 2>&1; then
  R2_CLIENT="cf"
elif command -v npx >/dev/null 2>&1 && npx --no-install wrangler --version >/dev/null 2>&1; then
  R2_CLIENT="wrangler"
fi

r2_put() {
  local key="$1" file="$2"
  case "$R2_CLIENT" in
    cf)
      cf r2 objects put "${PREFIX}/${key}" \
        --bucket-name "$CF_R2_BUCKET" \
        --file "$file" \
        --content-type "application/json"
      ;;
    wrangler)
      npx --no-install wrangler r2 object put "${CF_R2_BUCKET}/${PREFIX}/${key}" \
        --file "$file" \
        --content-type "application/json"
      ;;
    *)
      echo "ERROR: neither 'cf' nor 'wrangler' is available to upload to R2." >&2
      echo "Install one: npm i -g cf   (or)   npm i -D wrangler" >&2
      return 1
      ;;
  esac
}

# ---------------------------------------------------------------------------
# Transform: TSV -> the published JSON contract
# ---------------------------------------------------------------------------

echo "==> Reading   $TSV"
echo "==> Staging   $STAGE/v1"
echo "==> Provenance commit=$GIT_COMMIT dirty=$DIRTY teeline=$TEELINE_VERSION rust=$RUST_VERSION"

mkdir -p "$STAGE/v1/algorithms" "$STAGE/v1/problems"

python3 - "$TSV" "$STAGE/v1" "$GIT_COMMIT" "$DIRTY" "$TEELINE_VERSION" \
          "$RUST_VERSION" "$GENERATED_AT" "$TIER" "$BUILD_PROFILE" <<'PY'
"""Transform a benchmark TSV into the docs/benchmarks/schema.md v1 contract.

Statistics use the median as the headline (robust to the heavy right tail that
stochastic TSP solvers produce) and carry best/worst alongside, because for a
non-deterministic solver a single number is a lie by omission.

Instances without a known optimal get `optimal_cost: null` and no gap at all —
never a fabricated 0, which would read as "reached the optimum".
"""
import json
import re
import statistics
import sys
from collections import defaultdict
from pathlib import Path

(
    tsv_path, out_dir, git_commit, dirty, teeline_version,
    rust_version, generated_at, tier, build_profile,
) = sys.argv[1:10]

ROOT = Path.cwd()
REQUIRED = ["solver", "dataset", "run", "wall_s", "peak_rss_kb", "tour_cost"]
OPTIONAL = ["config", "status"]
SCHEMA_VERSION = 1

# --- parse the TSV ---------------------------------------------------------

rows = []
with open(tsv_path, "r", encoding="utf-8") as fh:
    lines = [ln.rstrip("\n") for ln in fh if ln.strip()]
if not lines:
    sys.exit(f"ERROR: {tsv_path} is empty")

header = lines[0].split("\t")
missing = [c for c in REQUIRED if c not in header]
if missing:
    sys.exit(f"ERROR: {tsv_path} is missing required column(s): {', '.join(missing)}")
idx = {name: header.index(name) for name in REQUIRED + [c for c in OPTIONAL if c in header]}

def cell(parts, name, default=""):
    return parts[idx[name]].strip() if name in idx and idx[name] < len(parts) else default

for line in lines[1:]:
    parts = line.split("\t")
    try:
        rows.append({
            "solver": cell(parts, "solver"),
            "dataset": cell(parts, "dataset"),
            "config": cell(parts, "config", "default") or "default",
            "run": int(cell(parts, "run", "0") or 0),
            "wall_s": float(cell(parts, "wall_s", "0") or 0),
            "peak_rss_kb": float(cell(parts, "peak_rss_kb", "0") or 0),
            "tour_cost": float(cell(parts, "tour_cost", "0") or 0),
            "status": (cell(parts, "status", "ok") or "ok").lower(),
        })
    except ValueError as exc:
        sys.exit(f"ERROR: malformed row in {tsv_path}: {line!r} ({exc})")

if not rows:
    sys.exit(f"ERROR: {tsv_path} has a header but no data rows")

# --- instance metadata (optimal costs) from the generated problem docs -----

OPTIMAL_RE = re.compile(r"^optimalCost:\s*(.+)$", re.M)
CITIES_RE = re.compile(r"^cities:\s*(\d+)$", re.M)

instances = {}
for doc in sorted((ROOT / "docs" / "problems").glob("*.md")):
    text = doc.read_text(encoding="utf-8")
    m_opt = OPTIMAL_RE.search(text)
    m_cit = CITIES_RE.search(text)
    raw = m_opt.group(1).strip() if m_opt else "null"
    optimal = None if raw in ("null", "~", "") else float(raw)
    instances[doc.stem] = {
        "id": doc.stem,
        "cities": int(m_cit.group(1)) if m_cit else None,
        "optimal_cost": optimal,
    }

# --- solver metadata from the canonical map -------------------------------

index = json.loads((ROOT / "bench" / "solvers.json").read_text(encoding="utf-8"))
solvers = {s["id"]: s for s in index["solvers"]}
# The harness may be driven by CLI names; accept either spelling.
by_cli = {s["cli"]: s for s in index["solvers"]}

def solver_entry(name):
    return solvers.get(name) or by_cli.get(name)

unknown = sorted({r["solver"] for r in rows if solver_entry(r["solver"]) is None})
if unknown:
    sys.exit(
        "ERROR: TSV contains solver name(s) missing from bench/solvers.json: "
        + ", ".join(unknown)
        + "\nAdd them there (and to teeline-web/src/lib/solver-index.ts) first."
    )

def solver_id(name):
    return solver_entry(name)["id"]

# --- aggregate -------------------------------------------------------------

def stats(values):
    """median/best/worst always; mean only where it is meaningful to show."""
    if not values:
        return None
    out = {
        "median": round(statistics.median(values), 4),
        "best": round(min(values), 4),
        "worst": round(max(values), 4),
    }
    if len(values) > 1:
        out["mean"] = round(statistics.fmean(values), 4)
    return out

# (solver_id, dataset, config) -> {"ok": [rows], "timeouts": n, "errors": n}
cells = defaultdict(lambda: {"ok": [], "timeouts": 0, "errors": 0})
for r in rows:
    key = (solver_id(r["solver"]), r["dataset"], r["config"])
    bucket = cells[key]
    if r["status"] == "ok":
        bucket["ok"].append(r)
    elif r["status"] == "timeout":
        bucket["timeouts"] += 1
    else:
        bucket["errors"] += 1

# --- algorithms/{id}.json --------------------------------------------------

per_solver = defaultdict(lambda: {"configs": set(), "results": {}})
per_problem = defaultdict(list)

for (sid, dataset, config), bucket in sorted(cells.items()):
    ok = bucket["ok"]
    meta = instances.get(dataset, {"id": dataset, "cities": None, "optimal_cost": None})
    optimal = meta["optimal_cost"]

    gaps = []
    if optimal:
        gaps = [(r["tour_cost"] - optimal) / optimal * 100.0 for r in ok]

    entry = {
        "runs": len(ok),
        "timeouts": bucket["timeouts"],
        "errors": bucket["errors"],
    }
    gap_stats = stats(gaps)
    if gap_stats:
        entry["gap_pct"] = gap_stats
    entry["tour_cost"] = stats([r["tour_cost"] for r in ok])
    entry["wall_s"] = stats([r["wall_s"] for r in ok])
    rss = stats([r["peak_rss_kb"] for r in ok])
    if rss:
        entry["peak_rss_kb"] = rss
    if optimal is not None:
        entry["optimal_cost"] = optimal

    slot = per_solver[sid]
    slot["configs"].add(config)
    problem = slot["results"].setdefault(dataset, {
        "cities": meta["cities"],
        "optimal_cost": optimal,
        "by_config": {},
    })
    problem["by_config"][config] = entry

    per_problem[dataset].append({
        "solver": sid,
        "config": config,
        "runs": entry["runs"],
        "timeouts": entry["timeouts"],
        "errors": entry["errors"],
        **({"gap_pct": entry["gap_pct"]} if "gap_pct" in entry else {}),
        "tour_cost": entry["tour_cost"],
        "wall_s": entry["wall_s"],
        **({"peak_rss_kb": entry["peak_rss_kb"]} if "peak_rss_kb" in entry else {}),
    })

out = Path(out_dir)

def write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")

for sid, slot in sorted(per_solver.items()):
    write_json(out / "algorithms" / f"{sid}.json", {
        "schema_version": SCHEMA_VERSION,
        "solver": sid,
        "configs": sorted(slot["configs"]),
        "results": dict(sorted(slot["results"].items())),
    })

for dataset, board in sorted(per_problem.items()):
    # Rank by median gap where an optimal is known, otherwise by median tour cost.
    # Solvers that only timed out never reach the leaderboard: there is nothing to rank.
    board = [b for b in board if b["runs"] > 0]
    board.sort(key=lambda b: (
        b.get("gap_pct", {}).get("median", float("inf"))
        if "gap_pct" in b else float("inf"),
        b["tour_cost"]["median"],
    ))
    meta = instances.get(dataset, {"cities": None, "optimal_cost": None})
    write_json(out / "problems" / f"{dataset}.json", {
        "schema_version": SCHEMA_VERSION,
        "dataset": dataset,
        "cities": meta["cities"],
        "optimal_cost": meta["optimal_cost"],
        "leaderboard": board,
    })

write_json(out / "index.json", {
    "schema_version": SCHEMA_VERSION,
    "generated_at": generated_at,
    "git_commit": git_commit,
    "dirty": dirty == "1",
    "teeline_version": teeline_version,
    "tier": tier or None,
    "environment": {
        "os": "linux",
        "build": build_profile,
        "rust_version": rust_version,
    },
    "solvers": [
        {
            "id": sid,
            "name": solver_entry(sid)["name"],
            "configs": sorted(slot["configs"]),
        }
        for sid, slot in sorted(per_solver.items())
    ],
    "problems": [
        {
            "id": dataset,
            "cities": instances.get(dataset, {}).get("cities"),
            "optimal_cost": instances.get(dataset, {}).get("optimal_cost"),
        }
        for dataset in sorted(per_problem)
    ],
})

print(f"    algorithms: {len(per_solver)} shard(s)")
print(f"    problems:   {len(per_problem)} shard(s)")
print(f"    cells:      {len(cells)}  runs: {len(rows)}")
PY

echo "==> Staged JSON:"
find "$STAGE/v1" -name '*.json' | sort | sed 's/^/    /' | head -40
if [[ "$(find "$STAGE/v1" -name '*.json' | wc -l)" -gt 40 ]]; then
  echo "    ... ($(find "$STAGE/v1" -name '*.json' | wc -l) files total)"
fi

# The solver-id <-> CLI-name mapping, emitted once here so the criteriondb mirror
# reads it instead of re-deriving it. Two independent copies of this lookup would
# drift silently, since nothing would enforce that they agree.
python3 - "$STAGE" <<'PY'
import json, sys
from pathlib import Path
stage = Path(sys.argv[1])
index = json.loads((Path.cwd() / "bench" / "solvers.json").read_text(encoding="utf-8"))
by_name = {}
for s in index["solvers"]:
    by_name[s["id"]] = s["id"]
    by_name[s["cli"]] = s["id"]
# Lives outside v1/ so it is never uploaded to R2.
(stage / "_solvers.json").write_text(json.dumps(by_name, indent=2) + "\n", encoding="utf-8")
print(f"    solver map: {len(index['solvers'])} solver(s)")
PY

# ---------------------------------------------------------------------------
# Upload
# ---------------------------------------------------------------------------

upload_r2() {
  if [[ -z "$R2_CLIENT" ]]; then
    echo "ERROR: no R2 client available — install the 'cf' CLI:" >&2
    echo "    npm i -g cf" >&2
    echo "('wrangler' also works as a fallback if 'cf' cannot be installed.)" >&2
    return 1
  fi
  echo "==> Uploading to R2 bucket '$CF_R2_BUCKET' prefix '$PREFIX' via $R2_CLIENT"
  local key file count=0
  # Shards first, index.json LAST.
  #
  # index.json is the manifest that advertises every shard, so publishing it
  # first opens a window in which a concurrent reader (read-benchmarks.sh list or
  # check, or a page fetching on demand) sees a manifest listing shards that do
  # not exist yet and 404s on them. Uploading the manifest last means it only
  # ever points at objects that are already live. The reverse mistake — a
  # manifest that briefly omits a newly published shard — is harmless, because
  # that just means the old shard is still being served.
  local shards
  shards="$(find "$STAGE/v1" -name '*.json' ! -name index.json | sort)"
  for file in $shards "$STAGE/v1/index.json"; do
    key="${file#"$STAGE/v1/"}"
    if [[ "$DRY_RUN" -eq 1 ]]; then
      echo "    [dry-run] would put ${PREFIX}/${key}"
    else
      r2_put "$key" "$file" >/dev/null
      echo "    put ${PREFIX}/${key}"
    fi
    count=$((count + 1))
  done
  echo "==> $count object(s) $([[ "$DRY_RUN" -eq 1 ]] && echo 'would be ' || echo '')uploaded"
}

mirror_criteriondb() {
  local key="${CRITERIONDB_API_KEY:-}"
  if [[ -z "$key" ]]; then
    echo "WARN: CRITERIONDB_API_KEY is unset — skipping criteriondb mirror." >&2
    return 0
  fi

  # Refuse to send a bearer token in cleartext. http:// is only acceptable for a
  # loopback endpoint (local development); anything else must be https.
  case "$CRITERIONDB_URL" in
    https://*)
      ;;
    http://localhost*|http://127.0.0.1*|http://\[::1\]*)
      echo "WARN: sending CRITERIONDB_API_KEY over plaintext http to a loopback host." >&2
      ;;
    http://*)
      echo "ERROR: refusing to send CRITERIONDB_API_KEY in cleartext to '$CRITERIONDB_URL'." >&2
      echo "Use an https:// endpoint, or a loopback http:// URL for local development." >&2
      return 1
      ;;
    *)
      echo "ERROR: CRITERIONDB_URL must start with https:// or http:// (got '$CRITERIONDB_URL')." >&2
      return 1
      ;;
  esac

  echo "==> Mirroring runs into criteriondb"
  echo "    url:     $CRITERIONDB_URL"
  echo "    project: $CRITERIONDB_PROJECT"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "    [dry-run] would POST one tspsolver-profile run per (solver, dataset, config) cell"
    return 0
  fi
  python3 - "$TSV" "$CRITERIONDB_URL" "$CRITERIONDB_PROJECT" "$key" "$STAGE/_solvers.json" <<'PY'
import json
import os
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

tsv_path, base_url, project, api_key, solver_map_path = sys.argv[1:6]
ROOT = Path.cwd()

# The solver-id <-> CLI-name map written by the transform above (single source,
# so it cannot drift from the mapping that produced the R2 shards).
try:
    by_name = json.loads(Path(solver_map_path).read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as exc:
    sys.exit(f"ERROR: cannot read solver map {solver_map_path}: {exc}")

# Optional environment override for the archived run. Validated up front — an
# unguarded json.loads inside the upload loop would abort midway with some runs
# already POSTed and an opaque traceback.
env_override = os.environ.get("CRITERIONDB_ENVIRONMENT")
if env_override:
    try:
        environment = json.loads(env_override)
    except json.JSONDecodeError as exc:
        sys.exit(f"ERROR: CRITERIONDB_ENVIRONMENT is not valid JSON: {exc}")
    if not isinstance(environment, dict):
        sys.exit("ERROR: CRITERIONDB_ENVIRONMENT must be a JSON object.")
else:
    environment = {
        "os": "linux", "cpu_name": "unknown", "n_threads": 1, "ram_gb": 0.0, "extra": {},
    }

optimal = {}
for doc in (ROOT / "docs" / "problems").glob("*.md"):
    m = re.search(r"^\s*optimalCost:\s*(.+)$", doc.read_text(encoding="utf-8"), re.M)
    if m and m.group(1).strip() not in ("null", "~", ""):
        try:
            optimal[doc.stem] = float(m.group(1).strip())
        except ValueError:
            continue

lines = [ln.rstrip("\n") for ln in open(tsv_path, encoding="utf-8") if ln.strip()]
header = lines[0].split("\t")
idx = {name: header.index(name) for name in header if name}

cells = defaultdict(list)
for line in lines[1:]:
    p = line.split("\t")
    def get(name, default=""):
        return p[idx[name]].strip() if name in idx and idx[name] < len(p) else default
    status = (get("status", "ok") or "ok").lower()
    if status != "ok":
        continue  # a timeout has no tour cost to archive
    sid = by_name.get(get("solver"))
    if sid is None:
        continue
    cells[(sid, get("dataset"), get("config", "default") or "default")].append({
        "run": int(get("run", "0") or 0),
        "wall_s": float(get("wall_s", "0") or 0),
        "peak_rss_kb": float(get("peak_rss_kb", "0") or 0),
        "tour_cost": float(get("tour_cost", "0") or 0),
    })

url = f"{base_url.rstrip('/')}/api/v1/projects/{os.environ.get('CRITERIONDB_OWNER', 'tspsolver')}/{project}/runs"
sent = failed = 0
for (sid, dataset, config), runs in sorted(cells.items()):
    opt = optimal.get(dataset)
    solver_results = []
    for r in sorted(runs, key=lambda x: x["run"]):
        metrics = [
            {"name": "tour_cost", "unit": "", "value": r["tour_cost"]},
            {"name": "wall_s", "unit": "s", "value": r["wall_s"]},
            {"name": "peak_rss_kb", "unit": "kb", "value": r["peak_rss_kb"]},
        ]
        if opt:
            metrics.append({"name": "optimal_cost", "unit": "", "value": opt})
            metrics.append({
                "name": "gap_pct", "unit": "%",
                "value": (r["tour_cost"] - opt) / opt * 100.0,
            })
        solver_results.append({
            "solver": sid,
            "dataset": dataset,
            "run_number": r["run"],
            "metrics": metrics,
        })

    payload = {
        "environment": environment,
        "git_commit": os.environ.get("GIT_COMMIT"),
        "git_branch": os.environ.get("GIT_BRANCH"),
        "tags": {"config_label": config, "tier": os.environ.get("BENCH_TIER", "")},
        "profile": "tspsolver",
        "benchmarks": [],
        "solver_results": solver_results,
    }

    proc = subprocess.run(
        ["curl", "-sS", "-o", "/dev/null", "-w", "%{http_code}",
         "-X", "POST", url,
         "-H", f"Authorization: Bearer {api_key}",
         "-H", "Content-Type: application/json",
         "--data-binary", "@-"],
        input=json.dumps(payload), text=True, capture_output=True,
    )
    code = proc.stdout.strip()
    if code.startswith("2"):
        sent += 1
    else:
        failed += 1
        print(f"    FAILED {sid}/{dataset}/{config}: HTTP {code} {proc.stderr.strip()[:200]}", file=sys.stderr)

print(f"    uploaded {sent} run(s), {failed} failure(s)")
if failed:
    sys.exit(1)
PY
}

if [[ "$ONLY" == "r2" || "$ONLY" == "both" ]]; then
  upload_r2
fi

if [[ "$ONLY" == "criteriondb" || "$ONLY" == "both" ]]; then
  # git metadata for the archived run
  GIT_COMMIT_FULL="$(git rev-parse HEAD 2>/dev/null || echo)"
  GIT_BRANCH_NAME="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo)"
  export GIT_COMMIT="$GIT_COMMIT_FULL"
  export GIT_BRANCH="$GIT_BRANCH_NAME"
  export BENCH_TIER="${TIER:-}"
  mirror_criteriondb
fi

echo "==> Done."
if [[ "$DRY_RUN" -eq 0 && "$ONLY" != "criteriondb" ]]; then
  echo "    Verify: scripts/read-benchmarks.sh index"
fi

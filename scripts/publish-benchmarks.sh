#!/usr/bin/env bash
#
# publish-benchmarks.sh — turn a benchmark TSV into the published JSON contract
# and upload it to the R2 bucket behind static.tspsolver.com.
#
# Usage:
#   scripts/publish-benchmarks.sh --tier a --dry-run
#   scripts/publish-benchmarks.sh --tier a --only r2
#   scripts/publish-benchmarks.sh --tier a --only criteriondb
#   scripts/publish-benchmarks.sh --tsv bench/runs/custom.tsv --prefix bench/v2 --dry-run
#
# Reads:  bench/runs/<tier>.tsv   (or --tsv), bench/solvers.json, docs/problems/*.md
# Writes: build/bench-json/v2/**  (local staging), then R2
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
PREFIX="bench/v2"
STAGE="build/bench-json"
# Previously published tree, merged in so a differential re-run keeps the solvers it did
# not re-measure (their shards and provenance) and the problem leaderboards complete.
PREVIOUS="build/bench-prev"
SNAPSHOT_TAG=""
BUDGETS="bench/budgets.json"
PUBLIC_BASE="${PUBLIC_BASE:-https://static.tspsolver.com}"
FETCH_PREVIOUS=1
FORCE_SNAPSHOT=0
FIRST_PUBLISH=0
PREVIOUS_EXPLICIT=0
CRITERIONDB_URL="${CRITERIONDB_URL:-http://127.0.0.1:8080}"
CRITERIONDB_PROJECT="${CRITERIONDB_PROJECT:-default}"

usage() {
  sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  cat <<'EOF'

Options:
  --tier <a|b|c>       Read bench/runs/tier-<tier>.tsv (lowercase)
  --tsv <path>         Read an explicit TSV instead of a tier file
  --prefix <key>       R2 key prefix (default: bench/v2)
  --stage <dir>        Local staging directory (default: build/bench-json)
  --previous <dir>     Previously published tree to merge with (default: build/bench-prev)
  --public-base <url>  Base URL to fetch the previous tree from (default: $PUBLIC_BASE)
  --no-fetch-previous  Never fetch the previous tree; fail if it is missing and needed
  --snapshot <tag>     Also write an immutable copy under snapshots/<tag>/ (release tag)
  --force-snapshot     Overwrite an existing snapshot tag (normally refused)
  --first-publish      Confirm there is no previous tree and this run covers everything
  --budgets <path>     Per-family budgets file (default: bench/budgets.json)
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
    --previous) PREVIOUS="${2:?}"; PREVIOUS_EXPLICIT=1; shift 2 ;;
    --public-base) PUBLIC_BASE="${2:?}"; shift 2 ;;
    --no-fetch-previous) FETCH_PREVIOUS=0; shift ;;
    --snapshot) SNAPSHOT_TAG="${2:?}"; shift 2 ;;
    --force-snapshot) FORCE_SNAPSHOT=1; shift ;;
    --first-publish) FIRST_PUBLISH=1; shift ;;
    --budgets) BUDGETS="${2:?}"; shift 2 ;;
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
# Merge with the previously published tree
#
# A differential re-run measures only the algorithms that changed, so the tree it publishes
# must be completed from what is already live: otherwise the other solvers lose their shards
# and the problem leaderboards lose the entries that rank them. There is no bucket listing,
# so the previous index is read first and it names the shards to fetch.
# ---------------------------------------------------------------------------

# HTTP status seen when reading the previous index: only 404 means "first publish".
PREVIOUS_STATUS=""

fetch_previous() {
  local base="$PUBLIC_BASE/${PREFIX}" dest="$PREVIOUS/v2"
  # Start clean. A shard left over from an earlier run must not be merged as if it had been
  # published, and the index fetched below is what defines which shards exist at all.
  rm -rf "$dest"
  mkdir -p "$dest/algorithms" "$dest/problems"

  # Capture the status rather than relying on curl's exit code: a 404 and a DNS failure both
  # fail, but only the first means there is nothing to merge with.
  PREVIOUS_STATUS="$(curl -sS -o "$dest/index.json" -w '%{http_code}' "$base/index.json" || true)"
  # curl prints 000 itself on a transport failure; normalise an empty result the same way.
  PREVIOUS_STATUS="${PREVIOUS_STATUS:-000}"
  case "$PREVIOUS_STATUS" in
    200) ;;
    404) rm -f "$dest/index.json"; return 0 ;;
    000) echo "    ERROR: cannot reach $base/index.json (network or DNS failure)" >&2; return 1 ;;
    *)   echo "    ERROR: $base/index.json returned HTTP $PREVIOUS_STATUS" >&2; return 1 ;;
  esac

  # stderr is not suppressed: if this fails we want to see why, not treat it as an empty tree.
  if ! jq -e '.solvers and .problems' "$dest/index.json" >/dev/null; then
    echo "    ERROR: the previous index.json is not a valid manifest" >&2
    return 1
  fi

  # A shard the manifest names but we cannot fetch would silently vanish from the republished
  # tree — the solver would drop out of the manifest and the leaderboards — so this is fatal.
  local failures=0 id
  while read -r id; do
    [[ -n "$id" ]] || continue
    if ! curl -fsS --retry 3 "$base/algorithms/$id.json" -o "$dest/algorithms/$id.json"; then
      echo "    ERROR: could not fetch algorithms/$id.json (named by the previous index)" >&2
      failures=$((failures + 1))
    fi
  done < <(jq -r '.solvers[]?.id' "$dest/index.json")

  while read -r id; do
    [[ -n "$id" ]] || continue
    if ! curl -fsS --retry 3 "$base/problems/$id.json" -o "$dest/problems/$id.json"; then
      echo "    ERROR: could not fetch problems/$id.json (named by the previous index)" >&2
      failures=$((failures + 1))
    fi
  done < <(jq -r '.problems[]?.id' "$dest/index.json")

  [[ "$failures" -eq 0 ]] || return 1
  return 0
}

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq is required (the merge reads the previous manifest with it)." >&2
  exit 1
fi

MERGE_PREVIOUS=0
if [[ "$PREVIOUS_EXPLICIT" -eq 1 ]]; then
  # An explicitly supplied tree is taken as-is; its freshness is the operator's call.
  if [[ ! -s "$PREVIOUS/v2/index.json" ]]; then
    echo "ERROR: --previous '$PREVIOUS' contains no v2/index.json" >&2
    exit 1
  fi
  MERGE_PREVIOUS=1
  echo "==> Previous  $PREVIOUS/v2 (explicit)"
elif [[ "$FETCH_PREVIOUS" -eq 1 && "$ONLY" != "criteriondb" ]]; then
  echo "==> Previous  fetching $PUBLIC_BASE/${PREFIX}"
  if ! fetch_previous; then
    echo "ERROR: could not obtain the previously published tree." >&2
    echo "       Refusing to continue: publishing now would replace the live manifest and" >&2
    echo "       leaderboards with a tree containing only this TSV's solvers." >&2
    exit 1
  fi
  if [[ "$PREVIOUS_STATUS" == "200" ]]; then
    MERGE_PREVIOUS=1
    echo "    fetched $(find "$PREVIOUS/v2" -name '*.json' | wc -l) previous shard(s)"
  else
    echo "    no previously published tree under ${PREFIX} (HTTP 404)"
  fi
elif [[ "$ONLY" != "criteriondb" && -s "$PREVIOUS/v2/index.json" ]]; then
  MERGE_PREVIOUS=1
  echo "==> Previous  $PREVIOUS/v2 (not fetched)"
fi

# Publishing a tree that only contains this TSV's solvers is legitimate for a first publish
# and destructive otherwise, so it needs saying out loud rather than being the silent default.
if [[ "$ONLY" != "criteriondb" && "$MERGE_PREVIOUS" -eq 0 && "$FIRST_PUBLISH" -eq 0 ]]; then
  echo "ERROR: there is no previous tree to merge with." >&2
  echo "       Without one, only this TSV's solvers and problems would be published, and any" >&2
  echo "       solver already live would drop out of the manifest and the leaderboards." >&2
  echo "       Pass --first-publish if this run really does cover everything, or supply" >&2
  echo "       --previous <dir>." >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Transform: TSV -> the published JSON contract
# ---------------------------------------------------------------------------

echo "==> Reading   $TSV"
echo "==> Staging   $STAGE/v2"
# The definitive provenance is resolved inside the transform below, which prefers
# the TSV's own header. This line reports the publish-time values as a fallback so
# it is clear which set the manifest ended up using.
echo "==> Publish-time git state: commit=$GIT_COMMIT dirty=$DIRTY (used only if the TSV carries no provenance)"

# Clear the staged shards first. Leftovers from an earlier run or a different TSV would
# otherwise be uploaded as orphans, and a stale snapshots/ tree would be re-uploaded on every
# later run — overwriting an immutable snapshot and bypassing the guard in snapshot_stage.
rm -rf "$STAGE/v2/algorithms" "$STAGE/v2/problems" "$STAGE/v2/snapshots"
mkdir -p "$STAGE/v2/algorithms" "$STAGE/v2/problems"

python3 - "$TSV" "$STAGE/v2" "$GIT_COMMIT" "$DIRTY" "$TEELINE_VERSION" \
          "$RUST_VERSION" "$GENERATED_AT" "$TIER" "$BUILD_PROFILE" \
          "$PREVIOUS/v2" "$BUDGETS" "$SNAPSHOT_TAG" "$PREFIX" <<'PY'
"""Transform a benchmark TSV into the docs/benchmarks/schema.md v2 contract.

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
    previous_dir, budgets_path, snapshot_tag, prefix,
) = sys.argv[1:14]

ROOT = Path.cwd()
REQUIRED = ["solver", "dataset", "run", "wall_s", "peak_rss_kb", "tour_cost"]
OPTIONAL = ["config", "status"]
SCHEMA_VERSION = 2

# --- provenance: prefer the TSV's own header over publish-time HEAD ---------
#
# The commit that produced the measurements is what makes a number reproducible —
# not whatever HEAD happens to be when someone runs the publish script. A TSV
# written by bench-matrix.sh carries `#`-prefixed provenance lines; when they are
# absent (a hand-written file, or the pre-existing baseline TSVs) we fall back to
# publish-time HEAD and say so explicitly in `git_commit_source`, rather than
# implying the data came from that commit.
provenance: dict[str, str] = {}
data_lines: list[str] = []
with open(tsv_path, "r", encoding="utf-8") as fh:
    for raw in fh:
        line = raw.rstrip("\n")
        if line.startswith("#"):
            body = line.lstrip("#").strip()
            if "=" in body:
                k, v = body.split("=", 1)
                provenance[k.strip()] = v.strip()
            continue
        if line.strip():
            data_lines.append(line)

if "git_commit" in provenance:
    git_commit = provenance["git_commit"]
    git_commit_source = "tsv"
    # Anything the TSV did not record becomes unknown rather than inheriting the publisher's
    # values. Pairing a measured commit with the publisher's dirty flag, version or rustc would
    # be a provenance claim the file does not support — and once written into a shard under the
    # differential cadence it is permanent.
    dirty = provenance.get("dirty")
    teeline_version = provenance.get("teeline_version")
    rust_version = provenance.get("rust_version")
else:
    git_commit_source = "publish-time"

# The provenance of *these* measurements. It describes only the solvers present in this TSV:
# under the per-release differential cadence the other solvers were measured at a different
# commit, so it can never be lifted to a dataset-level claim. Keeping provenance here, beside
# the numbers it describes, is what makes that mistake unrepresentable.
measurement = {
    "git_commit": git_commit,
    "git_commit_source": git_commit_source,
    "dirty": None if dirty is None else dirty == "1",
    "teeline_version": teeline_version,
    # When the measurement was taken, which is not when it was published: a TSV carried forward
    # or republished would otherwise be stamped with today's date. The harness records this in
    # the TSV header; a file without it yields null rather than a publish-time stand-in that
    # would misdate the result.
    "measured_at": provenance.get("measured_at") or None,
    "environment": {
        "os": "linux",
        "build": build_profile,
        "rust_version": rust_version,
    },
}

# --- per-family budgets ----------------------------------------------------
#
# Published with the data so a gap is never shown without the allowance it was measured
# under. Each entry records whether the figure is fixed by a decision or is still a
# provisional placeholder, so a consumer can avoid presenting the latter as agreed.
budgets: dict = {}
budgets_comment = ""
try:
    with open(budgets_path, "r", encoding="utf-8") as fh:
        budgets_doc = json.load(fh)
    budgets = budgets_doc.get("budgets", {})
    budgets_comment = budgets_doc.get("$comment", "")
except FileNotFoundError:
    print(f"    WARN: budgets file not found ({budgets_path}); budgets omitted", file=sys.stderr)

# --- parse the TSV ---------------------------------------------------------

rows = []
lines = data_lines
if not lines:
    sys.exit(f"ERROR: {tsv_path} is empty")

header = lines[0].split("\t")
missing = [c for c in REQUIRED if c not in header]
if missing:
    sys.exit(f"ERROR: {tsv_path} is missing required column(s): {', '.join(missing)}")
idx = {name: header.index(name) for name in REQUIRED + [c for c in OPTIONAL if c in header]}

def cell(parts, name, default="", required=False, numeric=False):
    """Read one TSV cell.

    `required=True` is a hard error when the column is absent from this row —
    NOT a default. A row cut short by a crashed benchmark process must fail
    loudly: defaulting `tour_cost`/`wall_s` to 0 would fabricate a measurement
    that looks like a real (and implausible) result, which is precisely what this
    module must never do.

    `numeric=True` additionally rejects a present-but-empty cell. A blank
    `tour_cost` is missing data, and coercing it to 0.0 would be worse than
    useless: it yields a gap of -100%, i.e. "twice as good as optimal". A run
    whose numbers were not captured must be recorded with a non-`ok` status
    instead, so it counts against `errors` rather than entering the statistics.
    Non-numeric columns keep the default, since a blank cell there is a genuine
    "not recorded" marker rather than a corruption.
    """
    if name in idx and idx[name] < len(parts):
        value = parts[idx[name]].strip()
        if value == "" and numeric:
            raise ValueError(f"required numeric column '{name}' is empty")
        return value
    if required:
        raise ValueError(f"row is missing a value for required column '{name}'")
    return default

for lineno, line in enumerate(lines[1:], start=2):
    parts = line.split("\t")
    # Catch a row truncated by an interrupted benchmark before per-cell reading,
    # so the error names the line and the exact shortfall.
    if len(parts) < len(header):
        sys.exit(
            f"ERROR: truncated row at {tsv_path}:{lineno}: expected {len(header)} "
            f"tab-separated columns, found {len(parts)}: {line!r}"
        )
    try:
        rows.append({
            "solver": cell(parts, "solver", required=True),
            "dataset": cell(parts, "dataset", required=True),
            "config": cell(parts, "config", "default") or "default",
            "run": int(cell(parts, "run", "0", required=True, numeric=True)),
            "wall_s": float(cell(parts, "wall_s", "0", required=True, numeric=True)),
            "peak_rss_kb": float(
                cell(parts, "peak_rss_kb", "0", required=True, numeric=True)
            ),
            "tour_cost": float(
                cell(parts, "tour_cost", "0", required=True, numeric=True)
            ),
            "status": (cell(parts, "status", "ok") or "ok").lower(),
        })
    except ValueError as exc:
        sys.exit(f"ERROR: malformed row at {tsv_path}:{lineno}: {line!r} ({exc})")

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
for sub in ("algorithms", "problems"):
    (out / sub).mkdir(parents=True, exist_ok=True)

def write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")

def load_previous(kind: str) -> dict:
    """Load the previously published shards, keyed by their `solver`/`dataset` field.

    Absent on a first publish or a full re-run, in which case there is simply nothing to
    merge and the run must cover every solver (the caller enforces that).
    """
    prev: dict = {}
    root = Path(previous_dir)
    d = root / kind
    if not d.is_dir():
        return prev

    # Only the shards the previous manifest names count as published state. A stray file in
    # the directory was never live, and merging it would resurrect something no reader could
    # have seen — including a shard deliberately withdrawn.
    manifest = root / "index.json"
    if not manifest.is_file():
        sys.exit(
            f"ERROR: previous tree has no index.json ({manifest}); cannot tell which shards "
            f"were published"
        )
    try:
        doc = json.loads(manifest.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        sys.exit(f"ERROR: previous index.json is unreadable: {exc}")

    field = "solvers" if kind == "algorithms" else "problems"
    listed = {e["id"] for e in doc.get(field, []) if "id" in e}

    for f in sorted(d.glob("*.json")):
        try:
            doc = json.loads(f.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            # Not a warning: this shard is live, and dropping it would remove the solver or the
            # leaderboard from the republished tree without anyone noticing.
            sys.exit(f"ERROR: previous shard {f} is unreadable: {exc}")
        key = doc.get("solver") if kind == "algorithms" else doc.get("dataset")
        if not key:
            sys.exit(f"ERROR: previous shard {f} has no {'solver' if kind == 'algorithms' else 'dataset'} key")
        # A file the manifest does not name was never live, so it is not published state.
        if key not in listed:
            continue
        prev[key] = doc

    missing = listed - set(prev)
    if missing:
        sys.exit(
            f"ERROR: the previous manifest names {kind} that are not present in the tree: "
            f"{sorted(missing)}. Republishing would drop them."
        )
    return prev

measured = set(per_solver)
prev_algos = load_previous("algorithms")
prev_problems = load_previous("problems")

# --- algorithms/{id}.json --------------------------------------------------
#
# A solver measured here is replaced wholesale, including its `measurement` block. Every
# other solver keeps the shard it already had — carried over byte for byte, so its
# provenance still describes when *it* was measured rather than inheriting this run's.
final_algos: dict = {}
for sid, doc in prev_algos.items():
    if sid not in measured:
        final_algos[sid] = doc
for sid, slot in sorted(per_solver.items()):
    final_algos[sid] = {
        "schema_version": SCHEMA_VERSION,
        "solver": sid,
        "measurement": measurement,
        "configs": sorted(slot["configs"]),
        "results": dict(sorted(slot["results"].items())),
    }

for sid, doc in sorted(final_algos.items()):
    write_json(out / "algorithms" / f"{sid}.json", doc)

# --- problems/{dataset}.json ----------------------------------------------
#
# A leaderboard lists every solver for that instance, so it cannot be replaced from a TSV
# that measured only some of them: the entries for solvers not in this run are kept and the
# fresh ones take their place, then the board is re-ranked.
def rank_key(b: dict):
    return (
        b.get("gap_pct", {}).get("median", float("inf")) if "gap_pct" in b else float("inf"),
        b["tour_cost"]["median"],
    )

final_problems: dict = {}
for dataset in sorted(set(prev_problems) | set(per_problem)):
    if dataset in per_problem:
        fresh = [b for b in per_problem[dataset] if b["runs"] > 0]
        kept = [
            b
            for b in prev_problems.get(dataset, {}).get("leaderboard", [])
            if b.get("solver") not in measured
        ]
        board = kept + fresh
        # Fall back to the previous instance metadata: a dataset with no problem doc still has
        # a known city count and possibly an optimal, and nulling them loses real information.
        if dataset in instances:
            cities, optimal = instances[dataset]["cities"], instances[dataset]["optimal_cost"]
        else:
            prev_doc = prev_problems.get(dataset, {})
            cities, optimal = prev_doc.get("cities"), prev_doc.get("optimal_cost")
    else:
        doc = prev_problems[dataset]
        # Drop entries for solvers re-measured in this run: their shards were replaced and no
        # longer contain this dataset, so keeping them would rank results nothing backs.
        board = [
            b for b in doc.get("leaderboard", []) if b.get("solver") not in measured
        ]
        cities, optimal = doc.get("cities"), doc.get("optimal_cost")

    # Solvers that only timed out never reach the leaderboard: there is nothing to rank.
    board = [b for b in board if b["runs"] > 0]
    board.sort(key=rank_key)
    final_problems[dataset] = {
        "schema_version": SCHEMA_VERSION,
        "dataset": dataset,
        "cities": cities,
        "optimal_cost": optimal,
        "leaderboard": board,
    }

for dataset, doc in sorted(final_problems.items()):
    write_json(out / "problems" / f"{dataset}.json", doc)

# --- index.json ------------------------------------------------------------
#
# Deliberately carries NO commit, dirty flag, version or environment. Those belong to each
# solver's own measurement, and a single value here would present twenty solvers that were
# measured at different commits as if they shared one. The manifest says what exists; the
# shards say when each thing was measured.
def solver_meta(sid: str) -> dict:
    entry = solver_entry(sid)
    if entry is None:
        # A shard carried over from a previous publish whose solver is no longer in
        # solvers.json: keep listing it rather than dropping data silently.
        return {"name": sid, "family": None}
    return {"name": entry["name"], "family": entry.get("family")}

write_json(out / "index.json", {
    "schema_version": SCHEMA_VERSION,
    "generated_at": generated_at,
    "tier": tier or None,
    "default_timeout_s": int(provenance["timeout_s"]) if provenance.get("timeout_s", "").isdigit() else None,
    "budgets": budgets,
    "budgets_note": budgets_comment,
    "solvers": [
        {"id": sid, "configs": final_algos[sid].get("configs", []), **solver_meta(sid)}
        for sid in sorted(final_algos)
    ],
    "problems": [
        {
            "id": dataset,
            "cities": doc.get("cities"),
            "optimal_cost": doc.get("optimal_cost"),
        }
        for dataset, doc in sorted(final_problems.items())
    ],
})

# Handed to the criteriondb mirror so the archived runs record the commit that produced the
# numbers rather than whatever HEAD happened to be when the publish ran — the same distinction
# the per-solver shards make. Written beside the staged tree, not into it, so it is not uploaded.
(out.parent / "_provenance.json").write_text(
    json.dumps(measurement, indent=2, sort_keys=False) + "\n", encoding="utf-8"
)

print(f"    measured:   {len(measured)} solver(s) from {tsv_path}")
print(f"    algorithms: {len(final_algos)} shard(s) written"
      + (f" ({len(final_algos) - len(measured)} carried over)" if len(final_algos) > len(measured) else ""))
print(f"    problems:   {len(final_problems)} shard(s) written")
print(f"    cells:      {len(cells)}  runs: {len(rows)}")
PY

echo "==> Staged JSON:"
find "$STAGE/v2" -name '*.json' | sort | sed 's/^/    /' | head -40
if [[ "$(find "$STAGE/v2" -name '*.json' | wc -l)" -gt 40 ]]; then
  echo "    ... ($(find "$STAGE/v2" -name '*.json' | wc -l) files total)"
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

# ---------------------------------------------------------------------------
# Release snapshots
#
# A snapshot is the complete tree as it stood at a release tag, so a regression can later be
# evaluated against what an algorithm actually produced in that release. It is immutable by
# design: re-writing a tag would silently redefine what that release measured.
# ---------------------------------------------------------------------------

snapshot_stage() {
  local tag="$1"
  # Validated before it reaches a path: a tag like `..` or one containing `/` would otherwise
  # let `rm -rf` below delete outside the snapshots directory.
  if [[ ! "$tag" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    echo "ERROR: invalid snapshot tag '$tag'." >&2
    echo "       Use letters, digits, dot, dash or underscore, starting with a letter or digit." >&2
    return 1
  fi
  # Kept as two statements: `local tag="$1" snap=...$tag` reads $tag before it is bound.
  local snap="$STAGE/v2/snapshots/$tag"

  if [[ "$DRY_RUN" -eq 0 && "$ONLY" != "criteriondb" && "$FORCE_SNAPSHOT" -eq 0 ]]; then
    # Fail closed. A network error or a 5xx must not be read as "the tag is free", because
    # then a transient failure would authorise overwriting an immutable snapshot.
    local status
    status="$(curl -sS -o /dev/null -w '%{http_code}' --head \
      "$PUBLIC_BASE/${PREFIX}/snapshots/$tag/index.json" || true)"
    status="${status:-000}"
    case "$status" in
      404) ;;
      200)
        echo "ERROR: snapshot '$tag' already exists at ${PREFIX}/snapshots/$tag." >&2
        echo "       Snapshots are immutable; pass --force-snapshot to overwrite it." >&2
        return 1
        ;;
      *)
        echo "ERROR: cannot confirm whether snapshot '$tag' exists (HTTP $status)." >&2
        echo "       Refusing to risk overwriting an immutable snapshot; retry, or pass" >&2
        echo "       --force-snapshot if you intend to replace it." >&2
        return 1
        ;;
    esac
  fi

  rm -rf "$snap"
  mkdir -p "$snap"
  cp "$STAGE/v2/index.json" "$snap/index.json"
  cp -r "$STAGE/v2/algorithms" "$STAGE/v2/problems" "$snap/"
  echo "==> Snapshot  snapshots/$tag ($(find "$snap" -name '*.json' | wc -l) object(s))"
}

upload_r2() {
  # A dry run uploads nothing, so it must not require a client: CI has neither `cf` nor
  # `wrangler`, and being able to build and inspect the tree without credentials is the point
  # of --dry-run.
  if [[ "$DRY_RUN" -eq 0 && -z "$R2_CLIENT" ]]; then
    echo "ERROR: no R2 client available — install the 'cf' CLI:" >&2
    echo "    npm i -g cf" >&2
    echo "('wrangler' also works as a fallback if 'cf' cannot be installed.)" >&2
    return 1
  fi
  echo "==> Uploading to R2 bucket '$CF_R2_BUCKET' prefix '$PREFIX' via ${R2_CLIENT:-nothing}$([[ "$DRY_RUN" -eq 1 ]] && echo ' (dry run)')"
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
  #
  # mapfile, not `for file in $(find ...)`: unquoted command substitution
  # word-splits and glob-expands, so a staged path containing whitespace (or a
  # `*`) would be silently mangled into several bogus "filenames".
  local -a shards=() manifests=() snapshot_filter=()
  # Without --snapshot there is no snapshot in this run's tree, and any snapshots/ directory
  # still on disk belongs to an earlier one: re-uploading it would rewrite an immutable
  # snapshot and skip the existence guard.
  [[ -n "$SNAPSHOT_TAG" ]] || snapshot_filter=(-not -path '*/snapshots/*')
  mapfile -t shards < <(find "$STAGE/v2" -name '*.json' ! -name index.json "${snapshot_filter[@]}" | sort)
  # Every index.json goes last *within its own directory*: the top-level manifest advertises
  # the top-level shards, and each snapshot's manifest advertises that snapshot's shards.
  mapfile -t manifests < <(find "$STAGE/v2" -name 'index.json' "${snapshot_filter[@]}" | sort)
  local -a files=("${shards[@]}" "${manifests[@]}")

  local skipped=0
  for file in "${files[@]}"; do
    key="${file#"$STAGE/v2/"}"
    # Skip objects whose bytes are unchanged. This is what makes a differential publish touch
    # only the shard it re-measured: every other object keeps its ETag, its provenance and its
    # place in the tree.
    if [[ "$MERGE_PREVIOUS" -eq 1 && -f "$PREVIOUS/v2/$key" ]] && cmp -s "$file" "$PREVIOUS/v2/$key"; then
      skipped=$((skipped + 1))
      continue
    fi
    if [[ "$DRY_RUN" -eq 1 ]]; then
      echo "    [dry-run] would put ${PREFIX}/${key}"
    else
      r2_put "$key" "$file" >/dev/null
      echo "    put ${PREFIX}/${key}"
    fi
    count=$((count + 1))
  done
  echo "==> $count object(s) $([[ "$DRY_RUN" -eq 1 ]] && echo 'would be ' || echo '')uploaded, $skipped unchanged"
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
        # "tsv" means the file recorded the commit its measurements came from; "publish-time"
        # means it did not, so git_commit is only HEAD when the publish ran.
        "git_commit_source": os.environ.get("GIT_COMMIT_SOURCE", "publish-time"),
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

if [[ -n "$SNAPSHOT_TAG" ]]; then
  snapshot_stage "$SNAPSHOT_TAG" || exit 1
fi

if [[ "$ONLY" == "r2" || "$ONLY" == "both" ]]; then
  upload_r2
fi

if [[ "$ONLY" == "criteriondb" || "$ONLY" == "both" ]]; then
  # Provenance for the archived run. Prefer the commit the TSV recorded: under the
  # differential cadence these runs may not all come from HEAD, and stamping them with the
  # publish-time commit would attach a claim the measurements do not support.
  if [[ -f "$STAGE/_provenance.json" ]]; then
    MEASURED_COMMIT="$(jq -r '.git_commit // empty' "$STAGE/_provenance.json" 2>/dev/null)"
    MEASURED_SOURCE="$(jq -r '.git_commit_source // empty' "$STAGE/_provenance.json" 2>/dev/null)"
  fi
  GIT_BRANCH_NAME="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo)"
  export GIT_COMMIT="${MEASURED_COMMIT:-$(git rev-parse HEAD 2>/dev/null || echo)}"
  export GIT_COMMIT_SOURCE="${MEASURED_SOURCE:-publish-time}"
  export GIT_BRANCH="$GIT_BRANCH_NAME"
  export BENCH_TIER="${TIER:-}"
  mirror_criteriondb
fi

echo "==> Done."
if [[ "$DRY_RUN" -eq 0 && "$ONLY" != "criteriondb" ]]; then
  echo "    Verify: scripts/read-benchmarks.sh index"
fi

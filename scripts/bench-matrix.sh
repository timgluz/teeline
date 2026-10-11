#!/usr/bin/env bash
# bench-matrix.sh — run a tier's benchmark matrix and append the results as JSONL.
#
# Usage:
#   scripts/bench-matrix.sh bench/matrix/tier-a.toml
#   scripts/bench-matrix.sh bench/matrix/tier-a.toml --solver lk
#   scripts/bench-matrix.sh bench/matrix/tier-a.toml --dry-run
#   task bench:tierA
#
# Reads:  a tier config (bench/matrix/tier-*.toml), bench/solvers.json
# Writes: bench/runs/tier-<tier>.jsonl  (git-ignored; append-only)
#
# The row shape is documented in docs/benchmarks/schema.md. Two properties matter more than
# the rest:
#
#   * Provenance travels on every row, not in a file header. This sweep is resumable and
#     --solver appends to an existing file, so a run interrupted by a rebuild leaves rows
#     from two builds. A file-level header would stamp the older rows with the newer commit;
#     per-row provenance makes that detectable, and publish-benchmarks.sh refuses a file
#     whose rows for one solver disagree rather than picking a winner.
#
#   * A cell that produced nothing is recorded as such, never fabricated. A killed run has no
#     tour, so it carries no tour_cost — a 0 would be a −100% gap, "twice as good as optimal".

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="${TEELINE_BIN:-./target/release/teeline}"
GNU_TIME="${GNU_TIME:-/usr/bin/time}"
SOLVERS_JSON="bench/solvers.json"
RUNS_OVERRIDE=""
OUT_OVERRIDE=""
DRY_RUN=0
FORCE=0
SKIP_VALIDATE=0
declare -a SOLVER_FILTER=()

usage() {
  sed -n '2,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  cat <<'EOF'

Options:
  --solver <id>     Measure only this solver (doc id or CLI name; repeatable)
  --runs <n>        Override the tier's runs-per-cell
  --out <path>      Override the output path (default: bench/runs/tier-<tier>.jsonl)
  --binary <path>   Override the binary (default: $TEELINE_BIN or target/release/teeline)
  --force           Re-run cells already present in the output
  --skip-validate   Do not pre-check that every planned instance parses
  --dry-run         List the cells that would run, and run nothing
  -h, --help        Show this help
EOF
}

TIER_TOML="${1:-}"
if [[ -z "$TIER_TOML" || "$TIER_TOML" == "-h" || "$TIER_TOML" == "--help" ]]; then
  usage >&2
  [[ -n "$TIER_TOML" ]] && exit 0
  exit 2
fi
shift

while [[ $# -gt 0 ]]; do
  case "$1" in
    --solver) SOLVER_FILTER+=("${2:?}"); shift 2 ;;
    --runs) RUNS_OVERRIDE="${2:?}"; shift 2 ;;
    --out) OUT_OVERRIDE="${2:?}"; shift 2 ;;
    --binary) BIN="${2:?}"; shift 2 ;;
    --force) FORCE=1; shift ;;
    --skip-validate) SKIP_VALIDATE=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ -f "$TIER_TOML" ]] || { echo "ERROR: tier config not found: $TIER_TOML" >&2; exit 1; }
[[ -f "$SOLVERS_JSON" ]] || { echo "ERROR: $SOLVERS_JSON not found" >&2; exit 1; }

for tool in python3 jq; do
  command -v "$tool" >/dev/null 2>&1 || { echo "ERROR: $tool is required" >&2; exit 1; }
done
if [[ "$DRY_RUN" -eq 0 ]]; then
  [[ -x "$GNU_TIME" ]] || { echo "ERROR: GNU time not found at $GNU_TIME (needed for peak RSS)" >&2; exit 1; }
  [[ -x "$BIN" ]] || { echo "ERROR: solver binary not found: $BIN (cargo build --release -p teeline-cli)" >&2; exit 1; }
fi

# ---------------------------------------------------------------------------
# Provenance — resolved once, written onto every row
#
# The commit that produced the measurements, not whatever HEAD happens to be when the file is
# read. `git_commit_source: "jsonl"` says the file recorded it, so a consumer knows the rows
# are attributable rather than merely current.
# ---------------------------------------------------------------------------

GIT_COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
if git diff-index --quiet HEAD -- 2>/dev/null; then DIRTY=false; else DIRTY=true; fi
TEELINE_VERSION="$(awk '/^version *=/ {gsub(/[" ]/,"",$3); print $3; exit}' Cargo.toml 2>/dev/null || echo unknown)"
RUST_VERSION="$(rustc --version 2>/dev/null | awk '{print $2}' || echo unknown)"
OS_NAME="$(uname -s | tr '[:upper:]' '[:lower:]')"
MEASURED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

MEASUREMENT="$(jq -cn \
  --arg commit "$GIT_COMMIT" \
  --argjson dirty "$DIRTY" \
  --arg version "$TEELINE_VERSION" \
  --arg measured_at "$MEASURED_AT" \
  --arg os "$OS_NAME" \
  --arg rust "$RUST_VERSION" \
  '{git_commit:$commit, git_commit_source:"jsonl", dirty:$dirty, teeline_version:$version,
    measured_at:$measured_at,
    environment:{os:$os, build:"release", rust_version:$rust}}')"

# ---------------------------------------------------------------------------
# Plan the cells
#
# The planner owns the tier semantics — families, instance groups, configs, budgets — and
# emits one JSON object per cell. Keeping it in one place means the bash below is only a
# runner, and the same planner can answer --dry-run.
# ---------------------------------------------------------------------------

PLAN_ARGS=(--tier "$TIER_TOML" --solvers-json "$SOLVERS_JSON")
[[ -n "$RUNS_OVERRIDE" ]] && PLAN_ARGS+=(--runs "$RUNS_OVERRIDE")
[[ "$FORCE" -eq 1 ]] && PLAN_ARGS+=(--force)
for s in "${SOLVER_FILTER[@]:-}"; do
  [[ -n "$s" ]] && PLAN_ARGS+=(--solver "$s")
done

TIER_NAME="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["tier"])' "$TIER_TOML")"
OUT="${OUT_OVERRIDE:-bench/runs/tier-${TIER_NAME}.jsonl}"
# Always resume from the output file, whichever way its path was chosen: --out names the file,
# it does not ask for a fresh one. (Getting this backwards silently re-ran every cell.)
PLAN_ARGS+=(--existing "$OUT")

mkdir -p "$(dirname "$OUT")"

# A previous run killed mid-write can leave a half-written final line, which is not valid
# JSON and would make the publisher reject the whole file. Drop it before continuing: it
# recorded nothing, so re-running the cell is the honest response.
if [[ -s "$OUT" ]]; then
  last="$(tail -n 1 "$OUT")"
  if [[ -n "$last" ]] && ! jq -e . >/dev/null 2>&1 <<<"$last"; then
    echo "WARN: dropping an incomplete final line from $OUT (interrupted write)" >&2
    head -n -1 "$OUT" > "$OUT.tmp" || true
    mv "$OUT.tmp" "$OUT"
  fi
fi

declare -a CELLS=()
plan_file="$(mktemp)"
# The plan is written to a file rather than piped, so the planner's exit status is actually
# checked. Under process substitution it is discarded, and a planner that failed — a missing
# instance, an unknown solver, a malformed tier — would look exactly like a sweep with nothing
# left to do. That is a silent hole, which is the one thing this harness must not produce.
if ! python3 "$ROOT/scripts/bench-matrix-plan.py" "${PLAN_ARGS[@]}" > "$plan_file"; then
  rm -f "$plan_file"
  echo "ERROR: could not plan the sweep (see above); nothing was run" >&2
  exit 1
fi
while IFS= read -r line; do
  [[ -n "$line" ]] && CELLS+=("$line")
done < "$plan_file"
rm -f "$plan_file"

echo "==> Tier $TIER_NAME — $((${#CELLS[@]})) cell(s) to run"
echo "    out:        $OUT"
echo "    commit:     $GIT_COMMIT (dirty=$DIRTY)"
echo "    measured_at: $MEASURED_AT"

if [[ "$DRY_RUN" -eq 1 ]]; then
  # Grouped by cell rather than by run: a tier is 12 instances x 5 runs, so listing every run
  # buries the shape of the sweep under repetition of the same line.
  declare -A run_counts=()
  for cell in "${CELLS[@]}"; do
    key="$(jq -r '[.solver,.instance,.config,(.timeout_s|tostring),.path]|@tsv' <<<"$cell")"
    run_counts["$key"]=$(( ${run_counts["$key"]:-0} + 1 ))
  done
  declare -a keys=()
  mapfile -t keys < <(printf '%s\n' "${!run_counts[@]}" | sort)
  for key in "${keys[@]}"; do
    IFS=$'\t' read -r d_solver d_instance d_config d_timeout d_path <<<"$key"
    printf '    %-14s %-10s %-8s timeout=%-6s runs=%-3s %s\n' \
      "$d_solver" "$d_instance" "$d_config" "${d_timeout}s" "${run_counts[$key]}" "$d_path"
  done
  echo "==> ${#keys[@]} cell(s), ${#CELLS[@]} run(s) — dry run, nothing executed"
  exit 0
fi

if [[ "${#CELLS[@]}" -eq 0 ]]; then
  echo "==> nothing to do"
  exit 0
fi

# ---------------------------------------------------------------------------
# Validate the instances before spending the budget
#
# A missing or unparseable instance must fail loudly. Discovering it three hours into a sweep
# — or worse, recording it as an `error` row among hundreds of real ones — is how a silent
# hole gets published. `nn` is the cheapest solver, so it is the probe.
# ---------------------------------------------------------------------------

if [[ "$SKIP_VALIDATE" -eq 0 ]]; then
  declare -A checked=()
  for cell in "${CELLS[@]}"; do
    path="$(jq -r .path <<<"$cell")"
    [[ -n "${checked[$path]:-}" ]] && continue
    checked[$path]=1
    if ! "$BIN" solve nn -i "$path" >/dev/null 2>"$path.validate.err"; then
      echo "ERROR: $path does not parse, or the solver cannot read it:" >&2
      sed 's/^/       /' "$path.validate.err" >&2
      rm -f "$path.validate.err"
      exit 1
    fi
    rm -f "$path.validate.err"
  done
  echo "==> validated ${#checked[@]} instance file(s)"
fi

# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------

ok=0; timeouts=0; errors=0
declare -a extra_args=()
tmp_out="$(mktemp)"; tmp_err="$(mktemp)"; tmp_time="$(mktemp)"
trap 'rm -f "$tmp_out" "$tmp_err" "$tmp_time"' EXIT

for cell in "${CELLS[@]}"; do
  solver="$(jq -r .solver <<<"$cell")"
  cli="$(jq -r .cli <<<"$cell")"
  instance="$(jq -r .instance <<<"$cell")"
  path="$(jq -r .path <<<"$cell")"
  config="$(jq -r .config <<<"$cell")"
  run_no="$(jq -r .run <<<"$cell")"
  timeout_s="$(jq -r .timeout_s <<<"$cell")"
  extra_args=()
  while IFS= read -r a; do extra_args+=("$a"); done < <(jq -r '.args[]?' <<<"$cell")

  start="$(date +%s.%N)"
  set +e
  "$GNU_TIME" -v -o "$tmp_time" \
    timeout -k 5 "${timeout_s}s" \
    "$BIN" solve "$cli" -i "$path" ${extra_args[@]+"${extra_args[@]}"} \
    >"$tmp_out" 2>"$tmp_err"
  rc=$?
  set -e
  end="$(date +%s.%N)"
  wall="$(awk -v a="$start" -v b="$end" 'BEGIN{printf "%.4f", b-a}')"
  rss="$(awk '/Maximum resident set size/ {print $NF}' "$tmp_time" || true)"
  [[ -n "$rss" ]] || rss=null

  tour="null"
  status="error"
  if [[ "$rc" -eq 0 ]]; then
    tour="$(head -n 1 "$tmp_out" | awk '{print $1}')"
    if [[ -n "$tour" ]]; then status="ok"; else status="error"; fi
  elif [[ "$rc" -eq 124 || "$rc" -eq 137 ]]; then
    # 124 is `timeout` giving up; 137 is the follow-up SIGKILL. Either way the run produced
    # nothing, so it carries no tour_cost.
    status="timeout"
  fi

  # wall_s and peak_rss_kb are observations even for a run that timed out, so they are always
  # recorded. `tour_cost` is not: a killed run has no tour, and a 0 there would read as a -100%
  # gap. The publisher drops the numerics for non-ok rows regardless; this is the raw record.
  row="$(jq -cn \
    --arg solver "$solver" --arg dataset "$instance" --arg config "$config" \
    --argjson run "$run_no" --arg status "$status" \
    --argjson wall "$wall" --argjson rss "$rss" --argjson tour "${tour:-null}" \
    --argjson m "$MEASUREMENT" \
    '{solver:$solver, dataset:$dataset, config:$config, run:$run, status:$status}
     + {wall_s:$wall, peak_rss_kb:$rss}
     + (if $status == "ok" then {tour_cost:$tour} else {} end)
     + {measurement:$m}')"
  printf '%s\n' "$row" >> "$OUT"

  case "$status" in
    ok) ok=$((ok + 1)); printf '    ok       %-14s %-10s %-8s run %s  %ss  %s\n' "$solver" "$instance" "$config" "$run_no" "$wall" "$tour" ;;
    timeout) timeouts=$((timeouts + 1)); printf '    timeout  %-14s %-10s %-8s run %s  (%ss)\n' "$solver" "$instance" "$config" "$run_no" "$timeout_s" ;;
    *) errors=$((errors + 1)); printf '    error    %-14s %-10s %-8s run %s  rc=%s: %s\n' "$solver" "$instance" "$config" "$run_no" "$rc" "$(head -n 1 "$tmp_err")" ;;
  esac
done

echo "==> $ok ok, $timeouts timeout, $errors error — appended to $OUT"
if [[ "$errors" -gt 0 ]]; then
  echo "    NOTE: error rows are recorded, not hidden; investigate before publishing." >&2
  exit 1
fi

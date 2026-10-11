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
# The row shape is documented in docs/benchmarks/schema.md. Two properties matter more than the
# rest:
#
#   * Provenance travels on every row, not in a file header. This sweep is resumable and --solver
#     appends to an existing file, so a run interrupted by a rebuild leaves rows from two builds.
#     A file-level header would stamp the older rows with the newer commit; per-row provenance
#     makes that detectable, and publish-benchmarks.sh refuses a file whose rows for one solver
#     disagree rather than picking a winner.
#
#   * A cell that produced nothing is recorded as such, never fabricated. A killed run has no
#     tour, so it carries no tour_cost — a 0 would be a -100% gap, "twice as good as optimal".

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
RETRY_ERRORS=0
SKIP_VALIDATE=0
declare -a SOLVER_FILTER=()

usage() {
  # Through the `Writes:` line only: past it the header explains *why* the format is what it
  # is, and pulling that into --help leaves the usage text ending mid-sentence.
  sed -n '2,11p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  cat <<'EOF'

Options:
  --solver <id>     Measure only this solver (doc id or CLI name; repeatable)
  --runs <n>        Override the tier's runs-per-cell
  --out <path>      Override the output path (default: bench/runs/tier-<tier>.jsonl)
  --binary <path>   Override the binary (default: $TEELINE_BIN or target/release/teeline)
  --force           Re-run every cell, ignoring what the output already holds
  --retry-errors    Drop recorded `error` rows and measure those cells again
  --skip-validate   Do not pre-check that every planned instance parses
  --dry-run         List the cells that would run, and change nothing
  -h, --help        Show this help
EOF
}

TIER_TOML="${1:-}"
if [[ -z "$TIER_TOML" || "$TIER_TOML" == "-h" || "$TIER_TOML" == "--help" ]]; then
  # An explicit request for help is output, not a diagnostic, so it goes to stdout.
  usage
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
    --retry-errors) RETRY_ERRORS=1; shift ;;
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

TIER_NAME="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["tier"])' "$TIER_TOML")"
OUT="${OUT_OVERRIDE:-bench/runs/tier-${TIER_NAME}.jsonl}"

# ---------------------------------------------------------------------------
# Provenance — resolved once, written onto every row
#
# The commit that produced the measurements, not whatever HEAD happens to be when the file is
# read. `git_commit_source: "jsonl"` says the file recorded it, so a consumer knows the rows are
# attributable rather than merely current.
# ---------------------------------------------------------------------------

GIT_COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
if git diff-index --quiet HEAD -- 2>/dev/null; then DIRTY=false; else DIRTY=true; fi
TEELINE_VERSION="$(awk '/^version *=/ {gsub(/[" ]/,"",$3); print $3; exit}' Cargo.toml 2>/dev/null || echo unknown)"
RUST_VERSION="$(rustc --version 2>/dev/null | awk '{print $2}' || echo unknown)"
OS_NAME="$(uname -s | tr '[:upper:]' '[:lower:]')"
MEASURED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# The build profile has to come from the binary actually run: --binary and TEELINE_BIN are how
# the tests and a local run point at a debug build, and publishing debug timings labelled
# "release" would misstate every number in the row.
case "$BIN" in
  */release/*) BUILD_PROFILE="release" ;;
  */debug/*) BUILD_PROFILE="debug" ;;
  *) BUILD_PROFILE="unknown" ;;
esac

MEASUREMENT="$(jq -cn \
  --arg commit "$GIT_COMMIT" \
  --argjson dirty "$DIRTY" \
  --arg version "$TEELINE_VERSION" \
  --arg measured_at "$MEASURED_AT" \
  --arg os "$OS_NAME" \
  --arg build "$BUILD_PROFILE" \
  --arg rust "$RUST_VERSION" \
  '{git_commit:$commit, git_commit_source:"jsonl", dirty:$dirty, teeline_version:$version,
    measured_at:$measured_at,
    environment:{os:$os, build:$build, rust_version:$rust}}')"

# A resumed sweep must keep the measurement its earlier rows already carry. `measured_at` is
# resolved per process, so reusing the stored block is what stops a resume from writing rows for
# the *same* solver that disagree on it — and the publisher, which compares the whole measurement,
# cannot tell a clock tick from a rebuild, so it would refuse the file and break resume entirely.
# When the build really has changed the identities differ, the new measurement is used, and the
# disagreement surfaces at publish time exactly as intended.
if [[ -s "$OUT" ]]; then
  stored="$(head -n 1 "$OUT" 2>/dev/null | jq -c '.measurement // empty' 2>/dev/null || true)"
  if [[ -n "$stored" ]] && jq -en --argjson a "$stored" --argjson b "$MEASUREMENT" '
        ($a.git_commit == $b.git_commit)
        and ($a.dirty == $b.dirty)
        and ($a.teeline_version == $b.teeline_version)
        and ($a.environment == $b.environment)' >/dev/null; then
    MEASUREMENT="$stored"
    MEASURED_AT="$(jq -r '.measured_at // "unknown"' <<<"$MEASUREMENT")"
    echo "==> reusing the measurement already in $OUT (same build, measured_at $MEASURED_AT)"
  fi
fi

count_status() {
  [[ -s "$OUT" ]] || { echo 0; return 0; }
  local n
  n="$(jq -r '.status? // empty' "$OUT" 2>/dev/null | grep -c "^$1\$" || true)"
  echo "${n:-0}"
}

# A previous run killed mid-write leaves a half-written final line that is not valid JSON and
# would make the publisher reject the whole file. It recorded nothing, so dropping it and
# measuring the cell again is the honest response.
repair_output() {
  [[ -s "$OUT" ]] || return 0
  # A valid final row with no trailing newline would be concatenated with the next append,
  # producing one unparseable line where two good rows should be.
  if [[ -n "$(tail -c 1 "$OUT" | tr -d '\n')" ]]; then
    printf '\n' >> "$OUT"
  fi
  local last
  last="$(tail -n 1 "$OUT")"
  [[ -n "$last" ]] || return 0
  if ! jq -e . >/dev/null 2>&1 <<<"$last"; then
    echo "WARN: dropping an incomplete final line from $OUT (interrupted write)" >&2
    # Rewritten to a sibling and swapped only on success: a failed `head` (full disk, unreadable
    # file) must leave the original alone rather than replace it with a partial copy.
    if ! head -n -1 "$OUT" > "$OUT.repair"; then
      rm -f "$OUT.repair"
      echo "ERROR: could not rewrite $OUT; leaving it untouched" >&2
      exit 1
    fi
    mv "$OUT.repair" "$OUT"
  fi
}

retry_error_rows() {
  [[ -s "$OUT" ]] || return 0
  local n
  n="$(count_status error)"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "    --retry-errors: would drop $n error row(s) from $OUT"
    return 0
  fi
  [[ "$n" -gt 0 ]] || return 0
  if ! jq -c 'select(.status != "error")' "$OUT" > "$OUT.retry"; then
    rm -f "$OUT.retry"
    echo "ERROR: could not rewrite $OUT to drop error rows" >&2
    exit 1
  fi
  mv "$OUT.retry" "$OUT"
  echo "==> dropped $n error row(s) from $OUT to measure again"
}

# Everything that mutates the output happens only for a real run: `--dry-run` promises to change
# nothing, and creating the directory or dropping a truncated line are both changes.
if [[ "$DRY_RUN" -eq 0 ]]; then
  mkdir -p "$(dirname "$OUT")"
  repair_output
  [[ "$RETRY_ERRORS" -eq 1 ]] && retry_error_rows
elif [[ "$RETRY_ERRORS" -eq 1 ]]; then
  retry_error_rows
fi

# ---------------------------------------------------------------------------
# Plan the cells
#
# The planner owns the tier semantics — families, instance groups, configs, budgets — and emits
# one JSON object per cell, so the bash below is only a runner and `--dry-run` is the same plan
# without the running.
# ---------------------------------------------------------------------------

PLAN_ARGS=(--tier "$TIER_TOML" --solvers-json "$SOLVERS_JSON" --existing "$OUT")
[[ -n "$RUNS_OVERRIDE" ]] && PLAN_ARGS+=(--runs "$RUNS_OVERRIDE")
[[ "$FORCE" -eq 1 ]] && PLAN_ARGS+=(--force)
for s in "${SOLVER_FILTER[@]:-}"; do
  [[ -n "$s" ]] && PLAN_ARGS+=(--solver "$s")
done

echo "==> Tier $TIER_NAME"
echo "    out:         $OUT"
echo "    commit:      $GIT_COMMIT (dirty=$DIRTY)"
echo "    profile:     $BUILD_PROFILE"
echo "    measured_at: $MEASURED_AT"

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

echo "    to run:      ${#CELLS[@]} cell(s)"

if [[ "$DRY_RUN" -eq 1 ]]; then
  if [[ "${#CELLS[@]}" -eq 0 ]]; then
    echo "==> 0 cell(s), 0 run(s) — dry run, nothing executed"
    exit 0
  fi
  # Grouped by cell rather than by run: a tier is 12 instances x 5 runs, so listing every run
  # buries the shape of the sweep under repetition of the same line.
  declare -A run_counts=()
  for cell in "${CELLS[@]}"; do
    key="$(jq -r '[.solver,.instance,.config,(.timeout_s|tostring),.path]|@tsv' <<<"$cell")"
    run_counts["$key"]=$(( ${run_counts["$key"]:-0} + 1 ))
  done
  declare -a keys=()
  mapfile -t keys < <(printf '%s\n' "${!run_counts[@]}" | grep -v '^$' | sort)
  for key in "${keys[@]}"; do
    IFS=$'\t' read -r d_solver d_instance d_config d_timeout d_path <<<"$key"
    printf '    %-14s %-10s %-8s timeout=%-6s runs=%-3s %s\n' \
      "$d_solver" "$d_instance" "$d_config" "${d_timeout}s" "${run_counts[$key]}" "$d_path"
  done
  echo "==> ${#keys[@]} cell(s), ${#CELLS[@]} run(s) — dry run, nothing executed"
  exit 0
fi

if [[ "${#CELLS[@]}" -eq 0 ]]; then
  # "Nothing to do" is only true if there is nothing to worry about either: a sweep whose cells
  # are all recorded but which contains errors would otherwise look finished.
  existing_errors="$(count_status error)"
  if [[ "$existing_errors" -gt 0 ]]; then
    echo "ERROR: nothing left to plan, but $OUT records $existing_errors error row(s)." >&2
    echo "       Re-run with --retry-errors to measure those cells again, or investigate them." >&2
    exit 1
  fi
  echo "==> nothing to do"
  exit 0
fi

# ---------------------------------------------------------------------------
# Validate the instances before spending the budget
#
# A missing or unparseable instance must fail loudly. Discovering it three hours into a sweep —
# or worse, recording it as an `error` row among hundreds of real ones — is how a silent hole
# gets published. `nn` is the cheapest solver, so it is the probe.
# ---------------------------------------------------------------------------

if [[ "$SKIP_VALIDATE" -eq 0 ]]; then
  declare -A checked=()
  validate_err="$(mktemp)"
  for cell in "${CELLS[@]}"; do
    path="$(jq -r .path <<<"$cell")"
    [[ -n "${checked[$path]:-}" ]] && continue
    checked[$path]=1
    if ! "$BIN" solve nn -i "$path" >/dev/null 2>"$validate_err"; then
      echo "ERROR: $path does not parse, or the solver cannot read it:" >&2
      sed 's/^/       /' "$validate_err" >&2
      rm -f "$validate_err"
      exit 1
    fi
  done
  rm -f "$validate_err"
  echo "==> validated ${#checked[@]} instance file(s)"
fi

# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------

# A value the publisher would reject must never be written as an `ok` row: it would fail the
# whole file at publish time, hours after the run. Such a cell becomes `error` instead, so it is
# visible rather than fatal.
is_positive_number() {
  [[ "$1" =~ ^[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?$ ]] && awk -v v="$1" 'BEGIN{exit !(v > 0)}'
}

# 137 is SIGKILL, which `timeout -k` sends — but also what the OOM killer and any external kill
# produce. Only a run that actually reached its budget counts as a timeout; anything else is a
# failure that would otherwise be misreported as the family's budget being exhausted.
reached_budget() {
  awk -v wall="$1" -v budget="$2" 'BEGIN{exit !(wall >= budget - 0.5)}'
}

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

  tour=""
  reason=""
  status="error"
  if [[ "$rc" -eq 0 ]]; then
    status="ok"
    tour="$(head -n 1 "$tmp_out" | awk '{print $1}')"
    if ! is_positive_number "$tour"; then
      status="error"; reason="unusable tour_cost '$tour'"
    elif [[ ! "$rss" =~ ^[0-9]+$ ]]; then
      status="error"; reason="GNU time reported no peak RSS"
    fi
  elif [[ "$rc" -eq 124 ]]; then
    status="timeout"
  elif [[ "$rc" -eq 137 ]] && reached_budget "$wall" "$timeout_s"; then
    status="timeout"
  else
    reason="rc=$rc $(head -n 1 "$tmp_err")"
  fi

  # wall_s and peak_rss_kb are observations even for a run that timed out, so they are always
  # recorded. `tour_cost` is not: a killed run has no tour, and a 0 there would read as a -100%
  # gap. The publisher drops the numerics for non-ok rows regardless; this is the raw record.
  if [[ "$status" == "ok" ]]; then
    row="$(jq -cn \
      --arg solver "$solver" --arg dataset "$instance" --arg config "$config" \
      --argjson run "$run_no" --arg status "$status" \
      --argjson wall "$wall" --argjson rss "$rss" --argjson tour "$tour" \
      --argjson m "$MEASUREMENT" \
      '{solver:$solver, dataset:$dataset, config:$config, run:$run, status:$status,
        wall_s:$wall, peak_rss_kb:$rss, tour_cost:$tour, measurement:$m}')"
  else
    row="$(jq -cn \
      --arg solver "$solver" --arg dataset "$instance" --arg config "$config" \
      --argjson run "$run_no" --arg status "$status" \
      --argjson wall "$wall" --argjson rss "${rss:-null}" \
      --argjson m "$MEASUREMENT" \
      '{solver:$solver, dataset:$dataset, config:$config, run:$run, status:$status,
        wall_s:$wall, peak_rss_kb:$rss, measurement:$m}')"
  fi
  printf '%s\n' "$row" >> "$OUT"

  case "$status" in
    ok) ok=$((ok + 1)); printf '    ok       %-14s %-10s %-8s run %s  %ss  %s\n' "$solver" "$instance" "$config" "$run_no" "$wall" "$tour" ;;
    timeout) timeouts=$((timeouts + 1)); printf '    timeout  %-14s %-10s %-8s run %s  (%ss)\n' "$solver" "$instance" "$config" "$run_no" "$timeout_s" ;;
    *) errors=$((errors + 1)); printf '    error    %-14s %-10s %-8s run %s  %s\n' "$solver" "$instance" "$config" "$run_no" "$reason" ;;
  esac
done

echo "==> $ok ok, $timeouts timeout, $errors error — appended to $OUT"
if [[ "$errors" -gt 0 ]]; then
  echo "    NOTE: error rows are recorded, not hidden; re-run with --retry-errors to measure them" >&2
  exit 1
fi

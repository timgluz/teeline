#!/usr/bin/env bash
# scripts/bench-solvers.sh
#
# End-to-end solver benchmark: runs nn/lk/fourier/branch_bound on representative
# TSPLIB datasets, capturing wall time (seconds), peak RSS (kB), and tour cost
# via GNU `time -v`.
#
# Usage:
#   task bench:solvers                    # prints raw TSV to stdout
#   task bench:solvers -- bench/out.tsv   # also writes raw TSV to bench/out.tsv
#   RUNS=3 task bench:solvers             # override runs per combo (default 5)
#
# Output columns (tab-separated):
#   solver  dataset  run  wall_s  peak_rss_kb  tour_cost
#
# Requires: release binary at target/release/teeline, GNU time at /usr/bin/time,
# and TSPLIB data files under tests/fixtures/ (tracked) or data/tsplib/.

set -euo pipefail

BIN="${TEELINE_BIN:-./target/release/teeline}"
RUNS="${RUNS:-5}"
GNU_TIME=/usr/bin/time

# solver | dataset1 dataset2 ...
declare -A DATASETS=(
  [nn]="berlin52 a280 att532"
  [lk]="berlin52 a280 att532"
  [fourier]="berlin52 a280"
  [branch_bound]="burma14 gr17"
)

# CLI alias map (solver key to canonical CLI name).
declare -A ALIAS=(
  [nn]="nn"
  [lk]="lk"
  [fourier]="fourier"
  [branch_bound]="branch_bound"
)

print_header() {
  printf 'solver\tdataset\trun\twall_s\tpeak_rss_kb\ttour_cost\n'
}

# Run one solver/dataset combo once, emit a TSV row.
run_once() {
  local solver=$1 dataset=$2 run=$3
  local cli_solver="${ALIAS[$solver]:-$solver}"

  # Prefer tests/fixtures/ (tracked) over data/tsplib/ (gitignored).
  local input=""
  for candidate in "tests/fixtures/${dataset}.tsp" "data/tsplib/${dataset}.tsp"; do
    if [[ -f "$candidate" ]]; then
      input="$candidate"
      break
    fi
  done

  if [[ -z "$input" ]]; then
    echo "WARN: ${dataset}.tsp not found in tests/fixtures/ or data/tsplib/, skipping" >&2
    return
  fi

  local time_tmp stdout_tmp
  time_tmp=$(mktemp)
  stdout_tmp=$(mktemp)

  # /usr/bin/time -v writes its report to stderr; solver output to stdout.
  if ! $GNU_TIME -v "$BIN" solve "$cli_solver" -i "$input" \
      >"$stdout_tmp" 2>"$time_tmp"; then
    echo "ERROR: solver $cli_solver on $dataset failed (run $run)" >&2
    cat "$time_tmp" >&2
    rm -f "$time_tmp" "$stdout_tmp"
    return 1
  fi

  # Tour cost = first number on the first line of stdout.
  local tour_cost
  tour_cost=$(head -1 "$stdout_tmp" | awk '{print $1}')
  tour_cost="${tour_cost:-NaN}"

  # Parse GNU time output. Wall time may be "0:01.23" (m:s) or "1.23" (s).
  local wall_s peak_rss_kb
  wall_s=$(awk '/Elapsed \(wall clock\)/ {
    val = $NF
    n = split(val, parts, ":")
    if (n > 1) { printf "%.4f", parts[1] * 60 + parts[2] }
    else       { printf "%.4f", parts[1] }
  }' "$time_tmp")
  peak_rss_kb=$(awk '/Maximum resident set/ {print $NF}' "$time_tmp")

  rm -f "$time_tmp" "$stdout_tmp"

  printf '%s\t%s\t%d\t%s\t%s\t%s\n' \
    "$solver" "$dataset" "$run" "$wall_s" "${peak_rss_kb:-0}" "$tour_cost"
}

main() {
  if [[ ! -x "$BIN" ]]; then
    echo "ERROR: release binary not found at $BIN" >&2
    echo "Run: cargo build -p teeline-cli --release" >&2
    exit 1
  fi
  if [[ ! -x "$GNU_TIME" ]]; then
    echo "ERROR: GNU time not found at $GNU_TIME (apt install time / brew install gnu-time)" >&2
    exit 1
  fi

  print_header
  for solver in nn lk fourier branch_bound; do
    for dataset in ${DATASETS[$solver]}; do
      for run in $(seq 1 "$RUNS"); do
        run_once "$solver" "$dataset" "$run"
      done
    done
  done
}

OUTPUT_FILE="${1:-}"
if [[ -n "$OUTPUT_FILE" ]]; then
  mkdir -p "$(dirname "$OUTPUT_FILE")"
  main | tee "$OUTPUT_FILE"
else
  main
fi

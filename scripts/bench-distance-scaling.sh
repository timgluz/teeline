#!/usr/bin/env bash
# bench-distance-scaling.sh — run one synthetic size per fresh process and record peak RSS.
#
# Usage:
#   scripts/bench-distance-scaling.sh
#   scripts/bench-distance-scaling.sh --sizes "1000 5000 10000"
#   scripts/bench-distance-scaling.sh --solver sfc -- --distance-backend on-the-fly
#   task bench:distance-scaling
#
# Reads:  scripts/gen-synthetic-tsp.py, the release CLI binary
# Writes: bench/runs/distance-scaling.jsonl (git-ignored, append-only)
#
# One process per size is deliberately mandatory: VmHWM is per process, so batching sizes in
# one run would attribute every earlier allocation to the last row. A matrix size beyond the
# machine's RAM is recorded as `oom`/`not-measured`, never extrapolated.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="${TEELINE_BIN:-./target/release/teeline}"
GNU_TIME="${GNU_TIME:-/usr/bin/time}"
SOLVER="${SOLVER:-sfc}"
OUT="${OUT:-bench/runs/distance-scaling.jsonl}"
SEED="${SEED:-42}"
TIMEOUT_S="${TIMEOUT_S:-900}"
FORCE=0
declare -a SIZES=(1000 5000 10000 25000 50000 100000)
declare -a EXTRA_ARGS=()

usage() {
  sed -n '2,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --sizes) read -r -a SIZES <<<"${2:?}"; shift 2 ;;
    --out) OUT="${2:?}"; shift 2 ;;
    --binary) BIN="${2:?}"; shift 2 ;;
    --solver) SOLVER="${2:?}"; shift 2 ;;
    --seed) SEED="${2:?}"; shift 2 ;;
    --timeout) TIMEOUT_S="${2:?}"; shift 2 ;;
    --force) FORCE=1; shift ;;
    --) shift; EXTRA_ARGS=("$@"); break ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

for tool in python3 jq; do
  command -v "$tool" >/dev/null 2>&1 || { echo "ERROR: $tool is required" >&2; exit 1; }
done
[[ -x "$GNU_TIME" ]] || { echo "ERROR: GNU time not found at $GNU_TIME" >&2; exit 1; }
[[ -x "$BIN" ]] || { echo "ERROR: binary not found: $BIN" >&2; exit 1; }

mkdir -p "$(dirname "$OUT")"

GIT_COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
if git diff-index --quiet HEAD -- 2>/dev/null; then DIRTY=false; else DIRTY=true; fi
TEELINE_VERSION="$(awk '/^version *=/ {gsub(/[" ]/,"",$3); print $3; exit}' Cargo.toml 2>/dev/null || echo unknown)"
RUST_VERSION="$(rustc --version 2>/dev/null | awk '{print $2}' || echo unknown)"
OS_NAME="$(uname -s | tr '[:upper:]' '[:lower:]')"
MEASURED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
case "$BIN" in
  */release/*) BUILD_PROFILE="release" ;;
  */debug/*) BUILD_PROFILE="debug" ;;
  *) BUILD_PROFILE="unknown" ;;
esac
MEASUREMENT="$(jq -cn \
  --arg commit "$GIT_COMMIT" --argjson dirty "$DIRTY" \
  --arg version "$TEELINE_VERSION" --arg rust "$RUST_VERSION" \
  --arg os "$OS_NAME" --arg build "$BUILD_PROFILE" --arg at "$MEASURED_AT" \
  '{git_commit:$commit, git_commit_source:"jsonl", dirty:$dirty, teeline_version:$version,
    measured_at:$at, environment:{os:$os, build:$build, rust_version:$rust}}')"

TMPDIR_BENCH="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_BENCH"' EXIT

printf '==> Distance scaling: solver=%s, %d size(s), out=%s\n' "$SOLVER" "${#SIZES[@]}" "$OUT"
printf '    binary: %s (%s)\n' "$BIN" "$BUILD_PROFILE"
printf '    seed:   %s, timeout: %ss\n' "$SEED" "$TIMEOUT_S"

for n in "${SIZES[@]}"; do
  if [[ "$FORCE" -eq 0 && -f "$OUT" ]] && jq -e --argjson n "$n" 'select(.size == $n)' "$OUT" >/dev/null 2>&1; then
    printf '    %-8s already recorded, skipping\n' "$n"
    continue
  fi

  tsp="$TMPDIR_BENCH/synthetic_${n}.tsp"
  python3 scripts/gen-synthetic-tsp.py --n "$n" --seed "$SEED" >"$tsp"

  tmp_time="$TMPDIR_BENCH/time.$n"
  tmp_out="$TMPDIR_BENCH/out.$n"
  tmp_err="$TMPDIR_BENCH/err.$n"

  start="$(date +%s.%N)"
  set +e
  "$GNU_TIME" -v -o "$tmp_time" \
    timeout -k 5 "${TIMEOUT_S}s" \
    "$BIN" solve "$SOLVER" -i "$tsp" ${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"} \
    >"$tmp_out" 2>"$tmp_err"
  rc=$?
  set -e
  end="$(date +%s.%N)"

  wall="$(awk -v a="$start" -v b="$end" 'BEGIN{printf "%.4f", b-a}')"
  rss="$(awk '/Maximum resident set size/ {print $NF}' "$tmp_time" 2>/dev/null || true)"
  tour="$(head -n 1 "$tmp_out" 2>/dev/null | awk '{print $1}')"
  status="error"
  reason=""

  if [[ "$rc" -eq 0 && "$tour" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
    status="ok"
  elif [[ "$rc" -eq 124 ]]; then
    status="timeout"
    reason="wall-clock budget"
  elif [[ "$rc" -eq 137 ]]; then
    status="oom"
    reason="process killed (out of memory or external kill)"
  elif [[ ! "$rss" =~ ^[0-9]+$ ]]; then
    status="oom"
    reason="GNU time reported no peak RSS"
  else
    reason="rc=$rc $(head -n 1 "$tmp_err")"
  fi

  if [[ "$status" == "ok" ]]; then
    row="$(jq -cn \
      --argjson size "$n" --arg solver "$SOLVER" --arg status "$status" \
      --argjson wall "$wall" --argjson rss "$rss" --argjson tour "$tour" \
      --argjson m "$MEASUREMENT" --args \
      '{size:$size, solver:$solver, status:$status, wall_s:$wall,
        peak_rss_kb:$rss, tour_cost:$tour, args:$ARGS.positional, measurement:$m}' \
      -- "${EXTRA_ARGS[@]}")"
  else
    row="$(jq -cn \
      --argjson size "$n" --arg solver "$SOLVER" --arg status "$status" \
      --argjson wall "$wall" --argjson rss "${rss:-null}" --arg reason "$reason" \
      --argjson m "$MEASUREMENT" --args \
      '{size:$size, solver:$solver, status:$status, wall_s:$wall,
        peak_rss_kb:$rss, reason:$reason, args:$ARGS.positional, measurement:$m}' \
      -- "${EXTRA_ARGS[@]}")"
  fi
  printf '%s\n' "$row" >>"$OUT"
  printf '    %-8s %-8s wall=%ss rss=%sKB%s\n' "$n" "$status" "$wall" "${rss:-?}" \
    "${reason:+ ($reason)}"
done

printf '==> wrote %s\n' "$OUT"

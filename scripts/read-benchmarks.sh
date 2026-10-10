#!/usr/bin/env bash
#
# read-benchmarks.sh — read benchmark data back from the R2 bucket behind
# static.tspsolver.com, for humans and for verification after a publish.
#
# Usage:
#   scripts/read-benchmarks.sh index                    # manifest + freshness
#   scripts/read-benchmarks.sh algorithm lk             # one algorithm shard
#   scripts/read-benchmarks.sh problem berlin52         # one problem leaderboard
#   scripts/read-benchmarks.sh list                     # every key under the prefix
#   scripts/read-benchmarks.sh check                    # verify every shard parses
#   scripts/read-benchmarks.sh raw <key>                # fetch any key verbatim
#
# Reads the public HTTP endpoint by default (no credentials needed). Pass
# --via r2 to read through the Cloudflare API (`cf r2 objects get`) instead,
# which also works for objects not yet behind the custom domain.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BASE_URL="${BENCH_BASE_URL:-https://static.tspsolver.com}"
PREFIX="${BENCH_PREFIX:-bench/v1}"
VIA="http"
CF_R2_BUCKET="${CF_R2_BUCKET:-}"

usage() {
  sed -n '2,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  cat <<'EOF'

Options:
  --base-url <url>   Base URL (default: https://static.tspsolver.com)
  --prefix <key>     R2 key prefix (default: bench/v1)
  --via <http|r2>    Read over HTTP (default) or via the Cloudflare API
  --bucket <name>    R2 bucket name, required for --via r2
  -h, --help         Show this help
EOF
}

ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-url) BASE_URL="${2:?}"; shift 2 ;;
    --prefix) PREFIX="${2:?}"; shift 2 ;;
    --via) VIA="${2:?}"; shift 2 ;;
    --bucket) CF_R2_BUCKET="${2:?}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) ARGS+=("$1"); shift ;;
  esac
done

if [[ ${#ARGS[@]} -eq 0 ]]; then
  usage >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq is required. Install it: apt install jq / brew install jq" >&2
  exit 1
fi

fetch() {
  local key="$1"
  case "$VIA" in
    http)
      curl -fsS --max-time 30 "${BASE_URL%/}/${PREFIX}/${key}"
      ;;
    r2)
      if [[ -z "$CF_R2_BUCKET" ]]; then
        echo "ERROR: --via r2 requires --bucket <name> (or CF_R2_BUCKET)" >&2
        return 2
      fi
      if ! command -v cf >/dev/null 2>&1; then
        echo "ERROR: --via r2 requires the 'cf' CLI" >&2
        return 2
      fi
      cf r2 objects get "${PREFIX}/${key}" --bucket-name "$CF_R2_BUCKET"
      ;;
    *)
      echo "ERROR: --via must be http or r2 (got '$VIA')" >&2
      return 2
      ;;
  esac
}

cmd="${ARGS[0]}"

case "$cmd" in
  index)
    body="$(fetch index.json)"
    echo "$body" | jq -r '
      "schema_version : \(.schema_version)",
      "generated_at   : \(.generated_at)",
      "git_commit     : \(.git_commit)\(if .dirty then " (DIRTY WORKING TREE)" else "" end)",
      # A publish-time commit does not identify the measured binary, so say so
      # rather than letting the reader assume the numbers are reproducible.
      "commit_source  : \(.git_commit_source // "unknown")
                        \(if (.git_commit_source // "") == "publish-time"
                          then "⚠ not the measured binary — TSV carried no provenance"
                          else "" end)",
      "teeline        : \(.teeline_version)",
      "tier           : \(.tier // "-")",
      "solvers        : \(.solvers | length)",
      "problems       : \(.problems | length)",
      "",
      "solvers measured:",
      (.solvers[] | "  \(.id)\t\(.name)\tconfigs=\(.configs | join(","))")
    '
    ;;

  algorithm)
    id="${ARGS[1]:?algorithm requires a solver id, e.g. 'algorithm lk'}"
    body="$(fetch "algorithms/${id}.json")"
    echo "$body" | jq -r '
      ([["\(.solver) — \(.results | length) instance(s), configs: \(.configs | join(", "))"],
        [""],
        ["instance", "config", "runs", "timeouts", "gap% med", "gap% best", "gap% worst", "wall_s med", "rss kb med"]]
       + [.results | to_entries[] as $e |
          $e.value.by_config | to_entries[] |
          [$e.key, .key, (.value.runs | tostring), (.value.timeouts | tostring),
           ((.value.gap_pct.median  // "-") | tostring),
           ((.value.gap_pct.best    // "-") | tostring),
           ((.value.gap_pct.worst   // "-") | tostring),
           ((.value.wall_s.median   // "-") | tostring),
           ((.value.peak_rss_kb.median // "-") | tostring)]]
      ) | .[] | @tsv' | column -t -s$'\t'
    ;;

  problem)
    id="${ARGS[1]:?problem requires a dataset id, e.g. 'problem berlin52'}"
    body="$(fetch "problems/${id}.json")"
    echo "$body" | jq -r '
      ([["\(.dataset): \(.cities // "?") cities, optimal=\(.optimal_cost // "unknown")"],
        [""],
        ["#", "solver", "config", "runs", "gap% med", "gap% best", "tour med", "wall_s med"]]
       + [.leaderboard | to_entries[] |
         [(.key + 1 | tostring), .value.solver, .value.config, (.value.runs | tostring),
          ((.value.gap_pct.median // "-") | tostring),
          ((.value.gap_pct.best   // "-") | tostring),
          ((.value.tour_cost.median // "-") | tostring),
          ((.value.wall_s.median  // "-") | tostring)]]
      ) | .[] | @tsv' | column -t -s$'\t'
    ;;

  list)
    if [[ "$VIA" == "r2" ]]; then
      [[ -n "$CF_R2_BUCKET" ]] || { echo "ERROR: --via r2 requires --bucket" >&2; exit 2; }
      cf r2 objects list --bucket-name "$CF_R2_BUCKET" --prefix "$PREFIX/" |
        jq -r '(.result // .objects // .)[]? | (.key // .name)' | sort
    else
      # The bucket serves objects but does not expose a listing over the custom
      # domain, so enumerate from the manifest and probe each key.
      manifest="$(fetch index.json)"
      {
        echo "index.json"
        echo "$manifest" | jq -r '.solvers[] | "algorithms/\(.id).json"'
        echo "$manifest" | jq -r '.problems[] | "problems/\(.id).json"'
      } | while read -r key; do
        if curl -fsS -o /dev/null --max-time 20 "${BASE_URL%/}/${PREFIX}/${key}"; then
          printf 'ok    %s\n' "$key"
        else
          printf 'MISS  %s\n' "$key"
        fi
      done
    fi
    ;;

  check)
    manifest="$(fetch index.json)"
    failures=0
    for key in $(echo "$manifest" | jq -r '.solvers[] | "algorithms/\(.id).json"') \
               $(echo "$manifest" | jq -r '.problems[] | "problems/\(.id).json"'); do
      if body="$(fetch "$key" 2>/dev/null)" && echo "$body" | jq -e '.schema_version' >/dev/null 2>&1; then
        printf 'ok    %s\n' "$key"
      else
        printf 'FAIL  %s\n' "$key"
        failures=$((failures + 1))
      fi
    done
    echo
    echo "manifest schema_version: $(echo "$manifest" | jq -r '.schema_version')"
    echo "generated_at:            $(echo "$manifest" | jq -r '.generated_at')"
    echo "commit:                  $(echo "$manifest" | jq -r '.git_commit')"
    if [[ "$failures" -gt 0 ]]; then
      echo "FAILED: $failures shard(s) missing or unparseable" >&2
      exit 1
    fi
    echo "all shards present and parseable"
    ;;

  raw)
    key="${ARGS[1]:?raw requires a key, e.g. 'raw index.json'}"
    fetch "$key"
    ;;

  *)
    echo "unknown command: $cmd" >&2
    usage >&2
    exit 2
    ;;
esac

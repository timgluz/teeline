#!/usr/bin/env bash
#
# publish-minimaps.sh — upload the generated problem minimaps to the R2 bucket behind
# static.tspsolver.com.
#
# Usage:
#   scripts/gen-problem-minimaps.mjs          # render build/minimaps/*.svg + index.json
#   scripts/publish-minimaps.sh --dry-run     # no credentials, no network writes
#   scripts/publish-minimaps.sh --bucket <name>
#
# Reads:  build/minimaps/index.json and the .svg files it names
# Writes: nothing locally; R2 objects under the prefix
#
# The prefix is versioned. Objects are immutable and served with a long cache lifetime, so a change
# to how a map is drawn (the density threshold, the MDS layout) is a prefix bump, not a cache purge
# — and the pages carry the matching URL in their frontmatter, so the two move together.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# The path is computed at run time, so shellcheck cannot follow it; the file is small and
# self-contained.
# shellcheck disable=SC1091
source "$ROOT/scripts/lib/r2.sh"

PREFIX="tsplib/minimaps/v1"
STAGE="build/minimaps"
PUBLIC_BASE="${PUBLIC_BASE:-https://static.tspsolver.com}"
CF_R2_BUCKET="${CF_R2_BUCKET:-}"
DRY_RUN=0
FETCH_PREVIOUS=1
PREVIOUS_INDEX_FILE=""
ONLY_ALL=0
ONLY_IDS=""
VERIFY=0

usage() {
  cat <<'EOF'
Upload generated problem minimaps to R2.

Usage: scripts/publish-minimaps.sh [options]

Options:
  --bucket <name>       R2 bucket name (default: $CF_R2_BUCKET)
  --prefix <key>        R2 key prefix (default: tsplib/minimaps/v1)
  --public-base <url>   Base URL (default: https://static.tspsolver.com)
  --stage <dir>         Staging dir holding index.json (default: build/minimaps)
  --only <ids>          Comma-separated instance ids to publish
  --all                 Upload every object, not only the changed ones
  --previous <file>     Read the already-published index from a file instead of
                        fetching it (used by the tests; implies --first-publish is off)
  --first-publish       Do not look for a previously published index
  --dry-run             Print what would be uploaded; needs no credentials
  --verify              Read every expected object back over the public URL and
                        report the ones that are not there (no upload, no creds)
  -h, --help            This message
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bucket) CF_R2_BUCKET="${2:?}"; shift 2 ;;
    --prefix) PREFIX="${2:?}"; shift 2 ;;
    --public-base) PUBLIC_BASE="${2:?}"; shift 2 ;;
    --stage) STAGE="${2:?}"; shift 2 ;;
    --only) ONLY_IDS="${2:?}"; shift 2 ;;
    --previous) PREVIOUS_INDEX_FILE="${2:?}"; FETCH_PREVIOUS=0; shift 2 ;;
    --all) ONLY_ALL=1; shift ;;
    --first-publish) FETCH_PREVIOUS=0; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --verify) VERIFY=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ "$DRY_RUN" -eq 0 && "$VERIFY" -eq 0 && -z "$CF_R2_BUCKET" ]]; then
  echo "ERROR: CF_R2_BUCKET is not set (or pass --bucket <name>)." >&2
  echo "Find it with: cf r2 buckets list" >&2
  exit 1
fi

command -v jq >/dev/null 2>&1 || { echo "ERROR: jq is required." >&2; exit 1; }

INDEX="$STAGE/index.json"
if [[ ! -f "$INDEX" ]]; then
  echo "ERROR: $INDEX not found." >&2
  echo "Generate it first: node scripts/gen-problem-minimaps.mjs" >&2
  exit 1
fi

if [[ "$VERIFY" -eq 1 ]]; then
  # Read back what the bucket serves, not what we believe we wrote. Probing the public URL also
  # covers the CDN and the custom domain, which is what a page actually fetches.
  keys="$(jq -r '.entries | keys[] | . + ".svg"' "$INDEX"; echo index.json)"
  missing=0
  total=0
  while IFS= read -r key; do
    [[ -n "$key" ]] || continue
    total=$((total + 1))
    code="$(curl -sS -o /dev/null -w '%{http_code}' --head --max-time 30 "$PUBLIC_BASE/$PREFIX/$key" || true)"
    if [[ "$code" != "200" ]]; then
      echo "    MISSING $PUBLIC_BASE/$PREFIX/$key (HTTP ${code:-000})"
      missing=$((missing + 1))
    fi
  done <<< "$keys"

  if [[ "$missing" -gt 0 ]]; then
    echo "==> $missing of $total object(s) are not published under $PUBLIC_BASE/$PREFIX" >&2
    exit 1
  fi
  echo "==> all $total object(s) are published under $PUBLIC_BASE/$PREFIX"
  exit 0
fi

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

# ---------------------------------------------------------------------------
# What to upload
#
# The local index is the source of truth for what exists; the published index is what the bucket
# already has. Comparing hashes is what makes a re-publish touch only the maps whose bytes changed,
# which matters because the objects are immutable and cached hard.
# ---------------------------------------------------------------------------

if [[ "$ONLY_ALL" -eq 1 ]]; then
  CHANGED="$(jq -r '.entries | keys[]' "$INDEX")"
  REASON="--all"
else
  PREVIOUS_INDEX="$PREVIOUS_INDEX_FILE"
  STATUS=""
  if [[ -n "$PREVIOUS_INDEX_FILE" ]]; then
    # A caller-supplied index is read as-is: this is how the comparison is exercised without a
    # bucket, and how a re-publish is reproduced from a saved index.
    [[ -f "$PREVIOUS_INDEX_FILE" ]] || { echo "ERROR: --previous $PREVIOUS_INDEX_FILE not found." >&2; exit 1; }
    STATUS=200
  elif [[ "$FETCH_PREVIOUS" -eq 1 ]]; then
    PREVIOUS_INDEX="build/minimaps-published-index.json"
    mkdir -p "$(dirname "$PREVIOUS_INDEX")"
    STATUS="$(curl -sS -o "$PREVIOUS_INDEX" -w '%{http_code}' --max-time 30 \
      "$PUBLIC_BASE/$PREFIX/index.json" || true)"
    [[ -n "$STATUS" ]] || STATUS="000"
  fi

  case "$STATUS" in
    200)
      CHANGED="$(jq -r --slurpfile prev "$PREVIOUS_INDEX" '
        .entries | to_entries[]
        | select(($prev[0].entries[.key].sha256 // "") != .value.sha256)
        | .key' "$INDEX")"
      REASON="changed since the published index"
      ;;
    404)
      CHANGED="$(jq -r '.entries | keys[]' "$INDEX")"
      REASON="first publish (no index at $PUBLIC_BASE/$PREFIX)"
      ;;
    *)
      if [[ "$FETCH_PREVIOUS" -eq 1 ]]; then
        echo "ERROR: could not read the published index (HTTP $STATUS from $PUBLIC_BASE/$PREFIX/index.json)." >&2
        echo "Use --first-publish to publish everything anyway, or --all to skip the comparison." >&2
        exit 1
      fi
      CHANGED="$(jq -r '.entries | keys[]' "$INDEX")"
      REASON="--first-publish"
      ;;
  esac
fi

if [[ -n "$ONLY_IDS" ]]; then
  SELECTED="$(echo "$ONLY_IDS" | tr ',' '\n')"
  CHANGED="$(comm -12 <(echo "$CHANGED" | sort -u) <(echo "$SELECTED" | sort -u))"
fi

TOTAL="$(jq -r '.entries | length' "$INDEX")"
CHANGED_COUNT="$(echo "$CHANGED" | grep -c . || true)"
UNCHANGED=$((TOTAL - CHANGED_COUNT))

echo "==> $TOTAL minimap(s) staged; $CHANGED_COUNT to upload ($REASON), $UNCHANGED unchanged"
if [[ "$DRY_RUN" -eq 0 ]]; then
  r2_require_client
  echo "==> Uploading to R2 bucket '$CF_R2_BUCKET' prefix '$PREFIX' via $R2_CLIENT"
else
  echo "==> Dry run: nothing will be uploaded"
fi

# Objects first, index last: a consumer that reads the index must never be pointed at a file that
# has not been written yet.
UPLOADED=0
while IFS= read -r id; do
  [[ -n "$id" ]] || continue
  file="$STAGE/$id.svg"
  if [[ ! -f "$file" ]]; then
    echo "ERROR: $file is named by the index but missing." >&2
    exit 1
  fi
  # The index is hashes; a file that no longer matches it means the staging dir was regenerated
  # part-way or edited by hand, and publishing it would put content behind a URL the index describes.
  expected="$(jq -r --arg id "$id" '.entries[$id].sha256' "$INDEX")"
  actual="$(sha256_of "$file")"
  if [[ "$expected" != "$actual" ]]; then
    echo "ERROR: $file does not match its index hash — re-run: node scripts/gen-problem-minimaps.mjs" >&2
    exit 1
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "    [dry-run] would put ${PREFIX}/${id}.svg (image/svg+xml)"
  else
    r2_put "$CF_R2_BUCKET" "$PREFIX" "${id}.svg" "$file" "image/svg+xml" >/dev/null
    echo "    put ${PREFIX}/${id}.svg"
  fi
  UPLOADED=$((UPLOADED + 1))
done <<< "$CHANGED"

if [[ "$CHANGED_COUNT" -gt 0 ]]; then
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "    [dry-run] would put ${PREFIX}/index.json (application/json)"
  else
    r2_put "$CF_R2_BUCKET" "$PREFIX" "index.json" "$INDEX" "application/json" >/dev/null
    echo "    put ${PREFIX}/index.json"
  fi
  UPLOADED=$((UPLOADED + 1))
fi

echo "==> $UPLOADED object(s) $([[ "$DRY_RUN" -eq 1 ]] && echo 'would be ' || echo '')uploaded, $UNCHANGED unchanged"

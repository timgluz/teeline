#!/usr/bin/env bash
#
# publish-minimaps.sh — upload the generated problem minimaps to the R2 bucket behind
# static.tspsolver.com.
#
# Usage:
#   scripts/gen-problem-minimaps.mjs          # render build/minimaps/*.svg + index.json
#   npm --prefix teeline-web run og:images    # render build/minimaps/og/*.png (Playwright)
#                                             # skipping this publishes pages whose card 404s, so
#                                             # the publisher refuses it when the docs link one
#   scripts/publish-minimaps.sh --dry-run     # no credentials, no network writes
#   scripts/publish-minimaps.sh --bucket <name>
#
# Reads:  build/minimaps/index.json and the objects it names (.svg, og/*.png), plus
#         docs/problems/*.md to learn which pages link a card
# Writes: nothing locally; R2 objects under the prefix
#
# The prefix is versioned, and that version is what makes the objects safe to cache for as long as
# Cloudflare will: a change to how a map is drawn (the density threshold, the MDS layout) is a
# prefix bump, not a cache purge, and the pages carry the matching URL in their frontmatter, so the
# two move together. It sits at the bucket root rather than under `tsplib/`, so a listing of the
# datasets never returns images and vice versa.
#
# No Cache-Control header is set: `cf r2 objects put` has no flag for one, and setting it only on
# the wrangler path would make the same publish cache differently depending on which client a
# machine has. A Cache Rule on the bucket can set the lifetime.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# The path is computed at run time, so shellcheck cannot follow it; the file is small and
# self-contained.
# shellcheck disable=SC1091
source "$ROOT/scripts/lib/r2.sh"

PREFIX="minimaps/v1"
STAGE="build/minimaps"
PUBLIC_BASE="${PUBLIC_BASE:-https://static.tspsolver.com}"
# Where the pages are. A directory that does not exist skips the card check, which is how the tests
# stage a bare index.
DOCS_DIR="${DOCS_DIR:-$ROOT/docs/problems}"
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
  --prefix <key>        R2 key prefix (default: minimaps/v1)
  --public-base <url>   Base URL (default: https://static.tspsolver.com)
  --stage <dir>         Staging dir holding index.json (default: build/minimaps)
  --only <ids>          Comma-separated instance ids; publishes both that instance's
                        map and its social card
  --all                 Upload every object, not only the changed ones
  --previous <file>     Read the already-published index from a file instead of
                        fetching it (used by the tests; implies --first-publish is off)
  --first-publish       Do not look for a previously published index
  --dry-run             Print what would be uploaded; needs no credentials
  --verify              Read every expected object back over the public URL and
                        report the ones that are not there (no upload, no creds)
  -h, --help            This message

Environment:
  DOCS_DIR              Where the pages are, for the card check below (default:
                        docs/problems). A directory that does not exist skips the
                        check, which is what lets the tests stage a bare index.
  CF_R2_BUCKET          R2 bucket name behind static.tspsolver.com

A map is `<id>.svg`, a card is `og/<id>.png`. The unit of upload is the object, so a
changed card never drags an unchanged map with it.

The card check covers every page in DOCS_DIR, not only the ids --only selects: a publish
is refused while any page links a card the index does not have.
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

# Every object the index describes, one key per line. The unit of upload is the object: a changed
# social card must not drag an unchanged map along with it. Defined here because --verify and the
# upload list must agree on what exists; two copies could drift into verifying a subset.
ALL_KEYS_JQ='.entries | to_entries[] | .key + ".svg", (select(.value.og != null) | "og/" + .key + ".png")'

# A page links `og/<id>.png` in its frontmatter and cannot check that it exists — the cards are
# rendered after the pages are written. Without this, skipping the card step publishes 111 pages
# pointing at 404s, and --verify passes because it enumerates the index rather than the pages. The
# docs are the only declaration of what the pages link.
assert_cards_exist() {
  [[ -d "$DOCS_DIR" ]] || return 0
  local with_og
  with_og="$(jq -r '.entries | to_entries[] | select(.value.og.sha256 != null) | .key' "$INDEX")" || {
    echo "ERROR: $INDEX is not a readable index — re-run scripts/gen-problem-minimaps.mjs." >&2
    exit 1
  }
  local f id missing=""
  for f in "$DOCS_DIR"/*.md; do
    [[ -e "$f" ]] || continue
    grep -q '^ogImageUrl:' "$f" || continue
    id="$(basename "$f" .md)"
    grep -qxF "$id" <<<"$with_og" || missing="$missing $id"
  done
  [[ -z "$missing" ]] && return 0
  echo "ERROR: these pages link an Open Graph card the index has no 'og' entry for:$missing" >&2
  echo "Render them first: npm --prefix teeline-web run og:images (needs 'npx playwright install chromium')" >&2
  exit 1
}

assert_cards_exist

if [[ "$VERIFY" -eq 1 ]]; then
  # Read back what the bucket serves, not what we believe we wrote. Probing the public URL also
  # covers the CDN and the custom domain, which is what a page actually fetches.
  keys="$(
    jq -r "$ALL_KEYS_JQ" "$INDEX"
    echo index.json
  )"
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
  CHANGED_KEYS="$(jq -r "$ALL_KEYS_JQ" "$INDEX")"
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
      CHANGED_KEYS="$(jq -r --slurpfile prev "$PREVIOUS_INDEX" '
        .entries | to_entries[] as $e
        | (if ($prev[0].entries[$e.key].sha256 // "") != $e.value.sha256
           then $e.key + ".svg" else empty end),
          (if ($e.value.og != null)
              and (($prev[0].entries[$e.key].og.sha256 // "") != $e.value.og.sha256)
           then "og/" + $e.key + ".png" else empty end)
      ' "$INDEX")"
      REASON="changed since the published index"
      ;;
    404)
      CHANGED_KEYS="$(jq -r "$ALL_KEYS_JQ" "$INDEX")"
      REASON="first publish (no index at $PUBLIC_BASE/$PREFIX)"
      ;;
    *)
      if [[ "$FETCH_PREVIOUS" -eq 1 ]]; then
        echo "ERROR: could not read the published index (HTTP $STATUS from $PUBLIC_BASE/$PREFIX/index.json)." >&2
        echo "Use --first-publish to publish everything anyway, or --all to skip the comparison." >&2
        exit 1
      fi
      CHANGED_KEYS="$(jq -r "$ALL_KEYS_JQ" "$INDEX")"
      REASON="--first-publish"
      ;;
  esac
fi

if [[ -n "$ONLY_IDS" ]]; then
  CHANGED_KEYS="$(echo "$CHANGED_KEYS" | awk -v only="$ONLY_IDS" '
    BEGIN { n = split(only, ids, ","); for (i = 1; i <= n; i++) want[ids[i]] = 1 }
    { id = $0; sub(/^og\//, "", id); sub(/\.(svg|png)$/, "", id); if (want[id]) print }')"
fi

TOTAL="$(jq -r '.entries | to_entries | map(1 + (if .value.og != null then 1 else 0 end)) | add // 0' "$INDEX")"
CHANGED_COUNT="$(echo "$CHANGED_KEYS" | grep -c . || true)"
UNCHANGED=$((TOTAL - CHANGED_COUNT))

echo "==> $TOTAL object(s) staged; $CHANGED_COUNT to upload ($REASON), $UNCHANGED unchanged"
if [[ "$DRY_RUN" -eq 0 ]]; then
  r2_require_client
  echo "==> Uploading to R2 bucket '$CF_R2_BUCKET' prefix '$PREFIX' via $R2_CLIENT"
else
  echo "==> Dry run: nothing will be uploaded"
fi

# Objects first, index last: a consumer that reads the index must never be pointed at a file that
# has not been written yet.
UPLOADED=0
while IFS= read -r key; do
  [[ -n "$key" ]] || continue
  case "$key" in
    *.svg)
      id="${key%.svg}"
      content_type="image/svg+xml"
      # `$id` is jq's variable, bound with --arg where this filter is used.
      # shellcheck disable=SC2016
      hash_filter='.entries[$id].sha256'
      ;;
    og/*.png)
      id="${key#og/}"
      id="${id%.png}"
      content_type="image/png"
      # shellcheck disable=SC2016
      hash_filter='.entries[$id].og.sha256'
      ;;
    *)
      echo "ERROR: unexpected object key '$key' — the index describes .svg and og/*.png only." >&2
      exit 1
      ;;
  esac
  file="$STAGE/$key"

  if [[ ! -f "$file" ]]; then
    echo "ERROR: $file is named by the index but missing." >&2
    exit 1
  fi
  # The index is hashes; a file that no longer matches it means the staging dir was regenerated
  # part-way or edited by hand, and publishing it would put content behind a URL the index
  # describes. The SVG is written by gen-problem-minimaps.mjs and the PNG by gen-og-images.mjs.
  expected="$(jq -r --arg id "$id" "$hash_filter" "$INDEX")"
  actual="$(sha256_of "$file")"
  if [[ "$expected" != "$actual" ]]; then
    echo "ERROR: $file does not match its index hash — re-run the generator that wrote it." >&2
    exit 1
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "    [dry-run] would put ${PREFIX}/${key} (${content_type})"
  else
    r2_put "$CF_R2_BUCKET" "$PREFIX" "$key" "$file" "$content_type" >/dev/null
    echo "    put ${PREFIX}/${key}"
  fi
  UPLOADED=$((UPLOADED + 1))
done <<< "$CHANGED_KEYS"

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

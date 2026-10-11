#!/usr/bin/env bash
# The R2 client, shared by every script that publishes objects.
#
# `cf` (the agentic Cloudflare CLI) is always preferred, per this repo's Cloudflare guidance and an
# explicit preference for `cf` over `wrangler`. `wrangler` is only a fallback for machines that
# have no `cf` binary at all (for example an older CI image), since `cf` is still an open beta. Both
# take the object body as a file.
#
# The content type is a parameter rather than a constant: benchmark shards are JSON and minimaps
# are SVG, and hardcoding either is how one of them ends up served as the wrong thing.

# Sets R2_CLIENT to cf, wrangler, or "" when neither is available.
r2_select_client() {
  R2_CLIENT=""
  if command -v cf >/dev/null 2>&1; then
    R2_CLIENT="cf"
  elif command -v npx >/dev/null 2>&1 && npx --no-install wrangler --version >/dev/null 2>&1; then
    R2_CLIENT="wrangler"
  fi
}

r2_require_client() {
  r2_select_client
  if [[ -z "$R2_CLIENT" ]]; then
    echo "ERROR: neither 'cf' nor 'wrangler' is available to upload to R2." >&2
    echo "Install one: npm i -g cf   (or)   npm i -D wrangler" >&2
    return 1
  fi
}

# r2_put <bucket> <prefix> <key> <file> <content-type>
r2_put() {
  local bucket="$1" prefix="$2" key="$3" file="$4" content_type="$5"
  case "$R2_CLIENT" in
    cf)
      cf r2 objects put "${prefix}/${key}" \
        --bucket-name "$bucket" \
        --file "$file" \
        --content-type "$content_type"
      ;;
    wrangler)
      npx --no-install wrangler r2 object put "${bucket}/${prefix}/${key}" \
        --file "$file" \
        --content-type "$content_type"
      ;;
    *)
      echo "ERROR: no R2 client selected; call r2_require_client first." >&2
      return 1
      ;;
  esac
}

#!/usr/bin/env bats
# End-to-end tests for scripts/publish-minimaps.sh.
#
# Run: ./tests/bats/bin/bats tests/e2e/publish-minimaps.bats
#
# Everything runs offline against a fixture staging dir and a `--previous` index, so no R2
# credentials and no bucket are needed. The properties under test are the ones that decide what
# ends up in an immutable, hard-cached bucket: which objects are uploaded, what they are served as,
# and that nothing is uploaded when nothing changed.

setup() {
    REPO_ROOT="$( cd "$( dirname "$BATS_TEST_FILENAME" )/../.." >/dev/null 2>&1 && pwd )"
    PUBLISH="$REPO_ROOT/scripts/publish-minimaps.sh"
    TMPDIR="$(mktemp -d)"
    cd "$REPO_ROOT"

    STAGE="$TMPDIR/stage"
    FAKE_BIN="$TMPDIR/bin"
    # Same tools, no `cf`, for the fallback case: the client selection must be exercised by what is
    # on PATH, so a PATH that still contains the fake `cf` would test the wrong branch.
    FAKE_BIN_NO_CF="$TMPDIR/bin-no-cf"
    mkdir -p "$STAGE" "$FAKE_BIN" "$FAKE_BIN_NO_CF"
    export CF_LOG="$TMPDIR/cf.log"
    : > "$CF_LOG"

    # A fake `cf` that records its argv. `$*` rather than `$@` so the assertion reads one line.
    cat > "$FAKE_BIN/cf" <<'FAKE'
#!/usr/bin/env bash
echo "cf $*" >> "$CF_LOG"
FAKE
    chmod +x "$FAKE_BIN/cf"

    # A fake `npx`: answers the version probe the client selection uses, and records the wrangler
    # upload. Anything else fails loudly rather than silently succeeding.
    cat > "$FAKE_BIN/npx" <<'FAKE'
#!/usr/bin/env bash
if [ "${1:-}" = "--no-install" ] && [ "${3:-}" = "--version" ]; then
    exit 0
fi
echo "npx $*" >> "$CF_LOG"
FAKE
    chmod +x "$FAKE_BIN/npx"
    cp "$FAKE_BIN/npx" "$FAKE_BIN_NO_CF/npx"
}

teardown() {
    # A fixture server left behind by a failing test would hold its port for the next run.
    if [ -f "$TMPDIR/port" ]; then
        pkill -f "http.server $(cat "$TMPDIR/port")" 2>/dev/null || true
    fi
    rm -rf "$TMPDIR"
}

# stage <dir> <id>... — SVG files plus an index whose hashes match them.
stage() {
    local dir="$1"; shift
    mkdir -p "$dir"
    jq -n '{generated_at:"2026-01-01T00:00:00Z", entries:{}}' > "$dir/index.json"
    local id hash bytes
    for id in "$@"; do
        printf '<svg xmlns="http://www.w3.org/2000/svg"><circle cx="1" cy="1" r="1"/></svg>\n' > "$dir/$id.svg"
        hash="$(sha256sum "$dir/$id.svg" | awk '{print $1}')"
        bytes="$(wc -c < "$dir/$id.svg" | tr -d ' ')"
        jq --arg id "$id" --arg h "$hash" --argjson b "$bytes" \
            '.entries[$id] = {sha256:$h, bytes:$b, derived:false, cities:3}' \
            "$dir/index.json" > "$dir/index.next" && mv "$dir/index.next" "$dir/index.json"
    done
}

# stage_og <dir> <id>... — the same, plus the 1200x630 raster and its index hash.
stage_og() {
    local dir="$1"; shift
    stage "$dir" "$@"
    mkdir -p "$dir/og"
    local id hash bytes
    for id in "$@"; do
        printf '\x89PNG\r\n\x1a\n' > "$dir/og/$id.png"
        hash="$(sha256sum "$dir/og/$id.png" | awk '{print $1}')"
        bytes="$(wc -c < "$dir/og/$id.png" | tr -d ' ')"
        jq --arg id "$id" --arg h "$hash" --argjson b "$bytes" \
            '.entries[$id].og = {sha256:$h, bytes:$b}' \
            "$dir/index.json" > "$dir/index.next" && mv "$dir/index.next" "$dir/index.json"
    done
}

@test "the social card is uploaded as a PNG beside its map" {
    stage_og "$STAGE" alpha beta

    run "$PUBLISH" --dry-run --first-publish --stage "$STAGE"
    [ "$status" -eq 0 ]
    [[ "$output" == *"would put minimaps/v1/alpha.svg (image/svg+xml)"* ]]
    [[ "$output" == *"would put minimaps/v1/og/alpha.png (image/png)"* ]]
    [[ "$output" == *"5 object(s) would be uploaded"* ]]
}

@test "a changed raster alone is enough to re-upload an instance" {
    stage_og "$STAGE" alpha beta
    cp "$STAGE/index.json" "$TMPDIR/published.json"
    # Same SVG bytes, different card: only the PNG (and the index) should move.
    jq '.entries.alpha.og.sha256 = "0000000000000000000000000000000000000000000000000000000000000000"' \
        "$TMPDIR/published.json" > "$TMPDIR/stale-og.json"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/stale-og.json" --stage "$STAGE"
    [ "$status" -eq 0 ]
    [[ "$output" == *"would put minimaps/v1/og/alpha.png"* ]]
    [[ "$output" != *"would put minimaps/v1/alpha.svg"* ]]
    [[ "$output" != *"og/beta.png"* ]]
    # 4 objects staged (2 maps + 2 cards); one card changed, so one object plus the index move.
    [[ "$output" == *"2 object(s) would be uploaded, 3 unchanged"* ]]
}

@test "first publish lists every staged minimap with its content type" {
    stage "$STAGE" alpha beta

    run "$PUBLISH" --dry-run --first-publish --stage "$STAGE"
    [ "$status" -eq 0 ]
    [[ "$output" == *"would put minimaps/v1/alpha.svg (image/svg+xml)"* ]]
    [[ "$output" == *"would put minimaps/v1/beta.svg (image/svg+xml)"* ]]
    [[ "$output" == *"would put minimaps/v1/index.json (application/json)"* ]]
    [[ "$output" == *"3 object(s) would be uploaded"* ]]
}

@test "nothing is uploaded when the published index already matches" {
    stage "$STAGE" alpha beta
    cp "$STAGE/index.json" "$TMPDIR/published.json"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/published.json" --stage "$STAGE"
    [ "$status" -eq 0 ]
    [[ "$output" != *"would put"* ]]
    [[ "$output" == *"0 object(s) would be uploaded, 2 unchanged"* ]]
}

@test "only the minimaps whose bytes changed are uploaded" {
    stage "$STAGE" alpha beta
    cp "$STAGE/index.json" "$TMPDIR/published.json"
    jq '.entries.alpha.sha256 = "0000000000000000000000000000000000000000000000000000000000000000"' \
        "$TMPDIR/published.json" > "$TMPDIR/stale.json"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/stale.json" --stage "$STAGE"
    [ "$status" -eq 0 ]
    [[ "$output" == *"would put minimaps/v1/alpha.svg"* ]]
    [[ "$output" != *"would put minimaps/v1/beta.svg"* ]]
    # The index always follows the objects it describes.
    [[ "$output" == *"would put minimaps/v1/index.json"* ]]
    [[ "$output" == *"2 object(s) would be uploaded, 1 unchanged"* ]]
}

@test "--only publishes just the named instances" {
    stage "$STAGE" alpha beta

    run "$PUBLISH" --dry-run --first-publish --only beta --stage "$STAGE"
    [ "$status" -eq 0 ]
    [[ "$output" != *"would put minimaps/v1/alpha.svg"* ]]
    [[ "$output" == *"would put minimaps/v1/beta.svg"* ]]
}

@test "an unreadable published index fails instead of guessing" {
    stage "$STAGE" alpha

    run "$PUBLISH" --dry-run --public-base "http://127.0.0.1:9/nothing" --stage "$STAGE"
    [ "$status" -eq 1 ]
    [[ "$output" == *"could not read the published index"* ]]
}

@test "a staging dir with no index says how to make one" {
    run "$PUBLISH" --dry-run --first-publish --stage "$TMPDIR/absent"
    [ "$status" -eq 1 ]
    [[ "$output" == *"gen-problem-minimaps.mjs"* ]]
}

@test "an SVG that does not match its index hash is refused" {
    stage "$STAGE" alpha
    printf 'tampered\n' >> "$STAGE/alpha.svg"

    run "$PUBLISH" --dry-run --first-publish --stage "$STAGE"
    [ "$status" -eq 1 ]
    [[ "$output" == *"does not match its index hash"* ]]
}

@test "a real upload needs a bucket" {
    stage "$STAGE" alpha

    run env -u CF_R2_BUCKET "$PUBLISH" --first-publish --stage "$STAGE"
    [ "$status" -eq 1 ]
    [[ "$output" == *"CF_R2_BUCKET is not set"* ]]
}

@test "cf is preferred and carries the bucket, key and content type" {
    stage "$STAGE" alpha

    PATH="$FAKE_BIN:/usr/bin:/bin" run "$PUBLISH" --bucket test-bucket --first-publish --stage "$STAGE"
    [ "$status" -eq 0 ]
    grep -q "cf r2 objects put minimaps/v1/alpha.svg --bucket-name test-bucket --file $STAGE/alpha.svg --content-type image/svg+xml" "$CF_LOG"
    grep -q "cf r2 objects put minimaps/v1/index.json --bucket-name test-bucket --file $STAGE/index.json --content-type application/json" "$CF_LOG"
}

@test "wrangler is the fallback when cf is absent" {
    stage "$STAGE" alpha
    # No `cf` on this PATH, but the fake `npx` satisfies the wrangler probe.
    PATH="$FAKE_BIN_NO_CF:/usr/bin:/bin" run env -u CF_R2_BUCKET "$PUBLISH" --bucket test-bucket --first-publish --stage "$STAGE"
    [ "$status" -eq 0 ]
    [[ "$output" == *"put minimaps/v1/alpha.svg"* ]]
    grep -q "npx --no-install wrangler r2 object put test-bucket/minimaps/v1/alpha.svg --file $STAGE/alpha.svg --content-type image/svg+xml" "$CF_LOG"
}

@test "--verify reports objects the bucket is not serving" {
    stage "$STAGE" alpha

    run "$PUBLISH" --verify --public-base "http://127.0.0.1:9/nothing" --stage "$STAGE"
    [ "$status" -eq 1 ]
    [[ "$output" == *"MISSING http://127.0.0.1:9/nothing/minimaps/v1/alpha.svg"* ]]
    [[ "$output" == *"are not published"* ]]
}

@test "--verify passes when every object is served at its prefix" {
    stage_og "$STAGE" alpha
    # Serve the staging dir at the prefix path, the layout a published object ends up under.
    local root="$TMPDIR/served"
    mkdir -p "$root/minimaps"
    ln -s "$STAGE" "$root/minimaps/v1"

    local port=$((20000 + $$ % 20000))
    echo "$port" > "$TMPDIR/port"
    python3 -m http.server "$port" --directory "$root" >/dev/null 2>&1 &

    local ready=0
    for _ in $(seq 1 50); do
        if curl -sf -o /dev/null "http://127.0.0.1:$port/minimaps/v1/index.json"; then
            ready=1
            break
        fi
        sleep 0.1
    done
    if [ "$ready" -ne 1 ]; then
        echo "fixture server did not start on port $port" >&2
        return 1
    fi

    run "$PUBLISH" --verify --public-base "http://127.0.0.1:$port" --stage "$STAGE"
    [ "$status" -eq 0 ]
    [[ "$output" == *"all 3 object(s) are published"* ]]
}

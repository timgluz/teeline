#!/usr/bin/env bats
# End-to-end tests for scripts/publish-benchmarks.sh — the published JSON contract.
#
# Run: ./tests/bats/bin/bats tests/e2e/publish-benchmarks.bats
#
# These exercise the transform offline (--dry-run --no-fetch-previous), so they need no R2
# credentials and no network. The property under test is provenance: a published number must be
# attributable to the commit that produced it, per solver, and a re-run of one solver must not
# disturb any other solver's claim.

setup() {
    REPO_ROOT="$( cd "$( dirname "$BATS_TEST_FILENAME" )/../.." >/dev/null 2>&1 && pwd )"
    PUBLISH="$REPO_ROOT/scripts/publish-benchmarks.sh"
    TMPDIR="$(mktemp -d)"
    # The transform resolves docs/problems/*.md and bench/solvers.json relative to the cwd.
    cd "$REPO_ROOT"

    # The default per-row provenance. Rows carry it individually, so a fixture can vary it
    # per row — which is the whole reason the raw format is JSONL: a sweep that merges new
    # rows into an existing file would otherwise put rows from two builds under one header.
    DEFAULT_MEASUREMENT='{"git_commit":"aaaaaaaa","git_commit_source":"jsonl","dirty":false,"teeline_version":"1.0.0","measured_at":"2026-01-02T03:04:05Z","environment":{"os":"linux","build":"release","rust_version":"1.90.0"}}'

    # A second build's provenance, used by the differential fixtures.
    ALT_MEASUREMENT='{"git_commit":"bbbbbbbb","git_commit_source":"jsonl","dirty":false,"teeline_version":"1.0.1","measured_at":"2026-01-02T03:04:05Z","environment":{"os":"linux","build":"release","rust_version":"1.90.0"}}'

    # row SOLVER DATASET CONFIG RUN WALL RSS TOUR STATUS [MEASUREMENT_JSON|-]
    #   '-' emits the row with no measurement at all, for the publish-time fallback.
    row() {
        local m="${9:-$DEFAULT_MEASUREMENT}"
        if [ "$m" = "-" ]; then
            jq -cn --arg s "$1" --arg d "$2" --arg c "$3" --argjson r "$4" --argjson w "$5" \
                --argjson k "$6" --argjson t "$7" --arg st "$8" \
                '{solver:$s,dataset:$d,config:$c,run:$r,wall_s:$w,peak_rss_kb:$k,tour_cost:$t,status:$st}'
        else
            jq -cn --arg s "$1" --arg d "$2" --arg c "$3" --argjson r "$4" --argjson w "$5" \
                --argjson k "$6" --argjson t "$7" --arg st "$8" --argjson m "$m" \
                '{solver:$s,dataset:$d,config:$c,run:$r,wall_s:$w,peak_rss_kb:$k,tour_cost:$t,status:$st,measurement:$m}'
        fi
    }

    # Two solvers over two instances: eil51 has a known optimal (426), zzz_unknown has no doc
    # at all and so no known optimal. `nn` is deterministic, `sa` is not.
    # berlin52 is deliberately NOT used: its doc records `optimalCost: null`, so it cannot
    # exercise the known-optimal path.
    {
        row nn eil51 default 1 0.01 6000 450.0 ok
        row nn eil51 default 2 0.01 6000 450.0 ok
        row nn zzz_unknown default 1 0.02 6100 1234.0 ok
        row nn zzz_unknown default 2 0.02 6100 1234.0 ok
        row sa eil51 default 1 0.30 8000 440.0 ok
        row sa eil51 default 2 0.31 8100 430.0 ok
        row sa zzz_unknown default 1 0.40 8200 1200.0 ok
        row sa zzz_unknown default 2 0.41 8200 1210.0 ok
    } > "$TMPDIR/prev.jsonl"

    # A later measurement of `sa` alone, at a different commit — the differential case.
    {
        row sa eil51 default 1 0.20 8000 428.0 ok "$ALT_MEASUREMENT"
        row sa eil51 default 2 0.21 8100 427.0 ok "$ALT_MEASUREMENT"
        row sa zzz_unknown default 1 0.30 8200 1100.0 ok "$ALT_MEASUREMENT"
        row sa zzz_unknown default 2 0.31 8200 1110.0 ok "$ALT_MEASUREMENT"
    } > "$TMPDIR/sa-only.jsonl"

    # Timeouts and an error must be counted, never averaged into the gap.
    {
        row nn eil51 default 1 0.01 6000 450.0 ok
        row nn eil51 default 2 180.0 6000 0.0 timeout
        row nn eil51 default 3 180.0 6000 0.0 timeout
        row nn eil51 default 4 0.02 6000 460.0 ok
        row nn eil51 default 5 0.01 6000 0.0 error
    } > "$TMPDIR/timeouts.jsonl"
}

teardown() {
    rm -rf "$TMPDIR"
}

publish() {
    # --first-publish because these fixtures deliberately publish a partial tree: there is no
    # previous tree to merge with, and the script refuses that unless it is stated explicitly.
    "$PUBLISH" --dry-run --no-fetch-previous --first-publish --stage "$TMPDIR/stage" "$@"
}

@test "algorithms/{id}.json carries a per-solver measurement block" {
    run publish --runs "$TMPDIR/prev.jsonl"
    [ "$status" -eq 0 ]

    local shard="$TMPDIR/stage/v2/algorithms/nn.json"
    [ -f "$shard" ]
    [ "$(jq -r '.measurement.git_commit' "$shard")" = "aaaaaaaa" ]
    [ "$(jq -r '.measurement.git_commit_source' "$shard")" = "jsonl" ]
    [ "$(jq -r '.measurement.dirty' "$shard")" = "false" ]
    [ "$(jq -r '.measurement.teeline_version' "$shard")" = "1.0.0" ]
    # os/build describe the measured binary and come from the row's own measurement.
    [ "$(jq -r '.measurement.environment.os' "$shard")" = "linux" ]
    [ "$(jq -r '.measurement.environment.build' "$shard")" = "release" ]
    [ "$(jq -r '.measurement.environment.rust_version' "$shard")" = "1.90.0" ]
    [ "$(jq -r '.measurement.measured_at' "$shard")" = "2026-01-02T03:04:05Z" ]
}

@test "the environment block describes the measured binary, per row" {
    row nn eil51 default 1 0.01 6000 450.0 ok \
        '{"git_commit":"eeee0001","git_commit_source":"jsonl","dirty":true,"teeline_version":"1.0.1","measured_at":"2026-01-02T03:04:05Z","environment":{"os":"freebsd","build":"debug","rust_version":"1.91.0"}}' \
        > "$TMPDIR/env.jsonl"
    run publish --runs "$TMPDIR/env.jsonl"
    [ "$status" -eq 0 ]
    local shard="$TMPDIR/stage/v2/algorithms/nn.json"
    [ "$(jq -r '.measurement.environment.os' "$shard")" = "freebsd" ]
    [ "$(jq -r '.measurement.environment.build' "$shard")" = "debug" ]
    [ "$(jq -r '.measurement.dirty' "$shard")" = "true" ]
}

@test "environment fields a row omits are null, never the publisher's" {
    row nn eil51 default 1 0.01 6000 450.0 ok \
        '{"git_commit":"ffff0001","git_commit_source":"jsonl"}' > "$TMPDIR/env-bare.jsonl"
    run publish --runs "$TMPDIR/env-bare.jsonl"
    [ "$status" -eq 0 ]
    local shard="$TMPDIR/stage/v2/algorithms/nn.json"
    # Inferring "linux/release" from the publishing machine would attach the publisher's
    # platform to a number measured elsewhere, in a shard carried over unchanged.
    [ "$(jq -r '.measurement.environment.os' "$shard")" = "null" ]
    [ "$(jq -r '.measurement.environment.build' "$shard")" = "null" ]
    [ "$(jq -r '.measurement.environment.rust_version' "$shard")" = "null" ]
    [ "$(jq -r '.measurement.dirty' "$shard")" = "null" ]
}

@test "a non-boolean dirty value is rejected, not read as a clean tree" {
    row nn eil51 default 1 0.01 6000 450.0 ok \
        '{"git_commit":"dddd0001","git_commit_source":"jsonl","dirty":"yes"}' \
        > "$TMPDIR/baddirty.jsonl"
    run publish --runs "$TMPDIR/baddirty.jsonl"
    # `dirty` is a JSON boolean. A string is a malformed row, and accepting it would put a
    # value nobody can interpret into a shard the differential cadence carries over unchanged.
    [ "$status" -ne 0 ]
    [[ "$output" == *"measurement.dirty"* ]]
    [ ! -f "$TMPDIR/stage/v2/index.json" ]
}

@test "an unknown field inside a row's measurement is rejected" {
    row nn eil51 default 1 0.01 6000 450.0 ok \
        '{"git_commit":"dddd0002","git_commit_source":"jsonl","cpu_name":"secret"}' \
        > "$TMPDIR/badfield.jsonl"
    run publish --runs "$TMPDIR/badfield.jsonl"
    [ "$status" -ne 0 ]
    [[ "$output" == *"unknown measurement field"* ]]
}

@test "an unknown status is rejected rather than counted as an error" {
    # A typed field is the point of leaving TSV: "timout" must not quietly become an error,
    # which is what a bare string comparison would have done.
    printf '%s\n' '{"solver":"nn","dataset":"eil51","run":1,"wall_s":1,"peak_rss_kb":1,"tour_cost":1,"status":"timout"}' \
        > "$TMPDIR/badstatus.jsonl"
    run publish --runs "$TMPDIR/badstatus.jsonl"
    [ "$status" -ne 0 ]
    [[ "$output" == *"status must be one of"* ]]
}

@test "a row missing a required numeric field is rejected" {
    # Defaulting tour_cost to 0 would fabricate a -100% gap, i.e. "twice as good as optimal".
    printf '%s\n' '{"solver":"nn","dataset":"eil51","run":1,"wall_s":1,"peak_rss_kb":1}' \
        > "$TMPDIR/short.jsonl"
    run publish --runs "$TMPDIR/short.jsonl"
    [ "$status" -ne 0 ]
    [[ "$output" == *"tour_cost"* ]]
}

@test "a file whose rows for one solver disagree on provenance is refused" {
    # The defect this format change removes. A resumable sweep merges new rows into an
    # existing file, so when the solver is rebuilt between runs the file holds rows from two
    # builds. One shard cannot describe both honestly, so the publish must refuse rather than
    # stamp the older rows with the newer commit.
    row nn eil51 default 1 0.01 6000 450.0 ok > "$TMPDIR/mixed.jsonl"
    row nn eil51 default 2 0.01 6000 450.0 ok "$ALT_MEASUREMENT" >> "$TMPDIR/mixed.jsonl"
    run publish --runs "$TMPDIR/mixed.jsonl"
    [ "$status" -ne 0 ]
    [[ "$output" == *"different provenance"* ]]
    [[ "$output" == *"'nn'"* ]]
    [ ! -f "$TMPDIR/stage/v2/index.json" ]
}

@test "a missing budgets file aborts rather than publishing an empty budget map" {
    run publish --runs "$TMPDIR/prev.jsonl" --budgets "$TMPDIR/nope.json"
    [ "$status" -ne 0 ]
    [[ "$output" == *"budgets file not found"* ]]
    # Publishing {} would overwrite the live budgets, leaving gaps without their allowance.
    [ ! -f "$TMPDIR/stage/v2/index.json" ]
}

@test "index.json makes no dataset-level claim about the measured commit" {
    run publish --runs "$TMPDIR/prev.jsonl"
    [ "$status" -eq 0 ]

    local idx="$TMPDIR/stage/v2/index.json"
    # The whole point: one commit cannot describe solvers measured at different commits.
    for field in git_commit git_commit_source dirty teeline_version environment; do
        [ "$(jq "has(\"$field\")" "$idx")" = "false" ]
    done
    [ "$(jq -r '.schema_version' "$idx")" = "2" ]
}

@test "the URL prefix and schema_version are bumped together" {
    run publish --runs "$TMPDIR/prev.jsonl"
    [ "$status" -eq 0 ]
    # publish is invoked with the default --prefix bench/v2; the transform wrote under v2.
    [ -d "$TMPDIR/stage/v2" ]
    [ "$(jq -r '.schema_version' "$TMPDIR/stage/v2/index.json")" = "2" ]
    [ "$(jq -r '.schema_version' "$TMPDIR/stage/v2/algorithms/nn.json")" = "2" ]
}

@test "per-family budgets are published with their agreed status" {
    run publish --runs "$TMPDIR/prev.jsonl"
    [ "$status" -eq 0 ]

    local idx="$TMPDIR/stage/v2/index.json"
    # meta 300s is the only figure fixed by a decision; the rest are flagged provisional.
    [ "$(jq -r '.budgets.meta.seconds' "$idx")" = "300" ]
    [ "$(jq -r '.budgets.meta.agreed' "$idx")" = "true" ]
    [ "$(jq -r '.budgets.local.agreed' "$idx")" = "false" ]
}

@test "an unknown optimal stays null and yields no gap, never a zero gap" {
    run publish --runs "$TMPDIR/prev.jsonl"
    [ "$status" -eq 0 ]

    local shard="$TMPDIR/stage/v2/algorithms/nn.json"
    [ "$(jq -r '.results.zzz_unknown.optimal_cost' "$shard")" = "null" ]
    # A gap of 0 would read as "reached the optimum", a claim we cannot make here.
    [ "$(jq -r '.results.zzz_unknown.by_config.default | has("gap_pct")' "$shard")" = "false" ]

    # The known-optimal instance does carry a gap.
    [ "$(jq -r '.results.eil51.by_config.default | has("gap_pct")' "$shard")" = "true" ]
}

@test "timeouts and errors are counted separately and kept out of the gap" {
    run publish --runs "$TMPDIR/timeouts.jsonl"
    [ "$status" -eq 0 ]

    local cell
    cell="$(jq -c '.results.eil51.by_config.default' "$TMPDIR/stage/v2/algorithms/nn.json")"
    # Two timeouts and one error: only the two ok runs feed the aggregate.
    [ "$(jq -r '.runs' <<<"$cell")" = "2" ]
    [ "$(jq -r '.timeouts' <<<"$cell")" = "2" ]
    [ "$(jq -r '.errors' <<<"$cell")" = "1" ]
    # A timeout records tour_cost 0.0, which would read as a huge negative gap if it were
    # included. Against eil51's 426 optimum the real gaps are ~5-8%.
    [ "$(jq -r '.gap_pct.worst < 100' <<<"$cell")" = "true" ]
    # Compared numerically: the median/best/worst are rounded floats, so 460.0 != "460".
    [ "$(jq -r '.tour_cost.worst == 460' <<<"$cell")" = "true" ]
}

@test "re-measuring one solver leaves the other solvers' provenance untouched" {
    publish --runs "$TMPDIR/prev.jsonl"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage2" \
        --runs "$TMPDIR/sa-only.jsonl"
    [ "$status" -eq 0 ]

    # `sa` was re-measured: new commit.
    [ "$(jq -r '.measurement.git_commit' "$TMPDIR/stage2/v2/algorithms/sa.json")" = "bbbbbbbb" ]
    # `nn` was not: byte-identical shard, so it still reports the commit that measured it.
    cmp -s "$TMPDIR/prev-tree/v2/algorithms/nn.json" "$TMPDIR/stage2/v2/algorithms/nn.json"
    [ "$(jq -r '.measurement.git_commit' "$TMPDIR/stage2/v2/algorithms/nn.json")" = "aaaaaaaa" ]
}

@test "a differential republish keeps unmeasured solvers on the problem leaderboard" {
    publish --runs "$TMPDIR/prev.jsonl"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    local before after
    before="$(jq -r '[.leaderboard[].solver] | sort | join(",")' "$TMPDIR/prev-tree/v2/problems/eil51.json")"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage2" \
        --runs "$TMPDIR/sa-only.jsonl"
    [ "$status" -eq 0 ]

    after="$(jq -r '[.leaderboard[].solver] | sort | join(",")' "$TMPDIR/stage2/v2/problems/eil51.json")"
    # Replacing the shard from a one-solver TSV would have dropped nn from the ranking.
    [ "$before" = "$after" ]
    [ "$after" = "nn,sa" ]
}

@test "a differential republish uploads only the shard it re-measured" {
    publish --runs "$TMPDIR/prev.jsonl"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage2" \
        --runs "$TMPDIR/sa-only.jsonl"
    [ "$status" -eq 0 ]

    # nn's shard is unchanged, so it must not be re-uploaded; sa's must be.
    [[ "$output" == *"would put bench/v2/algorithms/sa.json"* ]]
    [[ "$output" != *"would put bench/v2/algorithms/nn.json"* ]]
    # index.json is expected to differ (it carries generated_at); the point is nn's shard.
    [ "$(jq -r '.measurement.git_commit' "$TMPDIR/prev-tree/v2/algorithms/nn.json")" = "aaaaaaaa" ]
}

@test "a release snapshot is written as a complete immutable copy" {
    run publish --runs "$TMPDIR/prev.jsonl" --snapshot v1.0.0
    [ "$status" -eq 0 ]

    local snap="$TMPDIR/stage/v2/snapshots/v1.0.0"
    # Complete: the snapshot must stand alone, not just list what changed.
    [ -f "$snap/index.json" ]
    [ -f "$snap/algorithms/nn.json" ]
    [ -f "$snap/algorithms/sa.json" ]
    [ -f "$snap/problems/eil51.json" ]
    # It is the same tree that was published alongside it.
    cmp -s "$TMPDIR/stage/v2/index.json" "$snap/index.json"
}

@test "rows carrying no measurement are labelled publish-time, not implied" {
    # No measurement on the row: the commit can only be HEAD at publish time and does not
    # necessarily identify the binary, so the source must say so.
    row nn eil51 default 1 0.01 6000 450.0 ok - > "$TMPDIR/bare.jsonl"
    run publish --runs "$TMPDIR/bare.jsonl"
    [ "$status" -eq 0 ]

    local shard="$TMPDIR/stage/v2/algorithms/nn.json"
    [ "$(jq -r '.measurement.git_commit_source' "$shard")" = "publish-time" ]
    # `measured_at` cannot be inferred from publishing, so it stays null rather than being
    # stamped with today's date. The version and tree state ARE the publisher's, which is why
    # the block is labelled publish-time: a consumer must not read them as the measured binary.
    [ "$(jq -r '.measurement.measured_at' "$shard")" = "null" ]
    [ "$(jq -r '.measurement.git_commit | length > 0' "$shard")" = "true" ]
}

@test "a measurement without measured_at yields null rather than the publish time" {
    row nn eil51 default 1 0.01 6000 450.0 ok \
        '{"git_commit":"1234567","git_commit_source":"jsonl","dirty":false}' > "$TMPDIR/nostamp.jsonl"
    run publish --runs "$TMPDIR/nostamp.jsonl"
    [ "$status" -eq 0 ]
    # Stamping it with the publish time would misdate a result carried forward from elsewhere.
    [ "$(jq -r '.measurement.measured_at' "$TMPDIR/stage/v2/algorithms/nn.json")" = "null" ]
}

@test "measured_at is published when the row records it" {
    run publish --runs "$TMPDIR/prev.jsonl"
    [ "$status" -eq 0 ]
    [ "$(jq -r '.measurement.measured_at' "$TMPDIR/stage/v2/algorithms/nn.json")" = "2026-01-02T03:04:05Z" ]
}

@test "a partial publish with no previous tree is refused unless stated" {
    # This is the guard that stops a differential run from quietly replacing the live manifest
    # and leaderboards with a tree holding only this run log's solvers.
    run "$PUBLISH" --dry-run --no-fetch-previous --stage "$TMPDIR/nostage" --runs "$TMPDIR/prev.jsonl"
    [ "$status" -ne 0 ]
    [[ "$output" == *"--first-publish"* ]]
    [ ! -f "$TMPDIR/nostage/v2/index.json" ]
}

@test "an invalid snapshot tag is refused before it reaches the filesystem" {
    run publish --runs "$TMPDIR/prev.jsonl" --snapshot "../escape"
    [ "$status" -ne 0 ]
    [[ "$output" == *"invalid snapshot tag"* ]]
    # Nothing outside the snapshots directory may be touched.
    [ ! -d "$TMPDIR/stage/v2/escape" ]
}

@test "a differential republish drops entries for a re-measured solver on other datasets" {
    publish --runs "$TMPDIR/prev.jsonl"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    # Measure sa on eil51 only, so its replacement shard no longer covers zzz_unknown.
    row sa eil51 default 1 0.20 8000 428.0 ok "$ALT_MEASUREMENT" > "$TMPDIR/sa-partial.jsonl"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage3" \
        --runs "$TMPDIR/sa-partial.jsonl"
    [ "$status" -eq 0 ]

    # sa's shard no longer has zzz_unknown, so the leaderboard must not still rank it there.
    [ "$(jq -r '.results | has("zzz_unknown")' "$TMPDIR/stage3/v2/algorithms/sa.json")" = "false" ]
    [ "$(jq -r '[.leaderboard[].solver] | index("sa") == null' "$TMPDIR/stage3/v2/problems/zzz_unknown.json")" = "true" ]
    # nn was not re-measured, so it stays.
    [ "$(jq -r '[.leaderboard[].solver] | index("nn") != null' "$TMPDIR/stage3/v2/problems/zzz_unknown.json")" = "true" ]
}

@test "a partial TSV header does not inherit the publisher's version and rustc" {
    # git_commit present, everything else absent. Pairing that commit with the publisher's
    # dirty flag, version and rustc would be a provenance claim the file does not support —
    # and once written into a shard it is permanent.
    row nn eil51 default 1 0.01 6000 450.0 ok \
        '{"git_commit":"dddddddd","git_commit_source":"jsonl"}' > "$TMPDIR/partial-header.jsonl"
    run publish --runs "$TMPDIR/partial-header.jsonl"
    [ "$status" -eq 0 ]

    local shard="$TMPDIR/stage/v2/algorithms/nn.json"
    [ "$(jq -r '.measurement.git_commit' "$shard")" = "dddddddd" ]
    [ "$(jq -r '.measurement.dirty' "$shard")" = "null" ]
    [ "$(jq -r '.measurement.teeline_version' "$shard")" = "null" ]
    [ "$(jq -r '.measurement.environment.rust_version' "$shard")" = "null" ]
}

@test "a previous manifest naming a shard that is not on disk is refused" {
    publish --runs "$TMPDIR/prev.jsonl"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    # The manifest still lists sa, but its shard is gone. Merging would drop sa from the
    # republished tree without a word, so this must abort instead.
    rm "$TMPDIR/prev-tree/v2/algorithms/sa.json"
    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage-missing" \
        --runs "$TMPDIR/sa-only.jsonl"
    [ "$status" -ne 0 ]
    [[ "$output" == *"not present in the tree"* ]]
}

@test "a shard the previous manifest does not name is not merged" {
    publish --runs "$TMPDIR/prev.jsonl"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    # A stray shard was never live: resurrecting it would republish something no reader
    # could have seen, including one deliberately withdrawn.
    jq '.solver = "zzz_stray"' "$TMPDIR/prev-tree/v2/algorithms/nn.json" \
        > "$TMPDIR/prev-tree/v2/algorithms/zzz_stray.json"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage-stray" \
        --runs "$TMPDIR/sa-only.jsonl"
    [ "$status" -eq 0 ]
    [ ! -f "$TMPDIR/stage-stray/v2/algorithms/zzz_stray.json" ]
    [ "$(jq -r '[.solvers[].id] | index("zzz_stray") == null' "$TMPDIR/stage-stray/v2/index.json")" = "true" ]
}

@test "a first publish against an empty prefix succeeds via the 404 path" {
    # The route the previous tests could not reach: they use --no-fetch-previous, so the
    # fetch path never runs. It had a blocker — fetch_previous created the shard directories
    # before discovering the 404, and the transform then read the leftovers as a tree with no
    # manifest and aborted, so the very first publish could never succeed.
    local root="$TMPDIR/webroot" portfile="$TMPDIR/port" logfile="$TMPDIR/http.log"
    mkdir -p "$root"
    SERVER_PID=""
    python3 - "$root" "$portfile" >"$logfile" 2>&1 <<'PY' &
import functools
import http.server
import pathlib
import socketserver
import sys

root, portfile = sys.argv[1], sys.argv[2]
handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=root)
with socketserver.TCPServer(("127.0.0.1", 0), handler) as httpd:
    pathlib.Path(portfile).write_text(str(httpd.server_address[1]))
    httpd.serve_forever()
PY
    SERVER_PID=$!
    local i
    for i in $(seq 50); do
        [ -s "$portfile" ] && break
        sleep 0.1
    done
    [ -s "$portfile" ] || skip "could not start a local HTTP server"

    local port
    port="$(cat "$portfile")"
    run "$PUBLISH" --dry-run --first-publish --stage "$TMPDIR/first" \
        --public-base "http://127.0.0.1:$port" --runs "$TMPDIR/prev.jsonl"

    kill "$SERVER_PID" 2>/dev/null || true

    # An empty bucket is a first publish, not a failure.
    [ "$status" -eq 0 ]
    [[ "$output" == *"HTTP 404"* ]]
    [ -f "$TMPDIR/first/v2/index.json" ]
    [ "$(jq -r '.solvers | length' "$TMPDIR/first/v2/index.json")" = "2" ]
    # And the leftovers the failed attempt used to choke on must not be treated as a tree.
    [ ! -f "$TMPDIR/first/v2/../../bench-prev/v2/index.json" ]
}

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

    # Two solvers over two instances: eil51 has a known optimal (426), zzz_unknown has no doc
    # at all and so no known optimal. `nn` is deterministic, `sa` is not.
    # berlin52 is deliberately NOT used: its doc records `optimalCost: null`, so it cannot
    # exercise the known-optimal path.
    cat > "$TMPDIR/prev.tsv" <<'TSV'
# git_commit=aaaaaaaa
# dirty=0
# teeline_version=1.0.0
# rust_version=1.90.0
# tier=a
# timeout_s=180
solver	dataset	config	run	wall_s	peak_rss_kb	tour_cost	status
nn	eil51	default	1	0.01	6000	450.0	ok
nn	eil51	default	2	0.01	6000	450.0	ok
nn	zzz_unknown	default	1	0.02	6100	1234.0	ok
nn	zzz_unknown	default	2	0.02	6100	1234.0	ok
sa	eil51	default	1	0.30	8000	440.0	ok
sa	eil51	default	2	0.31	8100	430.0	ok
sa	zzz_unknown	default	1	0.40	8200	1200.0	ok
sa	zzz_unknown	default	2	0.41	8200	1210.0	ok
TSV

    # A later measurement of `sa` alone, at a different commit — the differential case.
    cat > "$TMPDIR/sa-only.tsv" <<'TSV'
# git_commit=bbbbbbbb
# dirty=0
# teeline_version=1.0.1
# rust_version=1.90.0
# tier=a
# timeout_s=180
solver	dataset	config	run	wall_s	peak_rss_kb	tour_cost	status
sa	eil51	default	1	0.20	8000	428.0	ok
sa	eil51	default	2	0.21	8100	427.0	ok
sa	zzz_unknown	default	1	0.30	8200	1100.0	ok
sa	zzz_unknown	default	2	0.31	8200	1110.0	ok
TSV

    # Timeouts and an error must be counted, never averaged into the gap.
    cat > "$TMPDIR/timeouts.tsv" <<'TSV'
# git_commit=cccccccc
# tier=a
solver	dataset	config	run	wall_s	peak_rss_kb	tour_cost	status
nn	eil51	default	1	0.01	6000	450.0	ok
nn	eil51	default	2	180.0	6000	0.0	timeout
nn	eil51	default	3	180.0	6000	0.0	timeout
nn	eil51	default	4	0.02	6000	460.0	ok
nn	eil51	default	5	0.01	6000	0.0	error
TSV
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
    run publish --tsv "$TMPDIR/prev.tsv"
    [ "$status" -eq 0 ]

    local shard="$TMPDIR/stage/v2/algorithms/nn.json"
    [ -f "$shard" ]
    [ "$(jq -r '.measurement.git_commit' "$shard")" = "aaaaaaaa" ]
    [ "$(jq -r '.measurement.git_commit_source' "$shard")" = "tsv" ]
    [ "$(jq -r '.measurement.dirty' "$shard")" = "false" ]
    [ "$(jq -r '.measurement.teeline_version' "$shard")" = "1.0.0" ]
    [ "$(jq -r '.measurement.environment.build' "$shard")" = "release" ]
    [ -n "$(jq -r '.measurement.measured_at' "$shard")" ]
}

@test "index.json makes no dataset-level claim about the measured commit" {
    run publish --tsv "$TMPDIR/prev.tsv"
    [ "$status" -eq 0 ]

    local idx="$TMPDIR/stage/v2/index.json"
    # The whole point: one commit cannot describe solvers measured at different commits.
    for field in git_commit git_commit_source dirty teeline_version environment; do
        [ "$(jq "has(\"$field\")" "$idx")" = "false" ]
    done
    [ "$(jq -r '.schema_version' "$idx")" = "2" ]
}

@test "the URL prefix and schema_version are bumped together" {
    run publish --tsv "$TMPDIR/prev.tsv"
    [ "$status" -eq 0 ]
    # publish is invoked with the default --prefix bench/v2; the transform wrote under v2.
    [ -d "$TMPDIR/stage/v2" ]
    [ "$(jq -r '.schema_version' "$TMPDIR/stage/v2/index.json")" = "2" ]
    [ "$(jq -r '.schema_version' "$TMPDIR/stage/v2/algorithms/nn.json")" = "2" ]
}

@test "per-family budgets are published with their agreed status" {
    run publish --tsv "$TMPDIR/prev.tsv"
    [ "$status" -eq 0 ]

    local idx="$TMPDIR/stage/v2/index.json"
    # meta 300s is the only figure fixed by a decision; the rest are flagged provisional.
    [ "$(jq -r '.budgets.meta.seconds' "$idx")" = "300" ]
    [ "$(jq -r '.budgets.meta.agreed' "$idx")" = "true" ]
    [ "$(jq -r '.budgets.local.agreed' "$idx")" = "false" ]
}

@test "an unknown optimal stays null and yields no gap, never a zero gap" {
    run publish --tsv "$TMPDIR/prev.tsv"
    [ "$status" -eq 0 ]

    local shard="$TMPDIR/stage/v2/algorithms/nn.json"
    [ "$(jq -r '.results.zzz_unknown.optimal_cost' "$shard")" = "null" ]
    # A gap of 0 would read as "reached the optimum", a claim we cannot make here.
    [ "$(jq -r '.results.zzz_unknown.by_config.default | has("gap_pct")' "$shard")" = "false" ]

    # The known-optimal instance does carry a gap.
    [ "$(jq -r '.results.eil51.by_config.default | has("gap_pct")' "$shard")" = "true" ]
}

@test "timeouts and errors are counted separately and kept out of the gap" {
    run publish --tsv "$TMPDIR/timeouts.tsv"
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
    publish --tsv "$TMPDIR/prev.tsv"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage2" \
        --tsv "$TMPDIR/sa-only.tsv"
    [ "$status" -eq 0 ]

    # `sa` was re-measured: new commit.
    [ "$(jq -r '.measurement.git_commit' "$TMPDIR/stage2/v2/algorithms/sa.json")" = "bbbbbbbb" ]
    # `nn` was not: byte-identical shard, so it still reports the commit that measured it.
    cmp -s "$TMPDIR/prev-tree/v2/algorithms/nn.json" "$TMPDIR/stage2/v2/algorithms/nn.json"
    [ "$(jq -r '.measurement.git_commit' "$TMPDIR/stage2/v2/algorithms/nn.json")" = "aaaaaaaa" ]
}

@test "a differential republish keeps unmeasured solvers on the problem leaderboard" {
    publish --tsv "$TMPDIR/prev.tsv"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    local before after
    before="$(jq -r '[.leaderboard[].solver] | sort | join(",")' "$TMPDIR/prev-tree/v2/problems/eil51.json")"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage2" \
        --tsv "$TMPDIR/sa-only.tsv"
    [ "$status" -eq 0 ]

    after="$(jq -r '[.leaderboard[].solver] | sort | join(",")' "$TMPDIR/stage2/v2/problems/eil51.json")"
    # Replacing the shard from a one-solver TSV would have dropped nn from the ranking.
    [ "$before" = "$after" ]
    [ "$after" = "nn,sa" ]
}

@test "a differential republish uploads only the shard it re-measured" {
    publish --tsv "$TMPDIR/prev.tsv"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage2" \
        --tsv "$TMPDIR/sa-only.tsv"
    [ "$status" -eq 0 ]

    # nn's shard is unchanged, so it must not be re-uploaded; sa's must be.
    [[ "$output" == *"would put bench/v2/algorithms/sa.json"* ]]
    [[ "$output" != *"would put bench/v2/algorithms/nn.json"* ]]
    # index.json is expected to differ (it carries generated_at); the point is nn's shard.
    [ "$(jq -r '.measurement.git_commit' "$TMPDIR/prev-tree/v2/algorithms/nn.json")" = "aaaaaaaa" ]
}

@test "a release snapshot is written as a complete immutable copy" {
    run publish --tsv "$TMPDIR/prev.tsv" --snapshot v1.0.0
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

@test "publish-time provenance is labelled rather than implied when the TSV has none" {
    # A TSV with no # header: the commit can only be HEAD at publish time and does not
    # necessarily identify the binary, so the source must say so.
    printf 'solver\tdataset\tconfig\trun\twall_s\tpeak_rss_kb\ttour_cost\tstatus\nnn\tberlin52\tdefault\t1\t0.01\t6000\t8980.0\tok\n' \
        > "$TMPDIR/bare.tsv"
    run publish --tsv "$TMPDIR/bare.tsv"
    [ "$status" -eq 0 ]

    local shard="$TMPDIR/stage/v2/algorithms/nn.json"
    [ "$(jq -r '.measurement.git_commit_source' "$shard")" = "publish-time" ]
}

@test "a TSV without a measured_at yields null rather than the publish time" {
    run publish --tsv "$TMPDIR/prev.tsv"
    [ "$status" -eq 0 ]
    # The fixture records no `# measured_at=`, and stamping it with the publish time would
    # misdate a result that may have been carried forward from an earlier run.
    [ "$(jq -r '.measurement.measured_at' "$TMPDIR/stage/v2/algorithms/nn.json")" = "null" ]
}

@test "a TSV that records measured_at has it published" {
    { echo "# measured_at=2026-01-02T03:04:05Z"; cat "$TMPDIR/prev.tsv"; } > "$TMPDIR/stamped.tsv"
    run publish --tsv "$TMPDIR/stamped.tsv"
    [ "$status" -eq 0 ]
    [ "$(jq -r '.measurement.measured_at' "$TMPDIR/stage/v2/algorithms/nn.json")" = "2026-01-02T03:04:05Z" ]
}

@test "a partial publish with no previous tree is refused unless stated" {
    # This is the guard that stops a differential run from quietly replacing the live manifest
    # and leaderboards with a tree holding only this TSV's solvers.
    run "$PUBLISH" --dry-run --no-fetch-previous --stage "$TMPDIR/nostage" --tsv "$TMPDIR/prev.tsv"
    [ "$status" -ne 0 ]
    [[ "$output" == *"--first-publish"* ]]
    [ ! -f "$TMPDIR/nostage/v2/index.json" ]
}

@test "an invalid snapshot tag is refused before it reaches the filesystem" {
    run publish --tsv "$TMPDIR/prev.tsv" --snapshot "../escape"
    [ "$status" -ne 0 ]
    [[ "$output" == *"invalid snapshot tag"* ]]
    # Nothing outside the snapshots directory may be touched.
    [ ! -d "$TMPDIR/stage/v2/escape" ]
}

@test "a differential republish drops entries for a re-measured solver on other datasets" {
    publish --tsv "$TMPDIR/prev.tsv"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    # Measure sa on eil51 only, so its replacement shard no longer covers zzz_unknown.
    cat > "$TMPDIR/sa-partial.tsv" <<'TSV'
# git_commit=bbbbbbbb
solver	dataset	config	run	wall_s	peak_rss_kb	tour_cost	status
sa	eil51	default	1	0.20	8000	428.0	ok
TSV

    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage3" \
        --tsv "$TMPDIR/sa-partial.tsv"
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
    printf '# git_commit=dddddddd\nsolver\tdataset\tconfig\trun\twall_s\tpeak_rss_kb\ttour_cost\tstatus\nnn\teil51\tdefault\t1\t0.01\t6000\t450.0\tok\n' \
        > "$TMPDIR/partial-header.tsv"
    run publish --tsv "$TMPDIR/partial-header.tsv"
    [ "$status" -eq 0 ]

    local shard="$TMPDIR/stage/v2/algorithms/nn.json"
    [ "$(jq -r '.measurement.git_commit' "$shard")" = "dddddddd" ]
    [ "$(jq -r '.measurement.dirty' "$shard")" = "null" ]
    [ "$(jq -r '.measurement.teeline_version' "$shard")" = "null" ]
    [ "$(jq -r '.measurement.environment.rust_version' "$shard")" = "null" ]
}

@test "a previous manifest naming a shard that is not on disk is refused" {
    publish --tsv "$TMPDIR/prev.tsv"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    # The manifest still lists sa, but its shard is gone. Merging would drop sa from the
    # republished tree without a word, so this must abort instead.
    rm "$TMPDIR/prev-tree/v2/algorithms/sa.json"
    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage-missing" \
        --tsv "$TMPDIR/sa-only.tsv"
    [ "$status" -ne 0 ]
    [[ "$output" == *"not present in the tree"* ]]
}

@test "a shard the previous manifest does not name is not merged" {
    publish --tsv "$TMPDIR/prev.tsv"
    cp -r "$TMPDIR/stage" "$TMPDIR/prev-tree"

    # A stray shard was never live: resurrecting it would republish something no reader
    # could have seen, including one deliberately withdrawn.
    jq '.solver = "zzz_stray"' "$TMPDIR/prev-tree/v2/algorithms/nn.json" \
        > "$TMPDIR/prev-tree/v2/algorithms/zzz_stray.json"

    run "$PUBLISH" --dry-run --previous "$TMPDIR/prev-tree" --stage "$TMPDIR/stage-stray" \
        --tsv "$TMPDIR/sa-only.tsv"
    [ "$status" -eq 0 ]
    [ ! -f "$TMPDIR/stage-stray/v2/algorithms/zzz_stray.json" ]
    [ "$(jq -r '[.solvers[].id] | index("zzz_stray") == null' "$TMPDIR/stage-stray/v2/index.json")" = "true" ]
}

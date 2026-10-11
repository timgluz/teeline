#!/usr/bin/env bats
# End-to-end tests for scripts/bench-matrix.sh — the benchmark measurement harness.
#
# Run: ./tests/bats/bin/bats tests/e2e/bench-matrix.bats
# Requires the debug binary: cargo build -p teeline-cli
#
# The properties worth guarding here are the ones a sweep depends on: a resumed run must not
# repeat cells, a run that produced nothing must not invent a number, and the rows must carry
# provenance the publisher will accept. Each is cheap to get wrong in a way that only shows up
# after an hours-long run.

setup() {
    REPO_ROOT="$( cd "$( dirname "$BATS_TEST_FILENAME" )/../.." >/dev/null 2>&1 && pwd )"
    BENCH="$REPO_ROOT/scripts/bench-matrix.sh"
    PUBLISH="$REPO_ROOT/scripts/publish-benchmarks.sh"
    BIN="$REPO_ROOT/target/debug/teeline"
    TMPDIR="$(mktemp -d)"
    cd "$REPO_ROOT"

    # Only `tests/fixtures/` instances are used: `data/tsplib/` is git-ignored and downloaded,
    # so a test that needs it would pass locally and fail in CI.
    cat > "$TMPDIR/tier.toml" <<'TOML'
tier = "test"
runs = 2

# No default_timeout_s: there is deliberately no fallback, because a family measured under some
# default while the published budgets state another number would put an allowance on the page
# that the data was not taken under.
[budgets]
constructive = 30
exact = 5
local = 30
meta = 1
utility = 30

# Every family is named, even the ones this tier measures nowhere: silence about a family is
# indistinguishable from forgetting it, and the planner refuses the latter. The campaign's
# criterion is that a solver "appears, or is explicitly excluded with a reason".
[coverage]
constructive = ["small"]
exact = ["small"]
local = []
meta = ["big"]
utility = []

[instances]
burma14 = "small"
gr17 = "small"
att532 = "big"

[configs.default]
TOML

    # A committed-tree-free output path, so a test never appends to a real sweep.
    OUT="$TMPDIR/runs/tier-test.jsonl"
}

teardown() {
    rm -rf "$TMPDIR"
}

bench() {
    "$BENCH" "$TMPDIR/tier.toml" --binary "$BIN" --out "$OUT" "$@"
}

@test "dry-run lists cells and writes nothing" {
    run bench --dry-run
    [ "$status" -eq 0 ]
    [[ "$output" == *"cell(s)"* ]]
    [[ "$output" == *"burma14"* ]]
    [ ! -f "$OUT" ]
}

@test "the per-family budget becomes the run timeout" {
    run bench --dry-run --solver nn --solver sa
    [ "$status" -eq 0 ]
    # constructive 30s, meta 1s — from [budgets], not the default 30s.
    [[ "$output" == *"timeout=30s"* ]]
    [[ "$output" == *"timeout=1s"* ]]
}

@test "capability tiering keeps an exact solver off instances it cannot finish" {
    # `bhk` is exponential; [coverage] gives the exact family only the "small" group, so it must
    # not be planned on att532 at all — rather than burning a budget to record a certain timeout.
    run bench --dry-run --solver bhk
    [ "$status" -eq 0 ]
    [[ "$output" == *"burma14"* ]]
    [[ "$output" != *"att532"* ]]
}

@test "every emitted row is a complete JSON object with provenance" {
    run bench --solver nn
    [ "$status" -eq 0 ]
    [ -f "$OUT" ]
    [ "$(wc -l < "$OUT")" -eq 4 ]   # 2 instances x 2 runs

    # One JSON object per line, and the fields the publisher requires.
    run jq -c 'select((.solver and .dataset and .run and .status
                       and (.measurement.git_commit | length > 0)
                       and .measurement.git_commit_source) | not)' "$OUT"
    [ "$status" -eq 0 ]
    [ -z "$output" ]
}

@test "resuming skips cells already recorded" {
    bench --solver nn
    local before
    before="$(wc -l < "$OUT")"

    run bench --solver nn
    [ "$status" -eq 0 ]
    [[ "$output" == *"nothing to do"* ]]
    # A resumed run that re-measured would append duplicates and quietly double-count runs.
    [ "$(wc -l < "$OUT")" -eq "$before" ]
}

@test "a killed run is recorded as a timeout with no tour_cost" {
    # att532 at a 1s metaheuristc budget cannot finish, and the row must say so rather than
    # carry a number: a 0 tour cost would read as a -100% gap, "twice as good as optimal".
    run bench --solver sa
    [ "$status" -eq 0 ]
    [ -f "$OUT" ]

    [ "$(jq -r '.status' "$OUT" | sort -u)" = "timeout" ]
    run jq -c 'select(has("tour_cost"))' "$OUT"
    [ -z "$output" ]
    # The elapsed time is still a real observation, so it is kept.
    [ "$(jq -r '.wall_s > 0' "$OUT" | sort -u)" = "true" ]
}

@test "--solver appends to the existing file, and it still publishes" {
    bench --solver nn
    bench --solver branch_bound

    [ "$(jq -r .solver "$OUT" | sort -u | tr '\n' ' ')" = "branch_bound nn " ]

    # The publisher refuses a file whose rows for one solver disagree; appending must not
    # introduce that, or the second solver's shard could never be published.
    run "$PUBLISH" --dry-run --no-fetch-previous --first-publish \
        --stage "$TMPDIR/stage" --runs "$OUT"
    [ "$status" -eq 0 ]
    [[ "$output" == *"algorithms: 2 shard"* ]]
}

@test "rows from two builds are refused by the publisher" {
    # The case the per-row format exists for. A sweep interrupted by a rebuild appends rows from
    # a new binary; one shard cannot describe both, so the mismatch must be caught at publish
    # time rather than silently stamped with whichever commit came last.
    bench --solver nn
    python3 - "$OUT" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
rows = [json.loads(line) for line in path.read_text().splitlines()]
rows[0]["measurement"]["git_commit"] = "deadbee"   # as if measured before a rebuild
path.write_text("\n".join(json.dumps(r, separators=(",", ":")) for r in rows) + "\n")
PY

    run "$PUBLISH" --dry-run --no-fetch-previous --first-publish \
        --stage "$TMPDIR/stage-mixed" --runs "$OUT"
    [ "$status" -ne 0 ]
    [[ "$output" == *"different provenance"* ]]
    [ ! -f "$TMPDIR/stage-mixed/v2/index.json" ]
}

@test "a config restricted to solvers and instances is planned only there" {
    cat > "$TMPDIR/tier-configs.toml" <<'TOML'
tier = "test"
runs = 1

[budgets]
constructive = 5
exact = 5
local = 5
meta = 5
utility = 5

[coverage]
constructive = []
exact = ["small"]
local = []
meta = []
utility = []

[instances]
burma14 = "small"
gr17 = "small"

[configs.default]

[configs.exact]
solvers = ["branch_bound"]
instances = ["burma14"]
args = ["--n_nearest", "100"]
TOML

    run "$BENCH" "$TMPDIR/tier-configs.toml" --binary "$BIN" --out "$TMPDIR/c.jsonl" --dry-run
    [ "$status" -eq 0 ]
    # branch_bound gets both configs on burma14, but only `default` on gr17.
    [ "$(printf '%s\n' "$output" | grep -c 'burma14.*exact')" -eq 1 ]
    [ "$(printf '%s\n' "$output" | grep -c 'gr17.*exact')" -eq 0 ]
    # bhk is exact too, but the config names branch_bound only.
    [ "$(printf '%s\n' "$output" | grep -c 'bhk.*exact')" -eq 0 ]
}

@test "a missing instance fails loudly and writes nothing" {
    cat > "$TMPDIR/tier-missing.toml" <<'TOML'
tier = "test"
runs = 1
[budgets]
constructive = 5
exact = 5
local = 5
meta = 5
utility = 5
[coverage]
constructive = ["small"]
exact = []
local = []
meta = []
utility = []
[instances]
no_such_instance = "small"
[configs.default]
TOML

    run "$BENCH" "$TMPDIR/tier-missing.toml" --binary "$BIN" --out "$TMPDIR/m.jsonl"
    [ "$status" -ne 0 ]
    [[ "$output" == *"no TSLIB file"* || "$output" == *"no TSPLIB file"* ]]
    [ ! -f "$TMPDIR/m.jsonl" ]
}

@test "an unknown --solver fails rather than silently measuring nothing" {
    run bench --solver not_a_solver
    [ "$status" -ne 0 ]
    [[ "$output" == *"unknown solver"* ]]
}

@test "a half-written final line is dropped on resume, not planned around" {
    bench --solver nn
    local before
    before="$(wc -l < "$OUT")"
    # As if the process died mid-write: the publisher would reject the whole file over this.
    printf '{"solver":"nn","dataset":"burma1' >> "$OUT"

    run bench --solver nn
    [ "$status" -eq 0 ]
    [[ "$output" == *"incomplete final line"* ]]
    [[ "$output" == *"nothing to do"* ]]
    # The truncated line is gone and no cell was re-run.
    [ "$(wc -l < "$OUT")" -eq "$before" ]
    run jq -e . "$OUT"
    [ "$status" -eq 0 ]
}

@test "a tier that stays silent about a solver family is refused" {
    # A family absent from [coverage] is indistinguishable from one somebody forgot, and the
    # result would be a published corpus missing those solvers with nothing to show for it.
    # Naming it with an empty group list is how a tier says "deliberately not measured here".
    cat > "$TMPDIR/tier-silent.toml" <<'TOML'
tier = "test"
runs = 1
[budgets]
constructive = 5
[coverage]
constructive = ["small"]
[instances]
burma14 = "small"
[configs.default]
TOML

    run "$BENCH" "$TMPDIR/tier-silent.toml" --binary "$BIN" --out "$TMPDIR/s.jsonl" --dry-run
    [ "$status" -ne 0 ]
    [[ "$output" == *"does not mention"* ]]
    [ ! -f "$TMPDIR/s.jsonl" ]
}

@test "the published budgets match the tier the numbers were measured under" {
    # The page states a per-family budget beside the results. If it disagreed with the tier that
    # produced them, it would state an allowance the data was never measured under — the same
    # false-claim class as a wrong provenance commit, and just as invisible to a reader.
    run python3 - "$REPO_ROOT/bench/budgets.json" "$REPO_ROOT/bench/matrix/tier-a.toml" <<'PY'
import json, sys, tomllib

published = json.load(open(sys.argv[1]))["budgets"]
tier = tomllib.load(open(sys.argv[2], "rb"))["budgets"]

published_secs = {family: entry["seconds"] for family, entry in published.items()}
mismatch = sorted(
    f"{family}: published={published_secs.get(family)}s tier={tier.get(family)}s"
    for family in set(published_secs) | set(tier)
    if published_secs.get(family) != tier.get(family)
)
if mismatch:
    sys.exit("budget mismatch:\n  " + "\n  ".join(mismatch))
print("budgets agree")
PY
    [ "$status" -eq 0 ]
    [[ "$output" == *"budgets agree"* ]]
}

@test "every instance in the tier parses" {
    # A tier entry that cannot be read must be caught here, not three hours into a sweep: si175
    # was rejected for its UPPER_DIAG_ROW format, which is why gr120 stands in for it. That
    # substitution outlives the gap — swapping the instance would break comparability with the
    # published runs.
    run python3 - "$REPO_ROOT/bench/matrix/tier-a.toml" "$BIN" <<'PY'
import subprocess, sys, tomllib
from pathlib import Path

tier = tomllib.load(open(sys.argv[1], "rb"))
binary = sys.argv[2]
dirs = ("tests/fixtures", "data/tsplib")
missing = []
for name in tier["instances"]:
    path = next((Path(d) / f"{name}.tsp" for d in dirs if (Path(d) / f"{name}.tsp").is_file()), None)
    if path is None:
        # data/tsplib is git-ignored, so an absent file is not a tier defect in CI; the harness
        # fails on it at run time instead.
        continue
    proc = subprocess.run([binary, "solve", "nn", "-i", str(path)],
                          stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    if proc.returncode != 0:
        missing.append(f"{name}: {proc.stderr.strip().splitlines()[0] if proc.stderr else 'rc'} ")
if missing:
    sys.exit("instances that do not parse:\n  " + "\n  ".join(missing))
print("instances parse")
PY
    [ "$status" -eq 0 ]
    [[ "$output" == *"instances parse"* ]]
}

@test "a resumed sweep keeps the measurement its earlier rows carry" {
    # measured_at is resolved per process, so a resume in a fresh process would otherwise stamp
    # the same solver's rows with a different time. The publisher compares the whole measurement
    # and cannot tell a clock tick from a rebuild, so it would refuse the file — which would make
    # resume, the feature the format exists for, unusable. The earlier block must be reused.
    bench --runs 1 --solver nn
    sleep 1.1                       # measured_at has one-second precision
    bench --runs 2 --solver nn

    [ "$(jq -r .measurement.measured_at "$OUT" | sort -u | wc -l)" -eq 1 ]
    [ "$(wc -l < "$OUT")" -eq 4 ]   # run 1 from the first sweep, run 2 from the second

    run "$PUBLISH" --dry-run --no-fetch-previous --first-publish \
        --stage "$TMPDIR/stage-resume" --runs "$OUT"
    [ "$status" -eq 0 ]
}

@test "a config naming a solver that does not exist is refused" {
    # A typo here matches nothing, so those cells are silently never planned and the tier reads as
    # if it measures something it does not.
    cat > "$TMPDIR/tier-typo-solver.toml" <<'TOML'
tier = "test"
runs = 1
[budgets]
exact = 5
[coverage]
exact = ["small"]
[instances]
burma14 = "small"
[configs.default]
[configs.exact]
solvers = ["branch-bound"]
args = ["--n_nearest", "100"]
TOML

    run "$BENCH" "$TMPDIR/tier-typo-solver.toml" --binary "$BIN" --out "$TMPDIR/t1.jsonl" --dry-run
    [ "$status" -ne 0 ]
    [[ "$output" == *"branch-bound"* ]]
}

@test "a config naming an instance that does not exist is refused" {
    cat > "$TMPDIR/tier-typo-instance.toml" <<'TOML'
tier = "test"
runs = 1
[budgets]
constructive = 5
[coverage]
constructive = ["small"]
[instances]
burma14 = "small"
[configs.default]
[configs.onlyburma]
instances = ["burma-14"]
TOML

    run "$BENCH" "$TMPDIR/tier-typo-instance.toml" --binary "$BIN" --out "$TMPDIR/t2.jsonl" --dry-run
    [ "$status" -ne 0 ]
    [[ "$output" == *"burma-14"* ]]
}

@test "an instance group that no family names is refused" {
    # Otherwise every instance in that group is never measured, and the tier still looks complete.
    cat > "$TMPDIR/tier-orphan-group.toml" <<'TOML'
tier = "test"
runs = 1
[budgets]
constructive = 5
[coverage]
constructive = ["small"]
[instances]
burma14 = "small"
gr17 = "smlal"
[configs.default]
TOML

    run "$BENCH" "$TMPDIR/tier-orphan-group.toml" --binary "$BIN" --out "$TMPDIR/o.jsonl" --dry-run
    [ "$status" -ne 0 ]
    [[ "$output" == *"smlal"* ]]
}

@test "a family in coverage with no budget is refused" {
    # It would be measured under some fallback while bench/budgets.json states another number, so
    # the page would report an allowance the data was not taken under.
    cat > "$TMPDIR/tier-no-budget.toml" <<'TOML'
tier = "test"
runs = 1
[budgets]
constructive = 5
[coverage]
constructive = ["small"]
meta = ["small"]
[instances]
burma14 = "small"
[configs.default]
TOML
    # `nn` is constructive, which does have a budget...
    run "$BENCH" "$TMPDIR/tier-no-budget.toml" --binary "$BIN" --out "$TMPDIR/b.jsonl" --dry-run --solver nn
    [ "$status" -eq 0 ]
    # ...while `sa` is meta, which [coverage] names but [budgets] does not.
    run "$BENCH" "$TMPDIR/tier-no-budget.toml" --binary "$BIN" --out "$TMPDIR/b.jsonl" --dry-run --solver sa
    [ "$status" -ne 0 ]
    [[ "$output" == *"no [budgets] entry"* ]]
}

@test "--help describes the options and is not truncated" {
    run "$BENCH" --help
    [ "$status" -eq 0 ]
    [[ "$output" == *"--retry-errors"* ]]
    [[ "$output" == *"--dry-run"* ]]
    # The header's rationale must not bleed into the usage text.
    [[ "$output" != *"Provenance travels"* ]]
}

@test "a dry run leaves an existing output file untouched" {
    bench --solver nn
    printf '{"solver":"nn","dataset":"burma1' >> "$OUT"     # as if killed mid-write
    local before
    before="$(wc -c < "$OUT")"

    run bench --solver nn --dry-run
    [ "$status" -eq 0 ]
    # --dry-run promises to change nothing, so it must not repair the file either.
    [ "$(wc -c < "$OUT")" -eq "$before" ]
}

@test "a tier with no configs at all is refused" {
    # The name is a label, so any name works — but with no configs there is nothing to measure,
    # and the sweep would plan zero cells and look complete.
    cat > "$TMPDIR/tier-noconfig.toml" <<'TOML'
tier = "test"
runs = 1
[budgets]
constructive = 5
[coverage]
constructive = ["small"]
[instances]
burma14 = "small"
TOML

    run "$BENCH" "$TMPDIR/tier-noconfig.toml" --binary "$BIN" --out "$TMPDIR/n.jsonl" --dry-run
    [ "$status" -ne 0 ]
    [[ "$output" == *"no [configs"* ]]
}

@test "a config may be named anything, since the name is only a label" {
    cat > "$TMPDIR/tier-named.toml" <<'TOML'
tier = "test"
runs = 1
[budgets]
constructive = 5
[coverage]
constructive = ["small"]
[instances]
burma14 = "small"
[configs.budget]
TOML

    # --solver nn so the tier need not enumerate every family it does not measure.
    run "$BENCH" "$TMPDIR/tier-named.toml" --binary "$BIN" --out "$TMPDIR/named.jsonl" \
        --dry-run --solver nn
    [ "$status" -eq 0 ]
    [[ "$output" == *"budget"* ]]
    [[ "$output" != *"default"* ]]
}

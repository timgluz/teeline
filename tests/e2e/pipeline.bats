#!/usr/bin/env bats
# End-to-end CLI tests for pipeline and solve subcommands.
# Run: ./tests/bats/bin/bats tests/e2e/pipeline.bats
# Requires the debug binary to be built first: cargo build

setup() {
    # Resolve the repo root relative to this file so tests work regardless of
    # the working directory from which bats is invoked.
    REPO_ROOT="$( cd "$( dirname "$BATS_TEST_FILENAME" )/../.." >/dev/null 2>&1 && pwd )"
    BIN="$REPO_ROOT/target/debug/teeline"
    BERLIN52="$REPO_ROOT/tests/fixtures/berlin52.tsp"
    GR17="$REPO_ROOT/tests/fixtures/gr17.tsp"
    FIXTURE_DIR="$REPO_ROOT/tests/fixtures"
}

# ---------------------------------------------------------------------------
# solve subcommand
# ---------------------------------------------------------------------------

@test "solve rejects --config flag" {
  run "$BIN" solve nn "--config=${FIXTURE_DIR}/pipeline_nn_minimal.toml" -i "$BERLIN52"
  [ "$status" -ne 0 ]
}

# ---------------------------------------------------------------------------
# pipeline subcommand — happy path
# ---------------------------------------------------------------------------

@test "pipeline --steps=nn,2opt exits 0" {
    run "$BIN" pipeline --steps=nn,2opt -i "$GR17"
    [ "$status" -eq 0 ]
}

@test "pipeline --config exits 0 with valid TOML config" {
    run "$BIN" pipeline "--config=${FIXTURE_DIR}/pipeline_nn_minimal.toml" -i "$GR17"
    [ "$status" -eq 0 ]
}

@test "pipeline --config with sa options exits 0" {
    run "$BIN" pipeline "--config=${FIXTURE_DIR}/pipeline_global_nn_sa.toml" -i "$GR17"
    [ "$status" -eq 0 ]
}

# ---------------------------------------------------------------------------
# pipeline subcommand — error cases
# ---------------------------------------------------------------------------

@test "pipeline --steps and --config are mutually exclusive" {
    run "$BIN" pipeline --steps=nn,2opt "--config=${FIXTURE_DIR}/pipeline_nn_minimal.toml" -i "$GR17"
    [ "$status" -ne 0 ]
    echo "$output" | grep -i "mutually exclusive"
}

@test "pipeline with neither --steps nor --config exits non-0" {
    run "$BIN" pipeline -i "$GR17"
    [ "$status" -ne 0 ]
}

@test "pipeline --steps with unknown solver exits non-0" {
    run "$BIN" pipeline --steps=nn,bogus -i "$GR17"
    [ "$status" -ne 0 ]
}

@test "pipeline --config with nonexistent file exits non-0 and names the file" {
    run "$BIN" pipeline --config=/nonexistent/path.toml -i "$GR17"
    [ "$status" -ne 0 ]
    echo "$output" | grep "path.toml"
}

@test "pipeline --config with bad cooling_rate exits non-0" {
    run "$BIN" pipeline "--config=${FIXTURE_DIR}/pipeline_bad_cooling_rate.toml" -i "$GR17"
    [ "$status" -ne 0 ]
}

@test "pipeline --config with unknown key exits non-0" {
    run "$BIN" pipeline "--config=${FIXTURE_DIR}/pipeline_unknown_key.toml" -i "$GR17"
    [ "$status" -ne 0 ]
}

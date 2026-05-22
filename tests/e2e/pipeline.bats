#!/usr/bin/env bats
# End-to-end CLI tests for pipeline and solve subcommands.
# Run: ./tests/bats/bin/bats tests/e2e/pipeline.bats
# Requires the debug binary to be built first: cargo build

setup() {
    # Resolve the repo root relative to this file so tests work regardless of
    # the working directory from which bats is invoked.
    REPO_ROOT="$( cd "$( dirname "$BATS_TEST_FILENAME" )/../.." >/dev/null 2>&1 && pwd )"
    BIN="$REPO_ROOT/target/debug/bin"
    BERLIN52="$REPO_ROOT/tests/fixtures/berlin52.tsp"
    FIXTURE_DIR="$REPO_ROOT/tests/fixtures"
}

# ---------------------------------------------------------------------------
# solve subcommand
# ---------------------------------------------------------------------------

@test "solve rejects --config flag" {
  run "$BIN" solve nn "--config=${FIXTURE_DIR}/pipeline_nn_minimal.toml" -i "$BERLIN52"
  [ "$status" -ne 0 ]
}

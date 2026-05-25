#!/usr/bin/env bats
# End-to-end CLI tests for the `convert` subcommand.
# Run: ./tests/bats/bin/bats tests/e2e/convert.bats
# Requires the debug binary to be built first: cargo build

setup() {
    REPO_ROOT="$( cd "$( dirname "$BATS_TEST_FILENAME" )/../.." >/dev/null 2>&1 && pwd )"
    BIN="$REPO_ROOT/target/debug/bin"
    FIXTURE_DIR="$REPO_ROOT/tests/fixtures"
    # Write converted output to a temp dir so tests are side-effect-free.
    TMPDIR="$(mktemp -d)"
}

teardown() {
    rm -rf "$TMPDIR"
}

@test "convert DiscOpt file exits 0 and produces a .tsp file" {
    run "$BIN" convert -i "${FIXTURE_DIR}/tiny5_discopt" -o "$TMPDIR"
    [ "$status" -eq 0 ]
    [ -f "${TMPDIR}/tiny5_discopt.tsp" ]
}

@test "convert DiscOpt file output contains NODE_COORD_SECTION" {
    "$BIN" convert -i "${FIXTURE_DIR}/tiny5_discopt" -o "$TMPDIR"
    grep -q "NODE_COORD_SECTION" "${TMPDIR}/tiny5_discopt.tsp"
}

@test "convert nonexistent input exits non-0" {
    run "$BIN" convert -i /nonexistent/file
    [ "$status" -ne 0 ]
}

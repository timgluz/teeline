#!/usr/bin/env bats
# End-to-end CLI tests for the `solve` subcommand.
# Run: ./tests/bats/bin/bats tests/e2e/solve.bats
# Requires the debug binary to be built first: cargo build

setup() {
    REPO_ROOT="$( cd "$( dirname "$BATS_TEST_FILENAME" )/../.." >/dev/null 2>&1 && pwd )"
    BIN="$REPO_ROOT/target/debug/teeline"
    GR17="$REPO_ROOT/tests/fixtures/gr17.tsp"
    BERLIN52="$REPO_ROOT/tests/fixtures/berlin52.tsp"
}

# ---------------------------------------------------------------------------
# Happy path — solver produces valid output
# ---------------------------------------------------------------------------

@test "solve nn exits 0 and prints cost on first line" {
    # Redirect stderr so tracing logs don't mix into $output.
    run bash -c "\"$BIN\" solve nn -i \"$GR17\" 2>/dev/null"
    [ "$status" -eq 0 ]
    # First line: "<float> <0|1>"
    echo "$output" | head -1 | grep -E '^[0-9]+\.[0-9]+ [01]$'
}

@test "solve 2opt exits 0 (auto-expands to nn → 2opt)" {
    run "$BIN" solve 2opt -i "$GR17"
    [ "$status" -eq 0 ]
}

@test "solve classic preset exits 0" {
    run "$BIN" solve classic -i "$GR17"
    [ "$status" -eq 0 ]
}

@test "solve fast preset exits 0" {
    run "$BIN" solve fast -i "$GR17"
    [ "$status" -eq 0 ]
}

@test "solve nn --no-seed exits 0" {
    run "$BIN" solve nn --no-seed -i "$GR17"
    [ "$status" -eq 0 ]
}

@test "solve nn reads from stdin when -i is omitted" {
    run bash -c "\"$BIN\" solve nn < \"$GR17\""
    [ "$status" -eq 0 ]
}

# ---------------------------------------------------------------------------
# Error cases
# ---------------------------------------------------------------------------

@test "solve unknown solver exits non-0" {
    run "$BIN" solve bogus -i "$GR17"
    [ "$status" -ne 0 ]
}

@test "solve rejects --config flag" {
    run "$BIN" solve nn "--config=foo.toml" -i "$GR17"
    [ "$status" -ne 0 ]
}

@test "solve sa rejects out-of-range cooling_rate" {
    run "$BIN" solve sa --cooling_rate 5.0 -i "$GR17"
    [ "$status" -ne 0 ]
    echo "$output" | grep -i "cooling_rate"
}

@test "solve sa rejects non-numeric cooling_rate" {
    run "$BIN" solve sa --cooling_rate abc -i "$GR17"
    [ "$status" -ne 0 ]
}

@test "solve nn rejects n_nearest=0" {
    run "$BIN" solve nn --n_nearest 0 -i "$GR17"
    [ "$status" -ne 0 ]
    echo "$output" | grep -i "n_nearest"
}

# ---------------------------------------------------------------------------
# --output-format=json
# ---------------------------------------------------------------------------

@test "solve nn --output-format=json exits 0" {
    run bash -c "\"$BIN\" solve nn -i \"$GR17\" --output-format=json 2>/dev/null"
    [ "$status" -eq 0 ]
}

@test "solve nn --output-format=json outputs valid JSON with cost and route" {
    run bash -c "\"$BIN\" solve nn -i \"$GR17\" --output-format=json 2>/dev/null"
    [ "$status" -eq 0 ]
    echo "$output" | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'cost' in d and 'route' in d"
}

@test "solve nn --output-format=json route is an array of integers" {
    run bash -c "\"$BIN\" solve nn -i \"$GR17\" --output-format=json 2>/dev/null"
    [ "$status" -eq 0 ]
    echo "$output" | python3 -c "import sys,json; d=json.load(sys.stdin); assert all(isinstance(x,int) for x in d['route'])"
}

@test "solve --output-format=json does not regress text default" {
    run bash -c "\"$BIN\" solve nn -i \"$GR17\" 2>/dev/null"
    [ "$status" -eq 0 ]
    echo "$output" | head -1 | grep -E '^[0-9]+\.[0-9]+ [01]$'
}

#!/usr/bin/env bats
# End-to-end CLI tests for the `solvers` subcommand.
# Run: ./tests/bats/bin/bats tests/e2e/solvers.bats
# Requires the debug binary to be built first: cargo build -p teeline-gui

setup() {
    REPO_ROOT="$( cd "$( dirname "$BATS_TEST_FILENAME" )/../.." >/dev/null 2>&1 && pwd )"
    BIN="$REPO_ROOT/target/debug/teeline"
}

# ---------------------------------------------------------------------------
# Default (table) output
# ---------------------------------------------------------------------------

@test "solvers exits 0" {
    run "$BIN" solvers
    [ "$status" -eq 0 ]
}

@test "solvers prints header row" {
    run "$BIN" solvers
    echo "$output" | grep -E '^NAME'
}

@test "solvers table includes nn as heuristic" {
    run "$BIN" solvers
    echo "$output" | grep -E 'nearest_neighbor\s+nn\s+heuristic'
}

@test "solvers table includes bhk as exact" {
    run "$BIN" solvers
    echo "$output" | grep -E 'bellman_karp\s+bhk\s+exact'
}

@test "solvers table includes random_shuffle as utility" {
    run "$BIN" solvers
    echo "$output" | grep -E 'random_shuffle\s+shuffle\s+utility'
}

# ---------------------------------------------------------------------------
# --short output
# ---------------------------------------------------------------------------

@test "solvers --short prints one token per line without header" {
    run "$BIN" solvers --short
    [ "$status" -eq 0 ]
    # No header line
    echo "$output" | grep -v '^NAME'
    # Each line is a single word (no spaces)
    while IFS= read -r line; do
        [[ "$line" =~ ^[a-z0-9_-]+$ ]]
    done <<< "$output"
}

@test "solvers --short output is usable in a for loop" {
    run bash -c "\"$BIN\" solvers --short | wc -l"
    [ "$status" -eq 0 ]
    # At least 10 solvers
    [ "$output" -ge 10 ]
}

# ---------------------------------------------------------------------------
# --heuristic filter
# ---------------------------------------------------------------------------

@test "solvers --heuristic --short excludes bhk" {
    run "$BIN" solvers --heuristic --short
    [ "$status" -eq 0 ]
    echo "$output" | grep -v '^bhk$'
    ! echo "$output" | grep -qx 'bhk'
}

@test "solvers --heuristic --short excludes branch_bound" {
    run "$BIN" solvers --heuristic --short
    ! echo "$output" | grep -qx 'branch_bound'
}

@test "solvers --heuristic --short includes nn" {
    run "$BIN" solvers --heuristic --short
    echo "$output" | grep -qx 'nn'
}

# ---------------------------------------------------------------------------
# --exact filter
# ---------------------------------------------------------------------------

@test "solvers --exact --short returns only bhk and branch_bound" {
    run "$BIN" solvers --exact --short
    [ "$status" -eq 0 ]
    # Must contain bhk
    echo "$output" | grep -qx 'bhk'
    # Must contain branch_bound
    echo "$output" | grep -qx 'branch_bound'
    # Must contain exactly 2 lines
    [ "$(echo "$output" | wc -l)" -eq 2 ]
}

# ---------------------------------------------------------------------------
# Conflicting flags
# ---------------------------------------------------------------------------

@test "solvers --heuristic --exact exits non-zero" {
    run "$BIN" solvers --heuristic --exact
    [ "$status" -ne 0 ]
}

# ---------------------------------------------------------------------------
# --output-format=json
# ---------------------------------------------------------------------------

@test "solvers --output-format=json exits 0" {
    run "$BIN" solvers --output-format=json
    [ "$status" -eq 0 ]
}

@test "solvers --output-format=json outputs a JSON array" {
    run "$BIN" solvers --output-format=json
    [ "$status" -eq 0 ]
    echo "$output" | python3 -c "import sys,json; arr=json.load(sys.stdin); assert isinstance(arr, list)"
}

@test "solvers --output-format=json includes nn with alias field" {
    run "$BIN" solvers --output-format=json
    echo "$output" | python3 -c "import sys,json; arr=json.load(sys.stdin); assert any(s['alias']=='nn' for s in arr)"
}

@test "solvers --output-format=json includes category and complexity fields" {
    run "$BIN" solvers --output-format=json
    echo "$output" | python3 -c "import sys,json; arr=json.load(sys.stdin); assert all('category' in s and 'complexity' in s for s in arr)"
}

@test "solvers --exact --output-format=json returns only exact solvers" {
    run "$BIN" solvers --exact --output-format=json
    [ "$status" -eq 0 ]
    echo "$output" | python3 -c "import sys,json; arr=json.load(sys.stdin); assert arr and all(s['exact'] for s in arr)"
}

@test "solvers --heuristic --output-format=json excludes exact solvers" {
    run "$BIN" solvers --heuristic --output-format=json
    [ "$status" -eq 0 ]
    echo "$output" | python3 -c "import sys,json; arr=json.load(sys.stdin); assert not any(s['exact'] for s in arr)"
}

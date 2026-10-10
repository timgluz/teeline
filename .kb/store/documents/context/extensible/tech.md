---
id: 019e2683-389f-78b2-9ffa-11f7e331658f
slug: context/extensible/tech
title: "Tech Context"
type: context
status: active
priority: medium
---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust (edition 2021) |
| Build | Cargo |
| CLI parsing | clap 4.6 |
| Visualisation | piston + piston_window |
| Randomness | rand 0.9 |
| Text parsing | regex 1.12 |
| CI | GitHub Actions (`rust.yml`) |
| Linting | Clippy (`-D warnings` enforced in CI) |

## Development Commands

```bash
cargo build           # debug
cargo build --release # optimised
cargo test            # all tests
cargo clippy -- -D warnings  # linting (must pass clean)
./target/debug/bin nn -i data/tsplib/berlin52.tsp
cat data/tsplib/berlin52.tsp | ./target/debug/bin sa
```

## Key Technical Constraints

- Piston requires the **main thread** for its event loop — the solver always runs in a background thread
- `--disable_progress` flag skips the Piston window (required for CI / headless environments)
- TSPLIB parser normalises keys to uppercase, lowercases metadata values; coordinates stored as `f32`
- City IDs in TSPLIB are 1-based

## External References

- Concorde TSP source: cloned at `../concorde` (academic-use licence; reference only)
- TSPLIB benchmark data: `data/tsplib/` (berlin52, att48, bayg29, ali535, att532, etc.)
- Known optima: berlin52=7542, att48=10628, bayg29=1610, a280=2579

## Planned Dependencies (not yet added)

- `z3 = "0.12"` — for Z3 exact reference solver (issue #46)

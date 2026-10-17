---
id: 01a12651-55eb-7e41-9ab3-08235b16bfab
slug: tasks/bench-platoo-epochs-help-name
title: "fix(cli): correct --platoo_epochs help text and naming"
type: task
status: completed
priority: medium
tags: [cli, benchmarks, ux]
blocked_by: [tasks/bench-sa-honours-epoch-budget]
---

## Overview

Two related defects in the `platoo_epochs` option, found while planning benchmarking:

1. ~~**Help text is wrong.**~~ **Corrected on investigation — this defect did not exist.**
   `--help` printed `--platoo_epochs`, matching the parser exactly, so the help output was
   copy-pasteable. What was actually wrong: two *error messages* formatted `--platoo-epochs`
   (hyphen) when reporting a bad value, and the parser rejects that spelling — so following the
   error produced `unexpected argument '--platoo-epochs' found`.
2. **The name is misspelled.** "platoo" should be "plateau". It is a public CLI flag, so
   renaming needs a deprecation path rather than a silent change.
3. **The config key accepted only the misspelling** (found on investigation). All seven TOML
   match arms recognised `platoo_epochs` only, and the unknown-field error listed the
   misspelling as the sole valid option — so a config written with the correct spelling failed.

## Goals

- Make the documented flag name match the accepted one
- Decide the rename strategy: alias the corrected spelling and keep the old one working, or
  keep the misspelling and fix only the help text
- Keep `--epochs` / `platoo_epochs` semantics consistent with the plateau work

## Acceptance Criteria

- [x] Help output and parser agree; copy-pasting the documented flag works
- [x] Rename strategy: CLI defines `--plateau_epochs`, keeps `--platoo_epochs` as a hidden alias
- [x] Alias covered by tests asserting both spellings parse
- [x] TOML key accepts either spelling; "valid fields" hints name the correct one
- [x] `cargo test` (15 suites), `cargo clippy --workspace -- -D warnings`, fmt clean; web 486 tests
- [x] `docs/algorithms/lin-kernighan.md` updated with the flag and the alias

## Notes

Smallest of the three benchmarking prerequisites; safe to fold into the plateau task if the
rename lands there instead. Created separately so it is not lost.

## Completion Evidence

- **PR**: [#563](https://github.com/timgluz/teeline/pull/563)
- **Tests written first**: the corrected-flag parse test failed with
  `unexpected argument '--plateau_epochs' found` before the fix.
- **Scope decision**: the field stays named `platoo_epochs` internally — a full rename is 78
  references across 9 files with no user-visible benefit. Only the user-facing surface changed.
- Verified end to end: `solve --help` shows `--plateau_epochs`; both spellings accepted; both
  config-key spellings parse; the two bogus error messages now name the real flag.

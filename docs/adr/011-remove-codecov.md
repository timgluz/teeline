# ADR-011: Remove Codecov from CI

## Status

Accepted

## Context

Codecov's project status compares the whole-project coverage percentage against the base
commit. On docs-only and chore PRs that percentage drifts by 0.1–0.2% even while the patch
status reports "Coverage not affected", so `codecov/project` fails for changes that cannot alter
code coverage. The 0.1% threshold in `codecov.yml` did not absorb the observed 0.13–0.16%
drift. Raising the threshold repeatedly would hide real small regressions, and keeping the
upload adds an external service and a repository secret to every CI run.

## Decision

Remove the `codecov/codecov-action` uploads and `codecov.yml` from CI. The Rust and API
workflows keep build, test, and clippy as their gates, but no longer install `cargo-llvm-cov`
or upload an LCOV report. `cargo-llvm-cov` remains available locally for ad-hoc coverage
measurement; ADR-003's measurement tool is retained, while its CI upload and status gate are
retired.

## Alternatives considered

- **Raise the project threshold.** The drift is baseline-dependent rather than a fixed offset,
  and a threshold wide enough to absorb it would also hide real small regressions.
- **Mark `codecov/project` as informational.** Codecov statuses would still appear on every
  PR, and failed checks would remain noisy even when nothing is wrong.
- **Keep the uploads and ignore failures.** This is exactly the failed-pipeline noise the
  change removes.

## Consequences

There is no external coverage history or PR annotation. Contributors measure coverage on demand
with `cargo llvm-cov`; CI continues to enforce tests and clippy. Reintroducing a coverage
service requires a new ADR with an explicit threshold model.

# Architecture Decision Records

Architecture Decision Records (ADRs) capture durable, dated decisions and the reasoning
behind them. They are not a changelog: an accepted ADR is history and is not rewritten.
When a decision changes, add a new ADR and mark the old one `Superseded by ADR-NNN`.

The project constitution ([`constitution.md`](../../constitution.md)) states the
principles every decision must respect; an ADR must not contradict it.

## Index

| ADR | Decision | Status |
| --- | --- | --- |
| [001](001-tracing.md) | Structured logging via `tracing` to stderr | Accepted |
| [002](002-taskfile.md) | Taskfile as task runner | Accepted |
| [003](003-coverage.md) | Test coverage via `cargo-llvm-cov` | Accepted |
| [004](004-egui-migration.md) | Replace `piston_window` with egui/eframe | Superseded by 008 |
| [005](005-validation-approach.md) | Hand-rolled validation over `garde` | Accepted |
| [006](006-teeline-gui-extraction.md) | Extract eframe GUI into `teeline-gui` | Superseded by 008 |
| [007](007-webauthn-auth.md) | Replace Clerk with WebAuthn + self-serve API keys | Accepted |
| [008](008-qt-gui.md) | Qt 6/QML desktop GUI in `teeline-qt` | Accepted |
| [009](009-wasm-component-model.md) | WebAssembly Component Model for solver exposure | Accepted |

## Adding an ADR

1. Take the next number; do not reuse or renumber.
2. Use the structure below.
3. Update the index.
4. If it replaces a decision, mark the old ADR `Superseded by ADR-NNN` and link forward.

```markdown
# ADR-NNN: Title

## Status

Proposed | Accepted | Superseded by ADR-NNN

## Context

What forced the decision.

## Decision

What we chose, and the constraints it imposes.

## Alternatives considered

What was rejected, and why.

## Consequences

What becomes easier or harder; what must now stay true.
```

---
id: 019e5e39-0e33-7a42-a8f5-7714e1db8a6f
slug: decisions/adr-005-validation-approach
title: "ADR-005: Hand-rolled validation over garde for solver options"
type: brief
status: active
tags: [validation, architecture, dependencies]
---

## Context

Issue #86 called for moving validation off the old monolithic `validate_options` function in
`config.rs` and onto the solver-specific option structs introduced in #87 (`SAOptions`,
`GAOptions`, `CSOptions`, `FPAOptions`, `HeuristicOptions`). The issue explicitly named
[`garde`](https://crates.io/crates/garde) as the candidate validation crate and asked for the
trade-off to be re-evaluated once the structs existed.

The structs each have 2–4 fields requiring range checks:

| Struct | Fields validated |
|---|---|
| `HeuristicOptions` | `n_nearest >= 1` |
| `SAOptions` | `cooling_rate ∈ (0,1)`, `max_temperature > 0`, `min_temperature >= 0`, `min_temperature < max_temperature` |
| `GAOptions` | `mutation_probability ∈ [0,1]` |
| `CSOptions` | `mutation_probability ∈ [0,1]` |
| `FPAOptions` | `mutation_probability ∈ [0,1]` |

## Decision

Use hand-rolled `pub fn validate(&self) -> Result<(), String>` methods on each struct.
Do **not** add `garde` (or any other validation crate) as a dependency.

## Alternatives Considered

**`garde` derive macro**
- Pros: validation rules declared at field site; no boilerplate `if` chains; cross-field rules
  are possible via custom validators.
- Cons:
  - Proc-macro dependency increases compile times and adds a transitive dependency tree.
  - `garde`'s `range` is inclusive (`min..=max`); `cooling_rate` requires an exclusive open
    interval `(0, 1)`, so the two SA range checks still need `#[garde(custom(...))]` — the
    derive saves nothing for the most complex case.
  - Cross-field rule `min_temperature < max_temperature` requires a custom validator that
    receives `&Self`, adding the same amount of code as the hand-rolled version.
  - `progress_tx` and similar runtime fields on wrapper types would need `#[garde(skip)]`
    annotations, coupling the derive to runtime concerns.
  - Issue #86 explicitly flagged garde as worth avoiding if it slows the build, which it would.

**Hand-rolled `validate()` per struct** _(chosen)_
- Each struct has ≤ 5 fields; validation is 3–5 `if` guards and one `Ok(())`.
- Zero new dependencies; no proc-macro overhead.
- Error messages name the field and the constraint in plain English — no formatting ceremony.
- `validate()` is called at the end of both `from_toml()` and `from_cli()`, giving a single
  consistent validation point regardless of input source.
- Each solver's `validate()` delegates to `self.heuristic.validate()?` first, so the whole
  struct tree is validated with one call.

## Implementation

`src/tsp/mod.rs` — PR #89 (issue #86):

- `HeuristicOptions::validate()` — rejects `n_nearest == 0`.
  Note: `epochs = 0` and `platoo_epochs = 0` are **intentional sentinels** (`epochs = 0` means
  "run until temperature stops"; `platoo_epochs = 0` disables plateau restarts) — they are
  explicitly **not** validated.
- `SAOptions::validate()`, `GAOptions::validate()`, `CSOptions::validate()`,
  `FPAOptions::validate()` — each delegates to `self.heuristic.validate()?` then checks its
  own float fields.
- All `from_cli()` methods changed from `Self` → `Result<Self, String>`; invalid parses now
  return `Err` with the flag name instead of silently falling back to defaults.
- `HeuristicOptions::from_toml()` now calls `self.validate()?` (was type-checking only).

## Consequences

- No new crate dependencies; build time unchanged.
- Adding a new solver with ≤ 5 option fields: write one `validate()` method of ~10 lines.
- If a future solver accumulates many cross-field constraints, reconsider `garde` at that point —
  the decision boundary is roughly "more than 3 custom validators" or "more than 8 fields".
- Validation is symmetric across TOML and CLI paths; both return `Result<_, String>` and call
  the same `validate()` method.

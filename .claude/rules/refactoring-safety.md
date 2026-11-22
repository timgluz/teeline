# Refactoring Safety

Before making changes that could break callers, check the blast radius with the project
knowledge graph before editing.

## Required checks

| Before doing this | Run this first |
| --- | --- |
| Changing a function signature | `graphify query "<symbol> callers"` |
| Renaming a public symbol | `graphify query "<symbol>"` |
| Modifying an interface or type's fields | `graphify query "<type> usages"` |
| Deleting a symbol | `graphify query "<symbol>"` or `graphify explain "<concept>"` |
| Changing a file's public API | `graphify query "<file> dependents"` |

If `graphify-out/graph.json` is missing, fall back to `rg`/`grep` for call sites and run
`graphify update .` afterwards.

## Risk assessment

| Callers | Risk | Action |
| --- | --- | --- |
| 0-2 callers, same module | Low | Proceed |
| 3-10 callers, multiple modules | Medium | Update all call sites, run tests |
| 10+ callers or public API | High | Confirm with the user before proceeding |

## Workflow

1. Check callers and dependents with `graphify query`.
2. List the files that need changes.
3. Update leaf callers first.
4. Update tests alongside production code.
5. Run `cargo test` and `cargo clippy -- -D warnings`.

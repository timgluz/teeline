---
id: 01a12651-5672-75e3-9298-3919b51d636a
slug: tasks/bench-single-solver-skill
title: "feat(kb): benchmark-solver skill for differential single-algorithm runs"
type: task
status: draft
priority: medium
tags: [benchmarks, tooling, skill]
blocked_by: [tasks/bench-publish-provenance]
---

## Overview

The agreed cadence is manual and differential: at each release, re-run only the algorithms that
changed, keeping older results for regression checking. That workflow is repetitive enough to
warrant a project-local skill that measures one algorithm and uploads the new version to R2.

Precedent for layout: `.agents/skills/open-code-review/SKILL.md` (tracked, one `SKILL.md` per
directory).

## Goals

- `benchmark-solver` skill that takes an algorithm id and does the whole loop
- Safe by construction: refuse to publish a single-solver re-run when shared code has changed,
  because that would mix incomparable numbers under one version label

## Steps the skill performs

1. Build the release binary
2. Run the harness filtered to the named solver (`--solver <id>`)
3. Fold the new rows into the current state
4. Publish to R2
5. Verify what is actually live with `scripts/read-benchmarks.sh`

## Safety requirement

Before publishing, compare the solvers' recorded measurement commits against the current code.
If shared code — `distance_matrix`, `kdtree`, `tsplib`, `comparison`, `mod`, `pipeline`,
`graph` — has changed since the other solvers were measured, the single-solver result is not
comparable to theirs and the skill must stop and explain why rather than publishing.

Without this check the skill is a fast route to publishing internally inconsistent data.

## Acceptance Criteria

- [ ] `.agents/skills/benchmark-solver/SKILL.md` exists with valid frontmatter (name, description)
- [ ] Skill documents the exact commands for each of the five steps
- [ ] Shared-code staleness check implemented and documented, with its failure message
- [ ] Skill refuses (not merely warns) on stale shared code, or escalates for a decision
- [ ] A successful single-solver run updates only that solver's shard and provenance
- [ ] Verification step reads back the live data and confirms the new version
- [ ] Skill works for an algorithm with no prior published results

## Notes

Depends on the harness (`--solver`) and on the publish step existing with per-solver provenance.
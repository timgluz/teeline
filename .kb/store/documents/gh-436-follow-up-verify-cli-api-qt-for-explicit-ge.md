---
id: 019fe69a-73aa-7682-af0f-11d9b21a4464
slug: gh-436-follow-up-verify-cli-api-qt-for-explicit-ge
title: "GH #436 follow-up: Verify CLI/API/QT for EXPLICIT/GEO (no changes needed)"
type: task
status: completed
priority: low
---

## Summary

CLI, API, and QT paths already use `TspLibData::distance_matrix()` and are correct. No changes needed for EXPLICIT/GEO distance support — they were already handled before this PR.

## Scope

Nothing to do. All three paths verified correct:
- **CLI** (`teeline-cli/src/main.rs`): uses `tsp_data.distance_matrix()`
- **API** (`teeline-api/src/services/tsp_service.rs`): uses `data.distance_matrix()`
- **QT** (`teeline-qt/src/solver_engine.rs`): uses `data.distance_matrix()`

The `TspLibData::distance_matrix()` method already correctly routes EXPLICIT through pre-computed distances and GEO through `geo_distance()`.
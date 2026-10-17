---
id: 01a12b79-88a3-7420-832a-f48fedf90677
slug: tasks/problem-minimaps-as-r2-assets
title: "feat(web): serve problem minimaps from R2 instead of inlining them in the docs"
type: task
status: completed
priority: high
tags: [teeline-web, docs, tooling, r2]
---

## Overview

`docs/problems/*.md` currently embed each instance's minimap as inline `<svg>`, so the committed
markdown is **19.6 MB** and the largest page (`pla85900`) was **6.2 MB** of `<circle>` markup. Any
change that regenerates them produces a diff of tens of thousands of lines: PR #581 is
+43,370/−223,321 across 36 files, almost all of it SVG blobs, and the interesting part (the layout
logic) is a few hundred lines. The same 19.6 MB also means every regeneration rewrites 111 files,
so two agents touching the corpus cannot merge cleanly.

The maps belong in the object store beside the datasets, not in the review diff: generate them as
**one file per instance**, upload to the R2 bucket behind `static.tspsolver.com`, and have the page
reference a URL. The pages then carry a single frontmatter line each, and the assets are fetched
separately — cached, lazy-loaded, and no longer inflating the HTML of every visit.

## Goals

- `docs/problems/{id}.md` contains a *reference* to its minimap, not the map itself.
- The map pipeline is reproducible and verifiable offline: generate → `--dry-run` → upload.
- A regeneration is a reviewable diff (one line per page), not a 223k-line deletion.
- Nothing regresses visually: the 94 existing maps and the 17 that #581 added all still appear.

## Design

**Assets.** `scripts/gen-problem-minimaps.mjs` writes one SVG per instance to a staging directory
(`build/minimaps/`, git-ignored), reusing the layout logic from #581: `NODE_COORD_SECTION`, then
`DISPLAY_DATA_SECTION`, then classical MDS for matrix-only instances, with the density grid above
3,000 cities. Public URL: `https://static.tspsolver.com/minimaps/v1/{id}.svg`. Deliberately *not* under the
`tsplib/` dataset prefix — images and the instances they draw should never share a listing — and
versioned so a rendering change is a prefix bump rather than a cache purge.

SVG rather than PNG because it needs no rasteriser in the pipeline (the density grid already bounds
the worst case) and stays crisp at any DPI. Social cards need a raster format, so if og:image
matters later, that is a separate pass using the Playwright already in the dev dependencies.

**Publishing.** `scripts/publish-minimaps.sh`, modelled on `publish-benchmarks.sh`:

- `--dry-run` prints the exact `would put <prefix>/<key>` lines, which is the seam its tests assert
  on — no credentials, no network.
- `--bucket` / `CF_R2_BUCKET`, `--public-base`, `--prefix` (default `minimaps/v1`).
- `--only-changed` via a published `index.json` of `{id: {sha256, bytes, derived, cities}}`, so a
  re-run uploads only what changed, and the manifest is also what documents the published set.
- Prefers `cf r2 objects put`, falls back to `wrangler r2 object put`, and must pass
  `--content-type image/svg+xml` — `r2_put` in `publish-benchmarks.sh` hardcodes
  `application/json`, so the client selection moves to a shared `scripts/lib/r2.sh` that takes the
  content type, rather than becoming a third copy of the `cf`/`wrangler` branch.
- A versioned prefix, so tuning the rendering is a prefix bump rather than a cache purge. No
  `Cache-Control` header: `cf r2 objects put` has no flag for one and only `wrangler` does, so
  setting it on one client path would make the same publish cache differently per machine.

**Pages.** `docs/problems/*.md` frontmatter gains `minimapUrl` (and `minimapDerived: true` for the
MDS layouts). `src/pages/problems/[id]/index.astro` renders
`<img src={data.minimapUrl} width="340" height="200" loading="lazy" decoding="async" alt="…">`,
keeps the existing visible note for derived layouts, and adds the URL to the `Dataset` JSON-LD as
`image`. Deriving the URL from `data.id` in the template was rejected: the field makes the page's
dependency on a published object explicit, and lets a page ship without a map.

**The bulk deletion is quarantined.** Removing the inline SVGs from 94 pages is unavoidable once the
pages reference assets — 19.6 MB of committed markup cannot leave the tree without a large deletion
diff — but it is mechanical and self-verifying. It lands as its own commit touching only
`docs/problems/*.md`, so the code review is the small diff, and the deletion commit is checked by
assertion rather than by reading: every hunk is a removed `<circle>`/`<rect>` run,
`grep -c '<svg' docs/problems/*.md` is 0 afterwards, and the directory drops from 19.6 MB to under
200 KB. It can be merged on its own, ahead of or behind the code.

## Relationship to #581

#581 inlines the maps and regenerates all 111 pages, which is the diff that prompted this. Its
*layout logic* is what this task needs — `DISPLAY_DATA_SECTION` support, MDS for the 12 matrix-only
instances with the derived-layout label, and the density grid above 3,000 cities — so the port is
mostly moving that code from "build a string" to "write a file". #581 should be closed rather than
merged, with this task superseding it.

## Acceptance Criteria

- [x] `node scripts/gen-problem-minimaps.mjs` writes 111 SVGs to the staging dir (`build/minimaps/`,
      already git-ignored) and nothing into `docs/problems/`.
- [x] All 111 pages render a minimap from `static.tspsolver.com`; the 12 derived layouts carry the
      visible note and a distinct `alt`.
- [x] The maps that exist today are visually unchanged — proved by pixel comparison rather than by
      eye: master's inline SVG and the generated file rendered side by side at 340x200 differ in
      **0 pixels** (`berlin52`, `att48`, `ali535`, `eil51`, `a280`, `bier127`, `ch150`, `pcb442`).
- [x] `--dry-run` lists 111 keys under `tsplib/minimaps/v1/` plus `index.json`, each with its
      content type, and needs no credentials.
- [x] BATS coverage for the uploader (12 tests): dry-run key set and content types, nothing uploaded
      when the index matches, only changed objects, `--only`, an unreadable published index, a
      tampered SVG refused against its index hash, a missing bucket, the exact `cf` argv including
      `image/svg+xml`, the `wrangler` fallback, and both `--verify` paths.
- [x] `publish-benchmarks.sh` is unchanged in behaviour after the client moved to
      `scripts/lib/r2.sh` — its 38 BATS tests pass.
- [x] An e2e assertion (3 tests, both browser projects) that the `<img>` points at the published
      base URL, has dimensions, lazy loading and a real `alt`; plus the derived caption and the
      JSON-LD `image`. Not run by CI: it runs only `test:e2e:auth`
      ([[tasks/ci-runs-only-auth-e2e]]).
- [x] A read-back verification: `--verify` probes every expected object over its public URL and
      exits non-zero listing the missing ones. It reads the CDN rather than the bucket listing
      (`read-benchmarks.sh` keeps the `cf r2 objects list` form) because the CDN is what a page
      actually fetches.
- [x] `docs/problems/` drops from 19.6 MB to **58 KB** (measured in bytes; `du` reports 444 KB
      because 111 small files each round up to a 4 KB block). The code commit is 1,224 insertions /
      95 deletions — 739 source and 485 tests. The "~600 lines" estimate covered the source and
      ignored the coverage this task asked for.
- [x] The deletion commit is verifiable without reading it: only `docs/problems/*.md`, no `<svg`
      remains, 111 pages gain exactly the two minimap fields and lose their bodies.

## Not done

- ~~The upload itself has not been run.~~ **Published 2026-10-11** to the `teeline` bucket (the one
  behind `static.tspsolver.com`, confirmed by the `tsplib/` and `bench/v2/` keys it holds): 112
  objects, 6.5 MB, in 1m55s. `--verify` reads all 112 back, and the served content types are
  `image/svg+xml` and `application/json` — the bug the shared client was extracted to fix. The
  built pages load the real 340x200 images from the CDN, including the derived `gr48` and the
  density-grid `pla85900`. Moved afterwards to the bucket root (`minimaps/v1/`) at the author's
  request, away from the `tsplib/` dataset prefix: republished as 112 objects and the 112 under
  `tsplib/minimaps/v1/` deleted, leaving `tsplib/` with only instances and tours.
- No `Cache-Control` is set on the objects, contrary to the plan above: `cf r2 objects put` has no
  flag for one and only `wrangler` does, so setting it on one client path would make the same
  publish cache differently per machine. The versioned prefix carries the correctness; a Cache Rule
  can set the TTL.
- ~~No PNG/`og:image` pass.~~ **Split into [[tasks/problem-social-cards]]** (PR #585), because it
  turned out to be its own deliverable — its own script, artifacts and review. Summary of what it
  did:
  `teeline-web/scripts/gen-og-images.mjs` renders each map to a 1200x630 PNG with Playwright (111
  cards, 5.55 MB, largest 142 KB), `BaseLayout` takes a per-page `og:image`, and the problem pages
  pass their card and advertise it in the Dataset JSON-LD — social clients and structured-data
  consumers reject SVG, so the page's own image was never usable for either. Publishing them
  uploaded 111 objects and skipped 111 unchanged maps, and `--verify` reads all 223 back.
  This also fixed a real defect in the publisher: the unit of upload had been the *instance*, so a
  changed card would have dragged an unchanged map along with it.
- `scripts/gen-problem-pages.mjs` and the module read all nine `EDGE_WEIGHT_FORMAT` layouts, so the
  page generator no longer disagrees with the Rust parser after
  [[tasks/tsplib-all-edge-weight-formats]] — that follow-up is closed here.

## Context

- R2 mechanism to reuse: `scripts/publish-benchmarks.sh` — `r2_put` (`:173`), client selection
  (`:166`), `PREFIX="bench/v2"` (`:31`), `PUBLIC_BASE` (`:38`), dry-run output (`:1078`).
- Read-back precedent: `scripts/read-benchmarks.sh` (`cf r2 objects get|list`).
- The layout logic to port lives in `scripts/gen-problem-pages.mjs` on #581.
- Supersedes #581. Relates to [[tasks/bench-publish-provenance]] for the publish/dry-run pattern.

## Open questions

- Whether to also publish a PNG per instance for `og:image`, which needs a rasteriser
  (Playwright is already a dev dependency) — deferred unless social previews matter.
- Whether an unpublished asset should fail the web build. Simplest is a `--check` mode that HEADs
  the 111 URLs and reports drift; a build-time hard failure would couple deploys to R2.

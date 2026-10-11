---
id: 01a12bc5-28ba-75d1-9113-8a7b81cf2534
slug: tasks/problem-social-cards
title: "feat(web): give every problem page a social card"
type: task
status: completed
priority: medium
tags: [teeline-web, docs, seo]
---

## Overview

A shared problem page rendered as a bare title and description. The site had exactly one
`og:image` — a brand image in `BaseLayout` — so every problem page advertised the same picture, and
structured-data consumers reject SVG, so the map the page itself renders was not usable as its
`image` either.

This is the deferred item from [[tasks/problem-minimaps-as-r2-assets]], split out once it was clear
it is its own deliverable (its own script, its own artifacts, its own PR).

## What it does

- `teeline-web/scripts/gen-og-images.mjs` (`npm run og:images`) renders each staged minimap SVG into
  a **1200x630 PNG** with Playwright — already a dev dependency of `teeline-web`, which is why the
  script lives there rather than in the dependency-free root `scripts/`. The SVG is inlined into the
  page instead of referenced, so no `file://` resource policy is involved, and the hashes are
  recorded in the same `index.json` the publisher reads.
- `BaseLayout` takes an optional `og:image` with alt and dimensions; `DocsLayout` forwards it;
  problem pages pass their card and advertise it as `image` in the `Dataset` JSON-LD. `twitter:card`
  follows the image — `summary_large_image` for a per-page card, `summary` for the site default. A
  page that passes nothing is unchanged.
- `publish-minimaps.sh` uploads per **object** rather than per instance, so a changed card no longer
  drags its unchanged map along. That was a real defect in the previous publisher, found by writing
  the test for it.

## Acceptance Criteria

- [x] Every problem page points `og:image`/`twitter:image` at its own 1200x630 card, with an alt
- [x] The `Dataset` JSON-LD `image` is the raster, not the SVG
- [x] Pages that pass no card keep the site default card and `summary` type
- [x] The publisher treats each artifact as its own object, and `--verify` covers both
- [x] Content types are per artifact (`image/svg+xml`, `image/png`)
- [x] 111 cards rendered (5.55 MB, largest 142 KB), published, and read back

## Verification

- Publishing showed the comparison working: **222 objects staged, 111 uploaded, 111 unchanged** —
  the 111 unchanged maps were skipped while the 111 new cards went up.
- `--verify` reads all **223 objects** back; the served card is a genuine PNG
  (`image/png`, magic bytes ok, 1200x630).
- **14 BATS tests** (2 new): the PNG uploads with its own content type, a changed raster alone
  re-uploads only that object, and `--verify` covers both artifacts.
- **10 Playwright tests** (new over both browser projects): `og:image`, dimensions, alt,
  `summary_large_image`, the JSON-LD raster, and `/problems/` keeping the site default.
- 588 vitest, `npm run build` 0 errors.

## Notes

- The card is the map alone — no text or branding. Social clients show the page's title and
  description beside it, so text on the image would be redundant, and leaving it out keeps the
  render free of font loading. A branded variant is possible.
- Playwright screenshots are deterministic for identical input on the same browser version, which is
  what lets the publisher compare hashes. A browser upgrade changes the bytes and re-uploads the
  cards; harmless, and the reason the prefix is versioned.

## Progress Log

### 2026-10-11 — implemented on `feat/problem-social-cards` (PR #585)

- Branched from the merge of #584 (`674ccec3`), after #584's own branch was closed by the merge.
- Cards are live under `minimaps/v1/og/{id}.png` and verified by reading all 223 objects back.

### 2026-10-11 — merged as #585 (`b79b7c36`); follow-ups in #588

An `ocr` review of the merged commit found one real hole and several smaller ones, all in the
pipeline rather than the cards themselves: the pages linked `og/<id>.png` while nothing verified the
card existed (`--verify` enumerated the index, not the pages, so a skipped card step published 111
404s behind a green verify), `jq`'s `add` on an empty index aborted the publisher's arithmetic under
`set -u`, and a page without a card advertised dimensions and an alt for an image it did not have.
[[tasks/minimap-pipeline-guards]] carries the fixes.

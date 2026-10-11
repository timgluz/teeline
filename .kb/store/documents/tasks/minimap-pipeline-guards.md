---
id: 01a12be6-bd6e-7ac0-8772-9b26bd6d3250
slug: tasks/minimap-pipeline-guards
title: "fix(web): do not publish a card link the bucket cannot serve"
type: task
status: completed
priority: medium
tags: [teeline-web, scripts, publishing, seo]
---

## Overview

An `ocr` review of the merged [[tasks/problem-social-cards]] (#585) found that the pipeline could
publish pages pointing at social cards that were never uploaded, and could report that as success.

Every problem page links `og/<id>.png` in its frontmatter, and no step could check that the card
existed — the cards are rendered *after* the pages are written, so the page cannot know, and the
publisher only looked at its own index. `publish-minimaps.sh --verify` enumerated that index, so
skipping the card step (`npm --prefix teeline-web run og:images`) left an index of maps only: 111
`og:image` URLs would 404 while `--verify` reported everything green.

The review found several smaller things in the same pipeline, all fixed here because they are the
same class of problem: a check that cannot fail, or a value that is only correct by accident.

## Acceptance Criteria

- [x] Publishing, and verifying, refuses when a page links a card the index has no `og` entry for,
      naming the ids and the command that renders them (`DOCS_DIR`, default `docs/problems`)
- [x] A `DOCS_DIR` that does not exist skips the check, so fixture-based tests can stage a bare index
- [x] An empty index reports zero objects instead of aborting the arithmetic (`jq`'s `add` is `null`)
- [x] `--verify` and the upload list share one key enumeration, so they cannot disagree about what
      exists
- [x] The per-object hash lookup binds the id with `--arg` rather than interpolating it into the jq
      program
- [x] `gen-og-images.mjs` imports the declared `@playwright/test`, closes the browser in a `finally`,
      and names a missing staged SVG
- [x] A page with no card passes no card metadata: no alt for an image that is not shown, no
      1200x630 dimensions for the 540x630 brand fallback, and no SVG in `Dataset.image`
- [x] `BaseLayout`'s 540x630 defaults apply only to the brand card they describe
- [x] The cards spec's fallback SVG has an intrinsic size, so its rendered-size assertion does not
      depend on local staging

## What the review got wrong, recorded so it is not "fixed" later

- "`tests/e2e/publish-minimaps.bats` is not updated" — it was, by +44 lines in the same commit,
  including the per-object cases the review asked for. The one real gap it named, an index with no
  `og` entries, is covered now.
- "`geoDistance`/`Math.round` is a no-op" — true for integer metrics; the part that mattered was a
  non-finite total reaching a page, which returns `null` now.

## Verification

- **17 BATS** (3 new): the refusal on the publish path *and* on the credential-free verify path, a
  page whose card is in the index passing, and the empty index.
- **588 vitest**, `tsc` clean, `astro check` 0 errors, **165-page build**.
- **5/5 cards specs**; the non-auth suite is 72/75 locally, where the three WebMCP failures are the
  missing jco-transpiled bindings — they fail identically with these changes stashed, and pass in CI
  (`e2e-web` green on #586, which now runs this spec).

## Progress Log

### 2026-10-11 — implemented on `fix/minimap-pipeline-guards` (PR #588)

- Branched from `b79b7c36` (#585), the commit the review covered.
- One thing found while writing the tests: the cards spec passed in CI and failed locally against the
  same stub, because its fallback SVG had no intrinsic size and Chrome then reports the *rendered*
  width (255 in a narrower layout, 340 otherwise). The stub states its size now.
- Status is `review` rather than `completed`: the criteria are met on the branch, not on master.

## Notes

- The guard reads the docs because they are the only declaration of what a page links. A cleaner
  contract would be an index that cannot lack a card — but the pages are generated before the cards
  exist, so the declaration has to come from the pages.
- `--verify` is the credential-free path a maintainer runs after a publish; it is the one that
  reported success over 404s, so it is guarded too.

### 2026-10-11 — merged

PR #588 merged as `dd0ee444`, with every check green at its head: `build`, `ci`, `e2e`, `e2e-web`
(1m31s — the non-auth suite on a fresh checkout, which is the condition this task exists for) and
both codecov gates. The cards are live and verified under `minimaps/v1/og/`.

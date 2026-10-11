---
id: 01a12b16-2fd0-7f11-abac-72499916b64e
slug: tasks/bench-data-download
title: "feat(web): offer the published benchmark data for download"
type: task
status: draft
priority: medium
tags: [benchmarks, teeline-web, data]
---

## Overview

The published data is public but effectively undiscoverable: a reader who wants to check a number has
to know the shards live under `https://static.tspsolver.com/bench/v2/` and guess the filename. The
pages that display the numbers should offer them.

This pairs with [[tasks/blog-benchmarks-duckdb]] — the post's first step is "get the data", which is
exactly this link.

## What to offer

- The page's own shard: `algorithms/<id>.json` on an algorithm page, `problems/<id>.json` on a problem
  page.
- The manifest (`index.json`), which lists what exists and carries the per-family budgets — the entry
  point for anyone fetching the whole tree.
- Optionally the release snapshot a number came from (`snapshots/<tag>/...`): immutable, so it is the
  right thing to cite.

## Details that shape it

- **Cross-origin `download` is ignored.** `static.tspsolver.com` is a different origin, so
  `<a download>` will not force a save — the honest label is "open the JSON", or the file has to be
  proxied same-origin if a real download is wanted. Worth deciding rather than shipping a label the
  browser does not honour.
- **No build-time network.** The URL is deterministic, so the link renders in the static build, which
  keeps the rule that the build works when R2 is unreachable.
- Say what the reader gets: a JSON shard, the contract it follows (`docs/benchmarks/schema.md`), and
  that a 404 means "not published" rather than "empty".

## Acceptance Criteria

- [ ] An algorithm page links to its own shard, and a problem page to its leaderboard shard
- [ ] The manifest is linked somewhere discoverable, with the budgets it carries
- [ ] The label matches what a browser actually does with a cross-origin link
- [ ] No build-time fetch is introduced
- [ ] A test pins the rendered href, so a base-URL change cannot silently break it

## Notes

- `docs/benchmarks/schema.md` is the contract to link alongside the data. It lives in the repo rather
  than on the site, so the link target is GitHub — the Fourier post already links `docs/benchmarks.md`
  that way.

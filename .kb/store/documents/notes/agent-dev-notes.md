---
id: 01a00bed-8240-7f70-8b8c-a080825810e6
slug: notes/agent-dev-notes
title: "Agent dev notes: gotchas, workarounds, reflections"
type: note
status: active
priority: medium
tags: [agent, workflow, gotchas, teeline-web]
---

# Agent dev notes: gotchas, workarounds, reflections

Session notes from the 2026-08-15/16 explainer series (GH #429–#434, PRs #468–#476). These are environment and workflow lessons that keep recurring — read before the next teeline-web session.

## Environment gotchas

### SSH to GitHub is broken on this machine
`/usr/etc/ssh/ssh_config.d/50-suse.conf` is owned by `nobody`, so every `git push/fetch/pull` fails unless you override the config:

```bash
GIT_SSH_COMMAND="ssh -F /dev/null -i /home/timgluz/.ssh/id_ed25519 -o StrictHostKeyChecking=accept-new" git push ...
```

### npm/npx are read-only (EROFS)
`~/.npm/_cacache` is on a read-only FS — **never use `npx`**. Always call the local binaries directly:

```bash
./node_modules/.bin/vitest run
./node_modules/.bin/tsc --noEmit -p teeline-web/tsconfig.json
./node_modules/.bin/astro check
./node_modules/.bin/playwright test
```

### `/tmp` is not persistent across tool calls
Any temp file written to `/tmp` is gone on the next command. Use a **repo-local** temp dir (e.g. `.tmp-bhkwork/`) and delete it when done. Note repo-local dirs must be gitignored or cleaned before committing.

### Playwright browsers
The bundled Chromium is not installed system-wide, and `~/.cache/ms-playwright` is read-only. Install into the repo and point Playwright at it:

```bash
PLAYWRIGHT_BROWSERS_PATH=/home/timgluz/repos/teeline/.pw-browsers ./node_modules/.bin/playwright install chromium
# then run e2e with the same env var:
PLAYWRIGHT_BROWSERS_PATH=/home/timgluz/repos/teeline/.pw-browsers ./node_modules/.bin/playwright test --project=chrome
```

`.pw-browsers/` is gitignored (commit `615d4a2`). System Chrome (`/usr/bin/google-chrome`) via `--project=chrome` is the fallback. `webmcp.spec.ts` needs a **production build** (WASM lives in `dist/`, not served by `astro dev`).

### `git rebase --continue` hangs on vim
Use `GIT_EDITOR=true git rebase --continue`. For empty commits after conflict resolution, `git rebase --skip`.

### Astro dev auto-daemonizes inside agent harnesses
`astro dev` detects "running in an agent" and backgrounds itself, which breaks Playwright's `webServer` lifecycle (`Process from config.webServer exited early`). Fix: `env: { ASTRO_DEV_BACKGROUND: 'false' }` in `playwright.config.ts` — any truthy value suppresses the daemonize heuristic.

## Git workflow gotchas

### Verify the branch before committing (learned the hard way — 3×)
`git add -A` on a **stacked branch** repeatedly staged changes into the wrong branch. Always check `git branch --show-current` before `git add`/`commit`. Untangling required `git reset --soft`, `git stash push/pop`, `git show <sha>:<path>` extraction, cherry-picks, and force-pushes — avoidable with a 1-second check.

### Stacked-branch squash absorption
Merging a stacked PR (e.g. #475) squash-merges **all** its parent content (e.g. #473 menubar, #474 B&B) into master. Later stacked branches then show empty/partial diffs. Resolution: rebase each branch onto master taking `--ours` **for every conflict** (in a rebase, `--ours` = base/master, `--theirs` = your replayed commits — **inverted vs. a merge**), then force-push. When a PR shows an unexpectedly empty diff, check whether its content was already absorbed this way before reopening it.

### `export { x } from './mod'` does not create local bindings
Re-exporting without importing gives TS2304 when the module body references `x`. Use `import { x } from './mod'` **plus** `export { x }`.

## Simulation / test gotchas (explainer pattern)

- **`structuredClone` can't clone functions** → a `SimState` must not hold closures (e.g. no prebound `dist` in state). Strip functions before `toEqual` in vitest; compare data-only.
- **Closed cycles**: TSP results must be closed tours. The B&B search was already correct (`bestTour` closed, cost includes the return edge) but the leaf display printed an open path — fixed with `const closed = [...path, state.startCity].join('→')`. Whenever an explainer prints/claims a "tour", assert it is closed.
- **e2e hydration**: explainer islands hydrate asynchronously; probe with a `waitHydrated` retry-loop (scroll inside `toPass`). `getByRole('button', { name: 'Back' })` also matched "Read-back" — use `{ name: '⏴ Back' }`.
- **SSR safety**: first render must not index arrays derived from zero-revealed state (`mstEdges[-1]` crash in B&B) — guard on reveal counters.

## Reflections

- **Rust-faithful simulation as the explainer backbone** was the right call: the `*-algo.ts` files mirror move semantics, `-1e-3` thresholds, tie-breaks, and node/leaf logic exactly, so the demos behave like the real solver and are pure/deterministic (seeded RNG) — unit-testable without a DOM.
- **The docs↔nav-data complexity consistency test** (nav-data.test.ts, `import.meta.glob` `?raw` over `docs/algorithms/*.md`) is a cheap guard that caught a real user-reported mismatch (NN page vs table). Worth extending to other frontmatter fields if the docs/nav drift again.
- **Two people committing KB changes on separate lines makes the KB store disagree with itself.**
  The symptom is `git-kb commit` failing with `... has previous_commit_id ... but current document tip
  is ...; stale sibling histories must rebase before storage apply`. What fixed it: `git-kb repair
  projection` (rebuilds the derived SQLite from the canonical file store — its own hint; it then
  reports "SQLite projection matches the canonical file store"), then `git-kb reset <slug>`,
  `git-kb checkout <slug>`, re-apply the edit and commit.
- `git-kb doctor` still warns that three documents need lineage repair
  (`bench-plateau-stop-all-iterative`, `gh372-lk-som-pipeline-toml`, `type-check-the-islands`); two of
  them predate today. Its remedy, `git-kb repair canonicalize --json` then `--apply --yes`, is
  documented as *offline emergency repair, only from the intended local authority copy* — do it
  deliberately with `.kb` backed up, not at the end of a session.
- **A conflicting PR gets no CI at all.** `pull_request` workflows run against `refs/pull/N/merge`,
  which GitHub cannot build while the branch conflicts with its base — so the checks list shows
  nothing, not even a queued job, and pushing again changes nothing. `gh pr view <n> --json
  mergeStateStatus` says `DIRTY`; rebase and the runs appear. Also check `gh api
  repos/OWNER/REPO/actions/runs?branch=<branch>` before blaming the branch: `total_count: 0` with a
  green githubstatus means the events are not reaching Actions at all (an exhausted Actions spending
  limit does this repo-wide, and raising it fixes it immediately).
- **Playwright in every verification plan** (user directive) paid off repeatedly: hydration races, SSR crashes, and the Back-button ambiguity were all found by e2e, not by unit tests or type checks. The `waitHydrated` retry pattern is the key to stable island probes.
- **Small instance sizes are a feature**: 8 cities for stochastic-hill (user: "you can decrease number of cities if epoch takes too long"), 6 for B&B/BHK, ≤10 with brute-forced OPT ratio for Christofides — demos stay snappy and exact algorithms remain legible on screen.
- **`ocr` review loop** (open-code-review, always exits 1 on a benign session-write EROFS — read the output file anyway) caught real issues: shared `explainer-cities.ts` extraction (#471), nested-ternary/style violations, and a stochastic-hill step race. The `--from master --to <branch>` invocation is the reliable shape.

  - Run it **after opening the PR** (user directive), and pass **`--from origin/master`**, not
    `--from master`: local `master` in a worktree is often many commits behind, so the review would
    cover everyone else's merged work as well as yours. Check the preview line (`ocr review
    --preview`) lists only your files before paying for a run.
  - It found real bugs in this repo's own fixes, so treat the output as a review rather than a
    formality: #587's completeness check admitted a route that repeated a city (right length,
    wrong tour — the test only covered a wrong-length route), and the merged #585 pipeline could
    publish 111 `og:image` links to 404s while `--verify` reported success, because verify
    enumerated the index instead of the pages.
  - A second pass scoped to the fix commit (`--from <prev> --to HEAD`) is cheap and confirms the
    findings are resolved; #588's fix commit came back with 0 findings.
  - Reviewers can be wrong: on #585 it claimed `tests/e2e/publish-minimaps.bats` was untouched (it
    was, +44 lines in the same commit) and that `Math.round` was a no-op. Verify before acting, and
    record the refusals in the task so nobody "fixes" them later.

## Links

- [[tasks/gh-476-algorithms-index]] — the nav/index work that closed the session
- [[tasks/gh-429-bhk-explainer]] / [[tasks/gh-430-branch-bound-explainer]] / [[tasks/gh-431-christofides-explainer]] / [[tasks/gh-432-or-opt-explainer]] / [[tasks/gh-433-stochastic-hill-explainer]] / [[tasks/gh-434-three-opt-explainer]]

# Development Notes

Environment and workflow lessons that keep recurring. Read this before a long
`teeline-web` or multi-worktree session. Operational commands live in
[`AGENTS.md`](../AGENTS.md); principles live in [`constitution.md`](../constitution.md).

## Environment gotchas

### SSH to GitHub is broken on this machine

`/usr/etc/ssh/ssh_config.d/50-suse.conf` is owned by `nobody`, so every `git push`,
`fetch` or `pull` fails unless the config is overridden per command:

```bash
git -c core.sshCommand="ssh -F /dev/null" push
```

The longer form also works when a specific key must be selected:

```bash
GIT_SSH_COMMAND="ssh -F /dev/null -i ~/.ssh/id_ed25519 -o StrictHostKeyChecking=accept-new" git push
```

Do not change the system file or the repository/remote configuration.

### npm/npx are read-only (EROFS)

`~/.npm/_cacache` is on a read-only filesystem — do not use `npx` to install or fetch
packages. Call the local binaries directly:

```bash
./node_modules/.bin/vitest run
./node_modules/.bin/tsc --noEmit -p teeline-web/tsconfig.json
./node_modules/.bin/astro check
./node_modules/.bin/playwright test
```

### `/tmp` is not persistent across tool calls

Write temporary files into a repo-local directory (for example `.tmp-bhkwork/`) and
delete them when done. Repo-local directories must be gitignored or cleaned before
committing.

### Playwright browsers

The bundled Chromium is not installed system-wide and `~/.cache/ms-playwright` may be
read-only. Install and run from the repo:

```bash
PLAYWRIGHT_BROWSERS_PATH=/home/timgluz/repos/teeline/teeline-web/.e2e-state/browsers \
  ./node_modules/.bin/playwright install chromium

PLAYWRIGHT_BROWSERS_PATH=/home/timgluz/repos/teeline/teeline-web/.e2e-state/browsers \
  ./node_modules/.bin/playwright test --project=chrome
```

System Chrome (`/usr/bin/google-chrome`) via `--project=chrome` is the fallback.
`webmcp.spec.ts` needs a production build (WASM lives in `dist/`, not served by
`astro dev`).

### `git rebase --continue` hangs on vim

Use `GIT_EDITOR=true git rebase --continue`. For empty commits after conflict
resolution, use `git rebase --skip`.

### Astro dev auto-daemonizes inside agent harnesses

`astro dev` detects "running in an agent" and backgrounds itself, which breaks
Playwright's `webServer` lifecycle (`Process from config.webServer exited early`). Fix:
set `env: { ASTRO_DEV_BACKGROUND: 'false' }` in `playwright.config.ts` — any truthy value
suppresses the daemonize heuristic.

## Git workflow gotchas

### Verify the branch before committing (learned three times)

`git add -A` on a stacked branch repeatedly staged changes into the wrong branch. Always
check `git branch --show-current` before `git add`/`commit`. Untangling required
`git reset --soft`, `git stash push/pop`, `git show <sha>:<path>` extraction, cherry-picks
and force-pushes — avoidable with a one-second check.

### Stacked-branch squash absorption

Merging a stacked PR squash-merges all of its parent content into master. Later stacked
branches then show empty or partial diffs. Resolution: rebase each branch onto master
taking `--ours` for every conflict (in a rebase, `--ours` is the base/master side;
`--theirs` is your replayed commits — inverted compared with a merge), then force-push.
When a PR shows an unexpectedly empty diff, check whether its content was already
absorbed this way before reopening it.

### Measure from a merged commit

A benchmark or long measurement taken from an open PR's branch records a commit that a
squash-merge will orphan. Start from `master`, or from a commit already merged, so the
recorded SHA remains checkable.

## Simulation and test gotchas (explainer pattern)

- **`structuredClone` cannot clone functions** → a `SimState` must not hold closures (for
  example no prebound `dist` in state). Strip functions before `toEqual` in vitest;
  compare data only.
- **Closed cycles**: TSP results must be closed tours. Whenever an explainer prints or
  claims a "tour", assert that it returns to the start city.
- **e2e hydration**: explainer islands hydrate asynchronously; probe with a `waitHydrated`
  retry loop (scroll inside `toPass`). `getByRole('button', { name: 'Back' })` also
  matches "Read-back" — use an exact label.
- **SSR safety**: the first render must not index arrays derived from zero-revealed
  state; guard on reveal counters.

## Reflections

- **Rust-faithful simulation as the explainer backbone** works: the `*-algo.ts` files
  mirror move semantics, thresholds, tie-breaks and node/leaf logic, so the demos behave
  like the real solver and are deterministic and unit-testable without a DOM.
- **The docs↔nav-data consistency test** (`nav-data.test.ts`, reading
  `docs/algorithms/*.md` via `import.meta.glob` `?raw`) is a cheap guard that caught a
  real user-reported mismatch. Extend it when docs and navigation drift again.
- **Playwright in every verification plan** pays off: hydration races, SSR crashes and
  the Back-button ambiguity were all found by e2e, not by unit tests or type checks.
- **Small instance sizes are a feature**: 8 cities for stochastic hill climbing, 6 for
  B&B/BHK, at most 10 with a brute-forced ratio for Christofides. Demos stay snappy and
  exact algorithms remain legible on screen.
- **The `ocr` review loop** (open-code-review; it exits 1 on a benign session-write EROFS,
  so read the output file anyway) catches real issues. Run it after opening the PR and
  pass `--from origin/master`: a worktree's local `master` is usually stale. Treat the
  output as a review, not a formality — it has found real defects in this repo's fixes —
  but verify its claims before acting.

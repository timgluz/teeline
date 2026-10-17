---
id: 019f2c32-1589-7803-825f-6212073de7f8
slug: task-api-serve-kills-backgrounded-server-on-task-e
title: "task api:serve kills backgrounded server on task exit"
type: incident
status: resolved
priority: medium
---

## Symptom

User ran `task api:serve`, saw the expected task output, but `curl`/browser to
port 8080 showed nothing — no server listening, no process running.

## Root Cause

`api:serve`, `test:e2e:api`, and `test:e2e:auth` (Taskfile.yml) all use the
pattern `CMD & echo $! > pidfile`. go-task doesn't shell out to `/bin/sh` for
each `cmds:` entry — it uses its own bundled POSIX shell interpreter
(`mvdan.cc/sh`), and that interpreter has two problems with this exact pattern:

1. **Wrong PID captured**: `echo $!` writes garbage (reproducibly `"g1"`) to
   the pidfile instead of the real PID.
2. **Child killed on task exit**: the backgrounded process does not survive
   past the end of the `task` invocation — task appears to clean up its own
   process group on exit, killing the child along with it. Confirmed via
   isolated repro: `sleep 30 &` inside a task is gone from `ps` the instant
   `task` returns, even though a plain shell backgrounding the same command
   would leave it running.

For `api:serve` (a single-line task with nothing after the background+echo),
this means the server dies almost immediately — the task exits right after
starting it. For `test:e2e:api`/`test:e2e:auth`, the server happens to survive
long enough for the subsequent `api:wait`/`hurl` steps *within the same task
invocation*, which is why those tests were passing despite the same bug — but
the intended `kill $(cat pidfile)` teardown was silently failing the whole
time (killing PID `"g1"`, a no-op), relying instead on task's own
exit-time process-group cleanup (or `test:e2e:auth`'s `pkill` fallback via
`api:stop`) to actually stop the server.

## Resolution

Wrap the background+PID-capture in an explicit `bash -c '...'` invocation with
`nohup` + `disown`, bypassing go-task's internal shell entirely:

```sh
bash -c 'nohup ./target/debug/teeline-api >/dev/null 2>&1 & echo $! > /tmp/teeline-api.pid; disown'
```

Verified in isolation: the child survives past `task`'s own exit, and the
pidfile contains the correct, real PID.

Fixed in `api:serve`, `test:e2e:api`, `test:e2e:auth` in Taskfile.yml.

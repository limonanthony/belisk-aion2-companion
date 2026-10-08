---
id: TASK-25.36
title: Errors written with eprintln! are lost in the shipped app
status: To Do
assignee: []
created_date: '2026-10-08 15:38'
labels:
  - bug
dependencies: []
parent_task_id: TASK-25
priority: medium
type: bug
ordinal: 86000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The shipped app loses every error and diagnostic written with `eprintln!`. That includes the messages that would explain a lost fight history. When a player reports "my history is gone", or a save failed, `aether.log` holds no trace.

## Why it happens

- **No console.** The app's `main.rs` declares `windows_subsystem = "windows"` for every build without the `probes` feature, which includes the shipped exe. A Windows GUI-subsystem process has no console, so `stderr` goes nowhere.
- **Where the calls are.** In the review snapshot there were 54 `eprintln!` calls across the crates. The biggest groups:

| File | Calls | What they report |
|---|---|---|
| `crates/app/src/secondary/mod.rs` | 8 | Window failures |
| `crates/app/src/lib.rs` | 6 | Startup and quit |
| `crates/core/src/history.rs` | 5 | `failed to write …`, `bad magic`, `unsupported version`: the history failures of TASK-25.3 |
| `crates/app/src/todos/view.rs` | 4 | |
| `crates/app/src/overlay.rs` | 4 | |
| `crates/app/src/todos/service.rs` | 3 | |
| `crates/app/src/settings/model.rs` | 3 | |
| `crates/core/src/engine/meter.rs` | 2 | `[dps_meter] save …`, `[dps_meter] main actor detected …` |

- **The logger exists but these calls bypass it.** The project has a logger, `AppLogger` (`crates/core/src/logger.rs`), that writes `aether.log` under `%LOCALAPPDATA%`, and the app already uses it.

## Evidence

Verified by Main on commit 7a48824: the `windows_subsystem` attribute in `crates/app/src/main.rs` and the `eprintln!` calls in `history.rs`. The per-file counts come from a grep of the snapshot, test code included. Re-count in the current code.

## Suggested direction (Proposed, not agreed)

- Route production `eprintln!` through `AppLogger` at the right level: errors that matter at info or error level, chatter at debug or deleted.
- Delete the debug prints that only narrate, such as `[dps_meter] save …`. The review flagged them as leftover debug.
- Keep `eprintln!` only in code that runs with a console: probes, xtask, packet-explorer, tests.
- Logging must stay cheap on the capture thread. `AppLogger` writes on the calling thread (see TASK-25.5 and TASK-25.7). Do not add file writes per packet.

## How to verify

Force a history load failure (a corrupt `combat_history.bin`) in a non-probes build. `aether.log` names the failure and the file. A grep finds no `eprintln!` left in shipped (non-test, non-probe) code.

## Related

- TASK-25.3: history wipe, whose failures are currently invisible.
- TASK-25.4: personal bests.
- TASK-25.5, TASK-25.7: I/O on hot threads.
- TASK-25.34: leftover debug and narration.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers and counts refer to that commit; re-locate in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A history load or save failure in a non-probes build is written to aether.log with the file and the reason
- [ ] #2 No eprintln! remains in shipped (non-test, non-probes) code
- [ ] #3 No log write is added per packet on the capture thread
<!-- AC:END -->

---
id: TASK-26.1
title: Delete dead code inherited from the forks
status: To Do
assignee: []
created_date: '2026-10-08 12:47'
labels:
  - maintenance
dependencies: []
parent_task_id: TASK-26
priority: low
type: chore
ordinal: 74000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The core still contains code that nothing uses, inherited from the forks: a counter that is written but never read, a logging function nobody calls, file-wide switches that silence the compiler's unused-code warnings, and old code left commented out. It makes the code harder to read and hides real warnings. Also, because every module of the core is public (see TASK-25.12), the compiler cannot flag public functions that have no callers, so more dead code is probably still there.

## Why it happens

Code was kept when features were removed or moved during the forks and the port to the new app: the "do not resend an unchanged frame" gate moved from the calculator to the snapshot loop and left its old bookkeeping behind; the compiler warnings were switched off for whole files instead of fixing them; commented-out experiments were never deleted.

## Items (classification: dead, delete)

| Item | Where | Evidence |
|---|---|---|
| `last_total_damage` is written and passed along, never read | `crates/core/src/engine/calculator.rs` lines 14, 21, 26, 62, 76, 117 | Verified by Main: no reads anywhere in the crates. It is a leftover of an older duplicate-frame suppression that now lives in the snapshot loop (see TASK-25.1) |
| `Logger::error` behind `#[allow(dead_code)]` | `crates/core/src/logger.rs` 63-66 | No callers |
| File-wide `#![allow(dead_code)]` hiding unused items | `crates/core/src/capture/capturer.rs` line 1; `crates/core/src/capture/channel.rs` line 1 | Remove the allow and delete what the compiler then flags |
| Commented-out code | `crates/core/src/capture/dispatcher.rs` 140-141 and 487-495; `crates/core/src/capture/processor.rs` 254-256; `crates/core/src/capture/parser/buff.rs` 10 and 64-69; `crates/core/src/capture/parser/damage.rs` 63; `crates/core/src/storage/data_storage.rs` 351, 506 and 828-832; `crates/core/src/engine/meter.rs` 732 | No runtime effect |

Not done by the scout, part of this ticket: a sweep of the remaining `pub` functions with no callers anywhere in the workspace (the compiler does not flag them while the whole core is public), and a sweep of `scripts/`. This sweep is easier after TASK-25.12 makes the core internals crate-private, because the compiler then reports unused ones; if it is done before, search by hand across all crates (app, xtask, packet-explorer, examples).

## Evidence

Reported by the inventory (static reading of commit 7a48824), not re-verified, except `last_total_damage`, which Main verified as unused. Re-locate each item in the current code (core capture, parser, events, meter and data_storage changed after that commit).

## Suggested direction (Proposed, not agreed)

Delete each item. For file-wide `allow(dead_code)`, remove the attribute first, then delete whatever the compiler flags; do not add item-level allows to keep dead code. Delete commented-out code, do not move it elsewhere. If a "dead" function turns out to be used by a test only, move it into the test module or delete it with the test.

## How to verify

The workspace builds and the tests pass after the deletions; a search for `last_total_damage` finds nothing; no file in the core has a file-wide `allow(dead_code)`; a replay of a recording gives the same boards as before (no behaviour change).

## Related

TASK-25.12 (core internals crate-private makes the sweep reliable), TASK-25.1 (the snapshot gate that replaced `last_total_damage`), TASK-25.34 (comments clean-up; commented-out code is here, narrating comments are there), the Clean up what is left of the forked projects work item (TASK-26).

Source: fork-leftovers inventory of commit 7a48824 (2026-10-08). Line numbers refer to that commit; re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 last_total_damage, Logger::error, the file-wide allow(dead_code) in capturer.rs and channel.rs, and the commented-out code listed are gone.
- [ ] #2 The sweep of public functions with no callers (and of scripts/) is done and every function found is deleted or documented as used.
- [ ] #3 The workspace builds, the tests pass, and a replay gives the same boards as before.
<!-- AC:END -->

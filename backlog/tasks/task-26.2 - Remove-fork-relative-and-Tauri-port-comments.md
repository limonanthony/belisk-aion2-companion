---
id: TASK-26.2
title: Remove fork-relative and Tauri-port comments
status: To Do
assignee: []
created_date: '2026-10-08 12:47'
labels:
  - maintenance
dependencies: []
parent_task_id: TASK-26
priority: low
type: chore
ordinal: 75000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Comments in the code talk about "upstream" (the project this one forked from) and about the Tauri overlay the code was ported from. They describe history, not the code in front of the reader, and some give reasons that are no longer true (for example, keeping a diff small against an upstream that is no longer merged).

## Why it happens

The code was ported from the Tauri app and the forks' core; the porting notes ("Ported from the Tauri overlay", "upstream's hardcoded Taiwan check") stayed in the files after the port was finished and the Tauri app was removed (tag `tauri-final`). Zero merge commits exist from upstream since the fork point `d9f1d98`.

## Items (classification: history, remove the comment)

| Item | Where |
|---|---|
| "keep the diff small against an upstream still actively developed" (the reason given for a process-wide global) | `crates/core/src/region.rs` 21-22 |
| "Region-aware replacement for upstream's hardcoded Taiwan check" | `crates/core/src/region.rs` 155 |
| An upstream regression comment | `crates/core/src/region.rs` 245-247 |
| Other "upstream" comments | `crates/core/src/engine/meter.rs`; `crates/core/src/capture/processor.rs` |
| "Ported from the Tauri overlay" headers | `crates/app/src/format.rs` 1-2; `crates/app/src/game_data.rs`; `crates/app/src/live_session.rs` 1-3, 18-19, 24, 32-33; `crates/app/src/detail/data.rs` 2; `crates/app/src/history/list.rs` 2-3; `crates/app/src/meter/fight.rs` 1-2; `crates/app/src/meter/model.rs` 595-596; `crates/app/src/meter/live.rs` 2; `crates/app/src/meter/summary.rs` 2-3; `crates/app/src/pvp/data.rs` 1-2 |

Careful with the first row: the process-wide region global is load-bearing (the server id locates fields in the packets), so remove only the comment's wrong reason. If a constraint is still real, state it in one sentence without mentioning upstream; if the global is itself a problem, that is TASK-25.18 (region profile) and not this ticket. Whether the "keep core diffs small against upstream" rule stays at all is TASK-25.11; this ticket must not pre-empt that decision for comments that state the rule as a current constraint (check TASK-25.11 first).

Not in scope: comments that only narrate code (TASK-25.34), and design essays in the capture module headers that retell the forks' history (also TASK-25.34's "Where" list).

## Evidence

Reported by the inventory (static reading of commit 7a48824), not re-verified. Re-locate each item in the current code before acting.

## Suggested direction (Proposed, not agreed)

Delete the comment, or reword it to the one fact that still matters (for example "the server id selects the field layout") with no reference to upstream or Tauri. No code change.

## How to verify

A search of `crates/` for "upstream" and "Tauri" finds only statements that are genuine external facts (for example the zh-CN protocol document reference or a data-source credit); the workspace builds and tests pass unchanged.

## Related

TASK-25.11 (upstream-diff rule decision), TASK-25.18 (region profile global), TASK-25.34 (narrating comments), the Clean up what is left of the forked projects work item (TASK-26), F3 (identity and metadata, same files may be touched).

Source: fork-leftovers inventory of commit 7a48824 (2026-10-08). Line numbers refer to that commit; re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The listed upstream and Tauri-port comments are removed or reworded to a current fact with no reference to upstream or Tauri.
- [ ] #2 No code changes; the workspace builds and the tests pass.
<!-- AC:END -->

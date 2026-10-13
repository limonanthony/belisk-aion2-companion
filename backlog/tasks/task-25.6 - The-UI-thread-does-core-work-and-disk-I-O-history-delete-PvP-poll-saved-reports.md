---
id: TASK-25.6
title: >-
  The UI thread does core work and disk I/O (history delete, PvP poll, saved
  reports)
status: In Progress
assignee: []
created_date: '2026-10-08 12:35'
updated_date: '2026-10-12 15:05'
labels:
  - bug
dependencies: []
parent_task_id: TASK-25
priority: medium
type: bug
ordinal: 43000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The app's UI thread does work that belongs elsewhere: disk writes and storage reads. History delete, delete all and reload block the window while a file is written; the PvP window does storage reads on the UI thread every 500 ms; opening a saved fight report builds the report on the UI thread. Each can make the window stutter or freeze.

## Why it happens

- History delete, delete all and reload call into the core meter object on the UI thread (`crates/app/src/history/model.rs`, around lines 87-130). Those calls take the history lock and write the file, so the disk write described in TASK-25.5 blocks the window.
- The PvP window uses a background timer every 500 ms, but the update that follows runs `poll` on the UI thread, which reads core storage (`get_pvp_watch_info` and `get_pvp_combat_stats`, `crates/app/src/pvp/model.rs`, around lines 36-41 and 138-145) and takes the capture read lock.
- Opening a saved fight report builds the report from the stored record (`FightReport::from_record`) on the UI thread, while the live report already builds on the background executor (`crates/app/src/secondary/mod.rs`, around lines 134-160).

## Evidence

Verified by reading the code (commit 7a48824) for the history delete and the PvP poll. The saved-report case is reported by the review, not re-verified.

## Suggested direction (Proposed, not agreed)

Use the same background-executor pattern as the live report: do the work off the UI thread and hand the result back to the UI.

## How to verify

A test or manual check that the window keeps responding while a large history delete is running; a code-level check that the three paths no longer call core storage from the UI thread. Manual check by the user in game for the PvP window.

## Related

TASK-25.5 (moving the write off the threads; this ticket moves the callers). If the TASK-25.5 fix already stops History delete from blocking the window, the remaining scope here is the PvP poll and saved reports.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Deleting history entries, deleting all, and reloading do not block the window.
- [ ] #2 The PvP window's periodic update does not read core storage on the UI thread.
- [ ] #3 Opening a saved fight report does not build the report on the UI thread.
- [ ] #4 History list, PvP numbers and reports show the same data as before.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: built and reviewed (agent-tested, not yet tested by the user). History delete, delete all and reload already only touch memory since TASK-25.5. The PvP window now reads the meter on a background thread, one read at a time, and ignores a read older than a name change or a clear. A saved fight report is built on a background thread like the live one; the window opens when it is ready.

2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Partly fixed in code: PvP polling and saved reports run off the UI thread, and history delete and reload are now memory work plus a queued save. History reload still reads the meter's history on the UI thread (memory only, no file).
<!-- SECTION:NOTES:END -->

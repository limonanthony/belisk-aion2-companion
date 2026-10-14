---
id: TASK-25.5
title: >-
  History and personal-best disk writes run on the capture, snapshot and UI
  threads while holding the history lock
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
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Saving fight history and personal bests writes to disk on threads that must not block: the packet-parsing thread, the snapshot thread, and the UI thread. While a save runs, packet parsing can stall (packets queue up, and can be lost, see TASK-25.7), a History "delete" can stall the snapshot thread's save, and a delete freezes the window. The project rule (AGENTS.md) says the meter's threads never block; this breaks it.

## Why it happens

`save_fight_to_history` (`crates/core/src/engine/meter.rs`, around lines 169-191) is called from:

1. the main-actor callback, on the packet-parsing (dispatcher) thread. That callback fires only when the identified player changes (a new main player is identified), not on every nickname packet. It is still disk I/O on the capture path;
2. the snapshot thread, on the idle timeout;
3. stop and quit.

History "delete" and "delete all" run on the UI thread. `HistoryStore` (`crates/core/src/history.rs`) holds its records mutex across serialise, lz4 compression, atomic write and fsync, and `get_records` needs the same mutex, so readers wait for the disk. Personal bests are written with plain file writes on the same paths.

`MobCatalog` (`crates/core/src/mobs/catalog.rs`) already solves this pattern with a writer thread and a dirty flag.

## Evidence

Verified by reading the code (commit 7a48824), with the narrowing above about when the callback fires. Related lines: `engine/meter.rs` 169-191, 297-307, 513, 691-693; `history.rs` 62-191; `personal_best.rs` 49-67, 121-131; app `history/model.rs` 88-127.

## Decision needed (not decided)

- Option A: a dedicated writer thread, as `MobCatalog` does.
- Option B: take a snapshot of the records under the lock and write it on the existing snapshot thread.

With either option: the packet-thread callback only sets a flag, and the records mutex is never held during I/O.

## Suggested direction (Proposed, not agreed)

Queue "fight ended" and "delete ids" requests to one writer that copies the data under the mutex, releases it, then writes atomically. Do not write when serialising fails (see TASK-25.3).

## How to verify

A test that a save does not hold the records lock during the write (for example a slow writer while `get_records` still answers); replay smoke test with a fight and a delete.

## Related

TASK-25.6 (History delete and reload run on the UI thread), TASK-25.7 (dropped packets), TASK-25.3 (history wipe), TASK-25.4 (personal bests). The planned DataStorage split ticket (A4, not yet created) must not start before this one.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Saving a fight does not hold the records lock while the file is written.
- [ ] #2 The packet-parsing thread does no disk write for history or personal bests.
- [ ] #3 A History delete does not block the snapshot thread or the window.
- [ ] #4 Saved fights and deletes still reach the disk, including on quit.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08: started at the user's request ('You can start 1'). Main's choice of direction, delegated by the user: Option A: a dedicated writer thread, as MobCatalog does; the records lock is never held during disk I/O; pending saves are flushed on quit.

2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Fixed in code: history and bests are written by a background writer and the records lock is free while the file is written. This round the same shared background writer (background_save.rs) also saves pets, to-dos and settings off the UI thread and retries a failed write after 10 s.
<!-- SECTION:NOTES:END -->

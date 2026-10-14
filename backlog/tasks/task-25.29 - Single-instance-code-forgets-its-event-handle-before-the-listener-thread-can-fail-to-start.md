---
id: TASK-25.29
title: >-
  Single-instance code forgets its event handle before the listener thread can
  fail to start
status: In Progress
assignee: []
created_date: '2026-10-08 12:41'
updated_date: '2026-10-12 15:05'
labels:
  - bug
dependencies: []
parent_task_id: TASK-25
priority: low
type: bug
ordinal: 66000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

If the single-instance listener thread cannot be started, the program is left with a leaked event handle and no way to be signalled. Starting the meter a second time then cannot bring the running one forward, while the mutex that marks the first one as running is still held. It is rare (thread creation failure).

## Why it happens

In `crates/app/src/single_instance.rs` (around lines 59-75) the code calls `std::mem::forget` on the owned event handle (so it lives for the whole process) and then spawns the listener thread with `thread::Builder::spawn(...).map_err(...)`. If the spawn fails the handle is already forgotten and leaked, no listener exists, and later launches cannot come forward.

## Evidence

Reported by the review (static reading, low severity), not re-verified.

## Suggested direction (Proposed, not agreed)

Spawn first and forget the handle only on success; on a spawn error drop the owned handle instead of forgetting it.

## How to verify

A test or a manual check where thread creation is made to fail shows the handle is released (the owned handle is dropped) rather than forgotten. Normal start still lets a second launch bring the first one forward.

## Related

TASK-12 (starting the meter again brings the running one forward) owns the product behaviour of this module.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 If the listener thread cannot be started, the event handle is released rather than leaked.
- [ ] #2 With the thread started normally, a second launch still brings the first instance forward.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: fixed: the event handle is forgotten only after the listener thread has started; if the thread cannot start, it is closed with the lock. Also a second launch (and the tray's Show toolbar) now brings the meter forward too when it is open and on screen. Reviewed, not yet tested by the user.

2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Fixed in code: the event handle is forgotten only after the listener thread has started. This round also: the single instance is claimed before the one-time data-folder move, so a second launch no longer retries the move.
<!-- SECTION:NOTES:END -->

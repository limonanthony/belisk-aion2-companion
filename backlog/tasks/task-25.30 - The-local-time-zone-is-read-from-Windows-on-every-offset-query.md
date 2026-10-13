---
id: TASK-25.30
title: The local time zone is read from Windows on every offset query
status: To Do
assignee: []
created_date: '2026-10-08 12:41'
updated_date: '2026-10-12 15:05'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: low
type: enhancement
ordinal: 67000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Every question about the local time offset asks Windows again for the time-zone information. It is not wrong, but it is wasted work: reminders and to-do schedules ask for the offset many times while walking days.

## Why it happens

`crates/app/src/clock/local.rs` (around lines 18-27 and 37-43): `utc_offset_minutes` calls `GetDynamicTimeZoneInformation` and then `SystemTimeToTzSpecificLocalTimeEx` on every call. Reminder `next_wall_start` and to-do `Resets` walk across days, so they call it repeatedly. On failure it falls back to UTC and logs once.

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

Cache the `DYNAMIC_TIME_ZONE_INFORMATION` (refresh on failure, or at most about every minute). DST transitions still need a time argument, so the offset is still computed for the given instant.

## How to verify

The existing reminder and to-do DST tests pass (Berlin spring and autumn transitions). A count of Windows calls during a schedule walk (or a test with an injected zone source) shows one read for the walk instead of one per step. A change of the Windows time zone is still picked up within the refresh period.

## Related

A18 (TASK-25.28, the weekday helper in the same clock folder). TASK-10 and TASK-18 (reminder timing problems) use the same clock.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The Windows time-zone information is read once per schedule walk (or cached) rather than on every offset query.
- [ ] #2 A time-zone change on the PC is still picked up within a short, stated refresh period.
- [ ] #3 Existing DST tests for reminders and to-dos pass.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Still present per the services review (clock/local.rs calls GetDynamicTimeZoneInformation on every query). Cheap; not touched.
<!-- SECTION:NOTES:END -->

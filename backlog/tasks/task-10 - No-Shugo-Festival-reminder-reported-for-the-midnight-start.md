---
id: TASK-10
title: No Shugo Festival reminder reported for the midnight start
status: Done
assignee: []
created_date: '2026-10-06 22:02'
updated_date: '2026-10-09 12:46'
labels:
  - bug
dependencies: []
type: bug
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Players rely on the reminder to catch every Shugo Festival; a friend running the shared build reports that the midnight one is missed.

Reported (verbatim): "Shugo at midnight not detected" (reported by the user's friend, 2026-10-07, build 01bc913)

## Scope

Covers the Shugo Festival reminder for the festival that starts at midnight local time. Reminders for other events and other hours are not covered and must keep working as they do.

## Evidence notes

Observed: only the report above. No screenshot or log.

Derived: Shugo Festival is an event reminder with rule `Hourly { minute: 0 }` (crates/app/src/reminders/events.rs:70), so a midnight start should get a reminder.

Unknown:
- The friend's time zone.
- Whether reminders (and the Shugo Festival reminder) were enabled.
- Whether reminders at other hours work for the friend.
- Whether "detected" means no reminder card appeared.
- Environment details beyond the build: shared zip of commit 01bc913 (v2.4.0).

Cause: Unknown.

Related: TASK-1 (Event reminders for recurring timed events, Done).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With the Shugo Festival reminder on, a reminder appears before the Shugo Festival that starts at midnight local time, as it does for other hours.
- [ ] #2 The midnight start is correct across a day change and on daylight-saving changeover days.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: user: 'For the shugo reminder for the midnight start I need to remember to check correctly I forget everytime (but I believe it's fixed it was when in a loading screen or during a boss fight notifications were not send I believe)'. Today's reminder code has no rule that holds a card back during a loading screen or a fight, and the card now shows under the toolbar when the meter is closed (TASK-24). Waiting for the user to check a midnight Shugo Festival while in game.

2026-10-09: user accepted: "put task-10 in done we'll put it back if it has issues again."
<!-- SECTION:NOTES:END -->

---
id: TASK-18
title: 'Shugo reminder at 20:55 not seen (2026-10-07)'
status: In Progress
assignee: []
created_date: '2026-10-07 19:02'
updated_date: '2026-10-09 12:36'
labels:
  - bug
dependencies: []
priority: high
type: bug
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Shugo Festival reminder was not seen by the user at 20:55 on 2026-10-07.

**Reported** by the user on 2026-10-07 at 21:01: "Shugo just happened and I didn't get the notif".

**Observed** (in the meter's log):
- At 20:55 the Shugo Festival reminder for 21:00 was due. The meter logged that it opened the reminder card, with sound on. The user saw nothing.
- The user was playing at the time (not relogging).
- The 20:00 and 20:30 reminders raised no complaint.
- The meter was not reading the game at that moment because of the capture bug in TASK-19. That does not by itself explain a missing card.

**Cause:** unknown.

**Next step:** the user tries Settings → Reminders → "Show a test reminder" while the game is in front, and says whether the card and the chime appear.

**Related:** TASK-10 (no reminder for the midnight Shugo start, reported by a friend) may have the same cause. It is not changed by this task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 While the user is playing, a due reminder shows its card over the game, and the reminder sound plays when sound is on.
- [ ] #2 Showing the reminder card never takes the keyboard focus from the game.
- [ ] #3 During play, the user sees the next Shugo Festival reminder.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: the user believes this is fixed (cards were held back during loading screens or boss fights; today's reminder code has no such rule, and the card shows under the toolbar when the meter is closed). Waiting for the user to see a Shugo reminder during play.
<!-- SECTION:NOTES:END -->

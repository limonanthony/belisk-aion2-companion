---
id: TASK-34.5
title: Soul tracker card
status: Done
assignee: []
created_date: '2026-10-10 12:46'
updated_date: '2026-10-12 15:05'
labels:
  - feature
dependencies:
  - TASK-34.1
  - TASK-34.2
  - TASK-34.3
parent_task_id: TASK-34
priority: high
type: feature
ordinal: 98000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The short-lived card that shows a pet's progress when the player gains its souls, as approved in the mockups. Part of the parent work item; agreed behavior is listed there.

Depends on the pet data, the name list and the approved mockups.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Gaining souls of a pet below level 3 shows the pet's name with progress toward the next level, for example 26/75 → Lv 3.
- [x] #2 Gaining more souls while the card is shown updates it; the card disappears 10 seconds after its last update.
- [x] #3 Gaining souls of a pet already at level 3 shows Lv 3 75/75 and no insight-crystal figure.
- [x] #4 The card appears while the DPS meter is closed.
- [x] #5 The user confirms the card matches what they gain in game.
- [ ] #6 Turning the "Soul tracker" master switch off in Settings › Pets stops the card from appearing, and the other card settings are disabled while it is off.
- [ ] #7 With "Only pinned pets" on, gaining souls of an unpinned pet shows no card; a pinned pet still does.
- [ ] #8 With "Pets already at Lv 3" off, farming a pet already at level 3 shows no card; on, it shows Lv 3 75/75.
- [ ] #9 With "Black-market purchases" off, a black-market purchase (+5 souls) shows no card; on, it does.
- [ ] #10 "Show a test card" shows the first pinned pet's real progress, or the first pet in game order when nothing is pinned.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-10, user's first in-game test: the card is 'very clean'. The user asked to replace the time-left line, which read as a second bar under the pets' progress bars: approved round docs/gpui-mockups/card-timer/ option A, a 12 px hollow ring left of the × emptying over the card's time, on both the soul tracker card (10 s) and the reminder card (30 s); promoted to docs/gpui-mockups/approved/ and implemented (both cards are 10 px shorter). Awaiting the user's retest.

2026-10-11: tested in game and accepted by the user.

Final card: timer ring left of the × (both cards), 14 px padding on all sides, bodies aligned with the header icon, name / Lv line / bar 8 px apart as seen; the card's window is kept hidden between cards so it reappears at once.

2026-10-12: the card now closes its window on dismissal like the reminder card (user decision); the hidden-and-reused window described above is gone, because a reused window came back under the game (TASK-35). Accepted behaviour on 2026-10-11 is unchanged. Not tested by the user.
<!-- SECTION:NOTES:END -->

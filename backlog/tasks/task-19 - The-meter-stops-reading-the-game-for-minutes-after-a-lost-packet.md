---
id: TASK-19
title: The meter stops reading the game for minutes after a lost packet
status: In Progress
assignee: []
created_date: '2026-10-07 19:23'
labels:
  - bug
dependencies: []
priority: high
type: bug
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The meter stopped reading the game for about 9 minutes during play, even though the game kept sending data.

**Observed** on 2026-10-07 (SEEN in the meter's log and in the user's whole-session recording): from 20:51 to 21:00 the game kept sending data on the same connection, but the meter stopped reading it. It reset the fight as idle at 20:54, and only recovered when the game opened a new connection at 21:00.

**Suspected** (INFERRED, being confirmed on the recording): when one piece of network data is lost, the meter waits for it until a large amount of later data has piled up (about 9 minutes at normal traffic) instead of giving up after a few seconds.

**Effect:** damage, names and fights are missing during that time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A lost piece of data costs at most a few seconds of reading.
- [ ] #2 The recording of 2026-10-07 20:37–21:07 replays without the 9-minute gap.
- [ ] #3 Totals of other recordings are unchanged.
- [ ] #4 The user sees no long blind periods in play.
<!-- AC:END -->

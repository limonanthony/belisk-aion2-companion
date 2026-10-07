---
id: TASK-16
title: 'Thamon: a boss effect is counted as the player''s damage ("Attack")'
status: In Progress
assignee: []
created_date: '2026-10-07 16:35'
labels:
  - bug
dependencies: []
priority: high
type: bug
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In Thamon, a skill shown as "Attack" does far too much damage on the user's own row. It is obviously not one of their attacks.

**Reported:** by the user on 2026-10-07.

**Found in History (seen):** this is a Thamon boss effect that the meter counts as players' damage.
- In the worst fight it made up 46% of the user's damage: 148 hits of up to about 96K, while the user's own skills average 3K to 10K per hit.
- It appears in all 22 Thamon fights saved since 2026-10-05.
- Other players get it on their rows too.

**Related:** TASK-4. The same effect used to show as "Skill 160467011"; that fix gave it a name, which hid the fact that it should not count as player damage at all.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 In a Thamon run, no "Attack" entry from this boss effect appears on any player's row, and it is not included in any player's damage in the fight report.
- [ ] #2 The user's own skills and damage in Thamon, and damage in other fights, are unchanged.
- [ ] #3 The user checks a Thamon run and confirms the effect is gone; ideally also confirmed on a recording of a full Thamon run (automatic recordings only cover two minutes).
<!-- AC:END -->

---
id: TASK-15.3
title: Fight report window
status: In Progress
assignee: []
created_date: '2026-10-07 12:14'
updated_date: '2026-10-07 16:45'
labels:
  - feature
dependencies:
  - TASK-15.1
  - TASK-15.2
parent_task_id: TASK-15
priority: medium
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build the approved fight report: after a boss fight the user clicks a button and sees how every player did.

## Scope

- The button appears where the approved mockups place it.
- The report shows, for every player: damage and DPS, heals, damage taken, and deaths (who died and what killed them).
- Matches the approved mockups.

Not included: clicking into a player (separate subtask), a boss-health graph, a hit-by-hit timeline.

Depends on the approved mockups and on the recording of deaths, damage taken and heals.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After a boss fight, clicking the button opens the report with damage and DPS, heals, damage taken and deaths (who died and what killed them) for every player.
- [ ] #2 The report matches the mockups the user approved.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Built from the approved design; opens from the boss summary card (also "Fight ended, may resume", not when locked) and from History's boss fights (crown + report button). Checked through the app's off-screen captures in English and French against the approved mockups and independently reviewed. Waiting for the user to try it after a real boss fight.
<!-- SECTION:NOTES:END -->

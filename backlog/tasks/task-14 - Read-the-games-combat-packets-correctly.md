---
id: TASK-14
title: Read the game's combat packets correctly
status: In Progress
assignee: []
created_date: '2026-10-07 10:02'
updated_date: '2026-10-07 11:48'
labels:
  - bug
dependencies: []
priority: medium
type: bug
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A comparison with how two other AION 2 meters read the game's traffic showed several mistakes in how our meter reads combat messages. Some hits, targets, owners and boss details can be missed or wrong as a result. The user agreed to fix them ("Yes continue the protocol fixes", 2026-10-07).

## Scope

Covers the four groups of fixes in the subtasks below. Changing what the meter displays, or adding new meter features, is not part of this item.

## Subtasks

- Messages of 128 bytes or more are read correctly (framing).
- Damage hits are read correctly (dropped hits, critical hits, reflected damage).
- Summon owners, monster spawns and boss HP are read correctly.
- Messages the meter does not read yet: monster/boss death, buff removal, combat state, damage-over-time check.

## Related records

TASK-3 mentions the large-message framing fix in the local build; subtask A is the full fix for that.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each subtask is verified by replaying the user's recordings before and after the change.
- [ ] #2 The user confirms the totals look right in a real fight.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
All four parts (TASK-14.1 to TASK-14.4) are implemented and reviewed. Waiting for the user to check totals in a real fight.
<!-- SECTION:NOTES:END -->

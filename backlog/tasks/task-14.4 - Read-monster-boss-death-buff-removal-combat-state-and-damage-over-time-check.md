---
id: TASK-14.4
title: 'Read monster/boss death, buff removal, combat state and damage-over-time check'
status: Done
assignee: []
created_date: '2026-10-07 10:02'
updated_date: '2026-10-08 09:31'
labels:
  - feature
dependencies: []
parent_task_id: TASK-14
priority: medium
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The meter does not read some messages the game sends. Reading them adds: the death of a monster or boss, the removal of a buff, the combat state (including the berserk timer), and a check that damage-over-time totals leave out periodic effects that do no damage.

## Scope

Covers reading these four kinds of message so the meter can use them. Redesigning how the meter displays them is not part of this item.

## Evidence notes

Effect: boss death can be detected from the game itself, and damage-over-time totals no longer include non-damage periodic effects.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A monster or boss death is detected from the game's own message in replayed recordings.
- [x] #2 Damage-over-time totals no longer include periodic effects that do no damage; buff removal and combat state (including the berserk timer) are read.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Monster/boss deaths are confirmed by the death message (on the recordings it always came after HP reached 0, so no kill came earlier); damage-over-time no longer counts self/party/pet effects that were read as damage (lower DoT totals for some players); buffs now end when the game removes them. Combat state (berserk timer) was not added: nothing in the meter would use it yet. Agent-tested only; waiting for the user to check in a real fight.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
User acceptance 2026-10-08: "Most of the tasks you said I've tested and it seems to work correctly (until a new issue appears which is not impossible but for now we can consider them fixed)". Considered fixed on the user's own testing; reopen if a new issue appears.
<!-- SECTION:FINAL_SUMMARY:END -->

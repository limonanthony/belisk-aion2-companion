---
id: TASK-14.3
title: 'Summon owners, monster spawns and boss HP read wrongly'
status: In Progress
assignee: []
created_date: '2026-10-07 10:02'
updated_date: '2026-10-07 11:48'
labels:
  - bug
dependencies: []
parent_task_id: TASK-14
priority: medium
type: bug
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Three things are read wrongly when working out who owns what and how much HP a boss has:
- A summon's owner is taken from a message that actually reports a death and its killer, so a player's summon damage can go to the wrong player.
- Monster spawns are found by guessing a pattern in the data instead of being read properly.
- HP is read as a number too small for large values, so bosses with 1 billion HP or more are dropped.

## Scope

Covers summon ownership, monster spawn detection and boss HP. Rules for how summon damage is shown on the scoreboard are not changed here.

## Evidence notes

Effect: damage attribution between players and boss HP can be wrong; bosses with 1 billion HP or more are missing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A player's summon damage goes to that player in the replayed recordings, not to the player named as a killer.
- [ ] #2 A boss with 1 billion HP or more is no longer dropped and shows its HP.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Monster spawns, summon owners and HP are read from the real fields. On the recordings, summons that showed as separate unnamed rows now count for their owner (for example an Elementalist's spirit); boss and target detection unchanged. HP is 64-bit in the meter's data (the windows still show up to about 4.29 billion). Agent-tested only; waiting for the user to check in a real fight.
<!-- SECTION:NOTES:END -->

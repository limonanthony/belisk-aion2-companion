---
id: TASK-20.1
title: Check the ticket list against the game
status: In Progress
assignee: []
created_date: '2026-10-07 20:31'
updated_date: '2026-10-07 21:36'
labels:
  - feature
dependencies: []
parent_task_id: TASK-20
priority: medium
type: task
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The developer tool decodes the ticket messages (the full list on entering the world and the per-ticket updates) in recordings, so the values can be compared with the game.

The user compares a few values with the in-game counters. That comparison settles the open questions on the parent: the Shugo Festival key id, the Deus Research Base / Shattered Arkanis cap (game table 4 vs our 14), which of two ids is Abandoned Balaur Fortress and which is Fafnium Laboratory, which tickets a Conquest entry spends, and the daily and weekly reset hours.

Part of the parent work item. Uses recordings (TASK-17).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The developer tool shows the decoded ticket list and ticket updates for a recording.
- [ ] #2 For each open question on the parent, the user's comparison with the in-game counters gives an answer or a recorded 'still unknown'.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Update 2026-10-07: the Shugo Festival key is confirmed in game (the user has 7 keys; the game's server-wide key pool shows 7): cap 12, or 21 with membership; +3 a day; extra pool up to 30.

Remaining checks: Invasion keys, Deus Research Base / Shattered Arkanis (cap 4 vs 14), Abandoned Balaur Fortress / Fafnium Laboratory id order, Odyle Energy, and which tickets a Conquest entry spends.

Findings: the owner's notes file `todo-autofill-findings.md`, with detailed notes next to it (notmeter-consumables.md, abyss-consumables.md, tickets-*.txt). Kept outside the repository on purpose (clean-room study of other meters).
<!-- SECTION:NOTES:END -->

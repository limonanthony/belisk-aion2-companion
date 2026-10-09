---
id: TASK-20.1
title: Check the ticket list against the game
status: In Progress
assignee: []
created_date: '2026-10-07 20:31'
updated_date: '2026-10-08 18:15'
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

Update 2026-10-08: the game servers were down, so the in-game comparison could not be done. The user chose to build first and compare the numbers once it works ("do you think it makes sense to implement now anyways and when it works I compare the numbers you found with what I have in game to gain time ?" — agreed).

New evidence from the 2026-10-07 22:40 – 2026-10-08 02:21 recordings:
- The Odyle refill at 00:00 and the shared Conquest pool refill at 01:00 both fit refills timed from a 09:00 (Paris time) daily reset.
- A server-wide weekly ticket (10/week, internal name "Unclaimed") was spent 5 times between 00:24 and 00:48 for an unknown activity.
- No per-dungeon Conquest ticket changed.

Expected values for the in-game check if the reset is 09:00: Shugo keys 9, Invasion keys 3, Balaur/Fafnium 7/7, Deus/Arkanis 4/4, Nightmare 2, Odyle 60 + 245 extra.

Update 2026-10-08: the comparison will be done on the built app when the servers are back.

Update 2026-10-08: in-game check confirmed Invasion keys 3, Nightmare 2, Odyle 90 + 245 extra (3-hour refills) and the 09:00 daily reset. The Daily Dungeon page shows "Attempt Count 14(+7)/14" (ticket 101: regular (+extra) / cap). Conquest, Deus Research Base, Shattered Arkanis, Abandoned Balaur Fortress and Fafnium Laboratory were settled as in TASK-20.3 (no per-dungeon limit on Global; only Odyle and the Daily Dungeon tickets limit entries). Remaining: the Ordeal ticket, known by name only.
<!-- SECTION:NOTES:END -->

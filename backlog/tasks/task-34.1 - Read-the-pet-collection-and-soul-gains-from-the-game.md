---
id: TASK-34.1
title: Read the pet collection and soul gains from the game
status: To Do
assignee: []
created_date: '2026-10-10 12:45'
labels:
  - feature
dependencies: []
parent_task_id: TASK-34
priority: high
type: feature
ordinal: 94000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Belisk reads the player's pet collection and every soul gain from the game's own messages, so later work can show them without the player typing anything.

- The collection arrives once per login, when the character enters the world: for each pet obtained, its level (1 to 3) and the souls gathered toward the next level. Pets never obtained are absent.
- Each soul gain arrives as a message with the pet and the amount (loot, black market). A first unlock of a pet has its own message.
- The collection is shared by the characters of a server, so it is kept per server.
- Reading continues while the DPS meter is closed (agreed: the pets work without the meter).
- Level-ups are computed with the Global thresholds 5/25/75 with carry-over (agreed).
- Souls of a pet already at level 3 are reported by the game without naming the pet; the pet is found through the mob (see the name list subtask).

Decoded from the 2026-10-10 recordings. Technical layout belongs in the repository docs (docs/technical-notes.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A recording of a login replayed in the developer tool (packet-explorer) shows the pet collection: each obtained pet with its level (1 to 3) and the souls gathered toward its next level; pets never obtained are absent.
- [ ] #2 Each soul gain in a recording shows the pet and the amount, and the level and souls after applying the 5/25/75 thresholds with carry-over.
- [ ] #3 A first unlock of a pet is recognised as a new pet at level 1.
- [ ] #4 The collection and the soul gains are read while the DPS meter is closed.
- [ ] #5 Two characters of the same server share one collection.
<!-- AC:END -->

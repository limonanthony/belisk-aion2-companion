---
id: TASK-34.1
title: Read the pet collection and soul gains from the game
status: Done
assignee: []
created_date: '2026-10-10 12:45'
updated_date: '2026-10-10 22:23'
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
- [x] #1 A recording of a login replayed in the developer tool (packet-explorer) shows the pet collection: each obtained pet with its level (1 to 3) and the souls gathered toward its next level; pets never obtained are absent.
- [x] #2 Each soul gain in a recording shows the pet and the amount, and the level and souls after applying the 5/25/75 thresholds with carry-over.
- [ ] #3 A first unlock of a pet is recognised as a new pet at level 1.
- [x] #4 The collection and the soul gains are read while the DPS meter is closed.
- [ ] #5 Two characters of the same server share one collection.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-10: implemented, agent-tested, not yet tested in game. The core decodes the login snapshot (00 90), soul gains (0D 90), first unlocks (09 90) and the six counters' gains (11 90), tied to the character and server like tickets, read while the meter is closed. The app keeps the collection per server in pets.json. packet-explorer `pets` and replay `--pets` show them; technical layout in docs/technical-notes.md. Known limit: gains captured before the game names the player in the session (Belisk started after login, before the next zone change) cannot be tied to a server and are dropped, as for tickets. Independent review: correct, doc findings fixed.

2026-10-11: tested in game and accepted by the user.
<!-- SECTION:NOTES:END -->

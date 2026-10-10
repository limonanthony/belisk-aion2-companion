---
id: TASK-34
title: 'Pet tracking: a pets window and a soul tracker'
status: Done
assignee: []
created_date: '2026-10-10 12:45'
updated_date: '2026-10-10 22:23'
labels:
  - feature
dependencies: []
priority: high
type: feature
ordinal: 93000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Belisk shows the player's pets and their souls, using the values the game itself sends. A window lists every pet with its level and soul count, pets can be pinned to the top, and a short-lived tracker card shows a pet's progress when the player gains its souls.

## Agreed (user, 2026-10-10)

Original request:

- A new window lists all pets, with tabs by family like the game's Pets window (Cogni, Fera, Natura, Varian, Special) plus All, and a search bar. The "research bar" in the original request meant the search bar.
- Pets can be pinned; pinned pets sit at the top of the window.
- Each pet shows its level and its soul count.
- The tracker appears when the player gains souls (soul pickup) and shows progress toward the next level, for example "26/75 → Lv 3". It disappears 10 seconds after its last update.
- A pet already at level 3 shows "Lv 3 75/75" when the player farms it. No insight-crystal figures are shown.
- Global thresholds are per level: 5 souls (level 0 to 1), 25 (1 to 2), 75 (2 to 3). Level 3 is the Global cap. Extra souls carry over into the next level.
- Pet names are in English and French, from free online sources like the npc and skill names. They are never extracted from the game client.
- Reading the pet data does not need the DPS meter to be open.
- The window and the card are designed in mockups first (docs/gpui-mockups/pets/; approved versions in docs/gpui-mockups/approved/).

Mockup decisions (rounds 1 and 2):

- Window layout A (meter rows) and card design A (a twin of the reminder card).
- The window is named "Pets" (French: "Familiers").
- Kept: the family summary per tab, the Show/Order menu, Special pets shown and marked "no stats" (not hidden), the "as of" line, and the card for black-market purchases (+5 souls).
- Dropped: the toolbar count of pinned pets, and typed-in counts.
- Level colours on the level badge only: Lv 0 grey, Lv 1 to 3 greens growing to a solid Lv 3, with the "Lv N" text kept. The progress fill stays neutral.
- A pets tool (paw-print icon) in the toolbar, after To-dos.
- Pets are in game order by default.
- One card holds up to 3 pets in arrival order; any gain restarts the 10 seconds.
- The pet name list from QuestLog.gg is accepted (TASK-34.2).

Settings › Pets page (between To-dos and General):

- Group "Soul tracker card":
  - "Soul tracker" master switch (on). When off, the other settings are disabled.
  - "Only pinned pets" (off).
  - "Pets already at Lv 3" (on).
  - "Black-market purchases" (on).
  - "Show a test card" button: shows the first pinned pet's real progress, or the first pet in game order when nothing is pinned.
- Group "Collection": a read-only line "Read at login on <server>, <date time>" with the owned count (for example "197 of 290"), or how to get the collection read.
- Not in Settings: window sort and filters (they live in the window menu), the 10 seconds, position, sound, forgetting the collection.

## Derived

- The pet collection is shared by the characters of one server, so it is stored per server (follows the game data: byte-identical snapshots for two characters, 2026-10-10 recordings).
- Pins are stored per server with that server's collection (Main's slice decision: what a player chases depends on the collection).
- The tracker card and the window must work while the meter is closed (follows "does not need the meter").

## Scope

Covers the window, the tracker card, the Settings page, reading the pet data, and the name list. Not in scope: insight-crystal display, any cloud storage, anything that touches the game process.

## Subtasks

Reading the data (TASK-34.1), the name list (TASK-34.2) and the mockups (TASK-34.3) come first; the window (TASK-34.4) and the card (TASK-34.5) depend on all three.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A window lists every pet of the player's collection with its level and soul count, with family tabs (Cogni, Fera, Natura, Varian, Special) plus All, and a search bar.
- [x] #2 A pinned pet appears above the unpinned pets in the window.
- [x] #3 Gaining souls of a pet shows a tracker card with progress to the next level (for example 26/75 → Lv 3); the card disappears 10 seconds after its last update.
- [x] #4 Gaining souls of a pet already at level 3 shows Lv 3 75/75; no insight-crystal figure appears anywhere.
- [x] #5 Pet names show in English and French; none comes from the game client.
- [x] #6 The window and the card work with the DPS meter closed.
- [x] #7 The user confirms the levels and souls shown match the game's Pets window.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-11: tested in game and accepted by the user.
<!-- SECTION:NOTES:END -->

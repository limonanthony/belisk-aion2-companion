---
id: TASK-34
title: 'Pet tracking: a pets window and a soul tracker'
status: To Do
assignee: []
created_date: '2026-10-10 12:45'
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

- A new window lists all pets. The user calls it "Pet trackers"; the final label is settled in the mockups.
- The window has tabs by family like the game's Pets window (Cogni, Fera, Natura, Varian, Special) plus All, and a search bar.
- Pets can be pinned; pinned pets sit at the top of the window.
- Each pet shows its level and its soul count.
- The tracker appears when the player gains souls (soul pickup) and shows progress toward the next level, for example "26/75 → Lv 3". It disappears 10 seconds after its last update.
- A pet already at level 3 shows "Lv 3 75/75" when the player farms it. No insight-crystal figures are shown.
- Global thresholds are per level: 5 souls (level 0 to 1), 25 (1 to 2), 75 (2 to 3). Level 3 is the Global cap. Extra souls carry over into the next level.
- Pet names are in English and French, from free online sources like the npc and skill names. They are never extracted from the game client.
- Reading the pet data does not need the DPS meter to be open.
- The window and the card are designed in mockups first (docs/gpui-mockups/, approved ones in approved/).

## Derived

- The pet collection is shared by the characters of one server, so it is stored per server (follows the game data: byte-identical snapshots for two characters, 2026-10-10 recordings).
- The tracker card and the window must work while the meter is closed (follows "does not need the meter").

## Proposed (not agreed; to settle in the mockups)

- A family summary on each tab: pets owned out of the family total, and how many are maxed.
- Filters: not owned, not maxed, closest to the next level.
- Hiding or marking Special pets, which have no stats.
- A toolbar count for pinned pets.
- Typed-in counts as a fallback.
- Showing the last saved collection with an "as of" time when Belisk was not running at login.
- Whether a black-market purchase (also +5 souls through the same game message) shows the tracker.
- The window's final label (the user wrote "Pet trackers").

## Scope

Covers the window, the tracker card, reading the pet data, and the name list. Not in scope: insight-crystal display, any cloud storage, anything that touches the game process.

## Subtasks

Reading the data, the name list and the mockups come first; the window and the card depend on all three.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A window lists every pet of the player's collection with its level and soul count, with family tabs (Cogni, Fera, Natura, Varian, Special) plus All, and a search bar.
- [ ] #2 A pinned pet appears above the unpinned pets in the window.
- [ ] #3 Gaining souls of a pet shows a tracker card with progress to the next level (for example 26/75 → Lv 3); the card disappears 10 seconds after its last update.
- [ ] #4 Gaining souls of a pet already at level 3 shows Lv 3 75/75; no insight-crystal figure appears anywhere.
- [ ] #5 Pet names show in English and French; none comes from the game client.
- [ ] #6 The window and the card work with the DPS meter closed.
- [ ] #7 The user confirms the levels and souls shown match the game's Pets window.
<!-- AC:END -->

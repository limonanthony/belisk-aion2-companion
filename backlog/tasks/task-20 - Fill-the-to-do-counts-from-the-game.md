---
id: TASK-20
title: Fill the to-do counts from the game
status: In Progress
assignee: []
created_date: '2026-10-07 20:31'
updated_date: '2026-10-08 11:47'
labels:
  - feature
dependencies: []
priority: medium
type: feature
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The to-do window should show the real counts of consumables and entry tickets (Odyle Energy, event keys, dungeon and trial tickets, and so on) instead of counts the user keeps by hand.

Agreed (user, 2026-10-07): "The main goal of getting packets reader … is to autofill the consommable like odyle, keys for some events, tickets, etc... For the Todos (might be useful later down the line on other stuff too)". Then, on improving the packet explorer for this: "You can yeah … don't hesitate to improve the packet-explorer for what you need".

Paused 2026-10-07: the user has other fixes to do first.

## What the study found (derived)

- The game sends the full list of its entry tickets (73 of them, including Odyle Energy, the festival and invasion keys, Nightmare, Conquest and Exploration dungeons, Ascension Trial, Subjugation, Sanctuary and more) each time the character enters the world, and an update each time one is spent, gained or refilled.
- Caps, refill amounts and reset hours are not sent; they come from the game's tables.
- Duty Missions and Duty Commands are not in that list.
- This contradicts the earlier to-do research (TASK-2), which concluded the counts cannot be read from the game. That is why the approved to-do design is manual.

## Findings

The summary is the owner's notes file `todo-autofill-findings.md`, with the detailed notes next to it (notmeter-consumables.md, abyss-consumables.md, tickets-*.txt). They are kept outside the repository on purpose (clean-room study of other meters).

## Scope

Rows of the to-do tracker (TASK-2) that are backed by a ticket take the game's value. Rows the game does not send (for example Duty Missions and Duty Commands) stay manual. TASK-2 itself is unchanged by this item.

## Agreed display

To be drawn in the TASK-20.2 mockups.

The user (2026-10-07), on keys going above stock because of premium: "The keys can be over stock (because of the premium with the +X shugo keys on my screen can be 21 but with +X maybe it can be more ? Idk maybe we put the vanilla /X but allow a number higher to count these wdyt ?"

Agreed proposal:

- Show the regular pool against its cap and the extra pool beside it when it is non-zero (for example "7 / 21" and "+3").
- A total above the cap is normal, never an error.
- "Full, being wasted" shows only when the regular pool reaches its cap.
- Typed values accept numbers above the cap (today they are clamped).
- The To-dos membership switch picks the membership cap for every ticket (for example Shugo keys 12 → 21, Odyle 560 → 840).

## Settled

- Shugo Festival key: confirmed in game 2026-10-07 (the user has 7 keys; the game's server-wide key pool shows 7). Cap 12, or 21 with membership; +3 a day; the extra pool goes up to 30. Our manual rule (28, +4 at 16:00) is wrong and will be corrected.

## Open questions (unknown, to settle in the first subtask)

- The Deus Research Base / Shattered Arkanis cap: game table says 4, our to-do has 14.
- Which of two ids is Abandoned Balaur Fortress and which is Fafnium Laboratory.
- Which tickets a Conquest entry spends.
- The exact daily and weekly reset hours (inferred: 09:00 CEST daily, Wednesday weekly).

## Related

TASK-2 (existing to-do tracker, manual counts), TASK-17 (whole-session recording, the source of the recordings the check uses).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A to-do row backed by a game ticket shows the game's value after the character logs in, and follows spends and refills while playing.
- [ ] #2 A to-do row the game does not send (for example Duty Missions, Duty Commands) stays manual.
- [ ] #3 The user confirms the filled values match the game's own counters.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Resumed 2026-10-08.
<!-- SECTION:NOTES:END -->

---
id: TASK-13
title: Settings window opens behind other windows
status: In Progress
assignee: []
created_date: '2026-10-07 00:15'
labels:
  - bug
dependencies: []
type: bug
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A player who opens Settings from the meter wants to change a setting straight away; today the Settings window opens behind the game and other windows, so they have to go looking for it.

Reported (user, 2026-10-07, verbatim): "could you make the settings window that opens when we click settings in the foreground please ? It's always in the background it's pretty annoying"

## Scope

Covers the Settings window opened from the meter's "…" menu: it appears in front and ready to use.

The other windows opened from the meter (history, detail, PvP, to-dos) are checked for the same behaviour; any change they need is recorded separately.

Does not change the meter itself: it stays a non-activating overlay that never takes focus from the game.

## Evidence notes

Observed (user report): the Settings window always opens in the background.

Expected: Settings opens in front of the game and other windows, with keyboard focus in it.

Cause: unknown, being investigated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Clicking Settings in the meter's "…" menu opens the Settings window in front of the game and other windows, with keyboard focus in it.
- [ ] #2 If the Settings window is already open behind other windows, clicking Settings brings it to the front.
- [ ] #3 Opening Settings does not make the meter itself take focus from the game when Settings closes.
<!-- AC:END -->

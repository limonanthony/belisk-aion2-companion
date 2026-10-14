---
id: TASK-38
title: Toolbar does not reopen where it was left
status: Done
assignee: []
created_date: '2026-10-14 15:09'
updated_date: '2026-10-14 19:09'
labels:
  - bug
dependencies: []
priority: high
type: bug
ordinal: 102000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Reported by the user on 2026-10-14: "the toolbar remember is not working on my end which is why I thought it didn't remember". The toolbar does not reopen where the user left it.

Expected (agreed intent, from the user's report): the toolbar reopens where the user last put it.

## Evidence

Observed by Main, 2026-10-14:
- The user's settings.json holds a saved toolbar position, `windows.toolbarAt` = (74, 48). So a position was saved at least once; whether it is the last place the user left the toolbar is unknown.
- The position is saved when the toolbar's mouse button is released after a drag, and read back when the toolbar opens. A saved place that is on no monitor falls back to the top centre of the primary screen.

Suspected, not proven: since the overlay drag change of 2026-10-11 the release may not reach the toolbar as a normal mouse-up, so nothing is saved; the position read after the drag may also race the last window move. The cause must be found and proven, not assumed.

## Scope

This item covers the toolbar's own saved position. Remembered positions for the meter and the other tool windows are the separate feature item.

## Related

TASK-24 (toolbar hub, Done), the feature item "Windows reopen where you put them (meter and every tool)".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 After dragging the toolbar and restarting Belisk, the toolbar opens where it was left.
- [x] #2 A saved place that is no longer on any monitor opens the toolbar at the top centre of the primary screen.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-14: cause proven by an agent test: the position was saved only from the toolbar root's mouse release after a drag; GPUI runs that handler only when the release is over the window, and since the 1.1.0 drag (the window follows the pointer with a lag) the release often landed outside it, so nothing was saved and a later click could save a stale spot. Fix in code (agents): the end of every overlay drag now reports where the window ended up, after its last move, and that saves the position (TASK-39's mechanism). Agent-tested; awaiting the user's test.

2026-10-14 (user test): within a run the meter and toolbar reopened where left; after a restart both opened at their defaults although settings.json held the saved places (toolbarAt 53,40; meterAt 11,817). Cause: the settings loader split every stored object into fields and refused a field whose default was not an object; window places default to null, so every saved place was dropped when settings.json was read. This is also why the toolbar's place never survived a restart before. Fixed in code by agents (the loader keeps a stored place whole), regression test fails before and passes after, independent review accepted; awaiting the user's restart test.

2026-10-14: user tested and confirmed after a restart: 'yup it works now'. AC #2 (a saved place on no monitor) is agent-tested only.

2026-10-14: user tested the delivered build in game on 2026-10-14 and, asked whether everything worked (start alert at a :00/:30, soul card after a mid-game start, tool windows reopening in place, clean launch log), answered: "Everything seem[s fine]". Evidence Main observed in the user's belisk.log and data folder: saved places restored after restarts (toolbar and meter, tool places in settings.json). Accepted by the user; AC #2 (a saved place on no monitor) stays agent-tested only and is checked on the strength of the user's overall acceptance.
<!-- SECTION:NOTES:END -->

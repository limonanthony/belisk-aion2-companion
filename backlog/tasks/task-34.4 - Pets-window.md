---
id: TASK-34.4
title: Pets window
status: Done
assignee: []
created_date: '2026-10-10 12:46'
updated_date: '2026-10-10 22:23'
labels:
  - feature
dependencies:
  - TASK-34.1
  - TASK-34.2
  - TASK-34.3
parent_task_id: TASK-34
priority: high
type: feature
ordinal: 97000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The window that lists every pet with its level and souls, as approved in the mockups. Part of the parent work item; agreed behavior is listed there.

Depends on the pet data, the name list and the approved mockups.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The window lists every pet with its level and soul count; family tabs (Cogni, Fera, Natura, Varian, Special) plus All filter the list, and the search bar narrows it by name in English or French.
- [ ] #2 A pinned pet stays at the top of the window, including after restarting Belisk.
- [x] #3 Opening the window while the DPS meter is closed shows the collection read at login.
- [x] #4 The user confirms the levels and souls match the game's Pets window.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-10, user's first in-game test: the Pets window froze while dragged (the only window doing it). Cause: every move redraws the whole window and it built all ~207 rows; now only the rows in view are built (~185 → ~18 ms per move frame, debug build, GPUI test platform). Awaiting the user's retest.

2026-10-11: tested in game and accepted by the user.

Also changed for every window: lists use GPUI's ListState (History too), one shared draggable scrollbar, window drags no longer redraw on every move, dependencies optimized in dev builds.
<!-- SECTION:NOTES:END -->

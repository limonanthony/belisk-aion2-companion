---
id: TASK-34.5
title: Soul tracker card
status: To Do
assignee: []
created_date: '2026-10-10 12:46'
labels:
  - feature
dependencies:
  - TASK-34.1
  - TASK-34.2
  - TASK-34.3
parent_task_id: TASK-34
priority: high
type: feature
ordinal: 98000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The short-lived card that shows a pet's progress when the player gains its souls, as approved in the mockups. Part of the parent work item; agreed behavior is listed there.

Depends on the pet data, the name list and the approved mockups.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Gaining souls of a pet below level 3 shows the pet's name with progress toward the next level, for example 26/75 → Lv 3.
- [ ] #2 Gaining more souls while the card is shown updates it; the card disappears 10 seconds after its last update.
- [ ] #3 Gaining souls of a pet already at level 3 shows Lv 3 75/75 and no insight-crystal figure.
- [ ] #4 The card appears while the DPS meter is closed.
- [ ] #5 The user confirms the card matches what they gain in game.
<!-- AC:END -->

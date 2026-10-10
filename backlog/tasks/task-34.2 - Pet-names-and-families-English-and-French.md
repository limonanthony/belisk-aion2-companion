---
id: TASK-34.2
title: Pet names and families (English and French)
status: To Do
assignee: []
created_date: '2026-10-10 12:45'
labels:
  - feature
dependencies: []
parent_task_id: TASK-34
priority: high
type: feature
ordinal: 95000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Belisk ships a list of pets so the window and the card can show names instead of numbers, in English and French (agreed), taken from free online sources, never from the game client (agreed).

- For each pet: English name, French name and family (Cogni, Fera, Natura, Varian, Special).
- Which mobs give which pet. A pet already at level 3 gets its souls from the game without being named, so the mob that was hit is what identifies it.
- French UI keeps the English gaming terms that French players know; class and skill names follow the project's existing translation rules.
- Source and licence of the list are recorded with the data.

Part of the parent work item. Needs no game data from the first subtask beyond the pet ids it reads; the id-to-name mapping is matched to the ids decoded there.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every pet id read from the game maps to an English name, a French name and one of the families Cogni, Fera, Natura, Varian, Special.
- [ ] #2 A mob that gives a pet maps to that pet, so souls of a pet already at level 3 can be attributed.
- [ ] #3 No name or mapping comes from the game client, and the data holds no Chinese text.
- [ ] #4 A pet id missing from the list is shown with a visible fallback instead of a blank or a crash.
<!-- AC:END -->

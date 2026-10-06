---
id: TASK-4
title: Skill shown as a raw id (Skill 160467011)
status: To Do
assignee: []
created_date: '2026-10-06 14:49'
labels:
  - bug
dependencies: []
type: bug
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A skill showed up in the kill summary as a raw id instead of its name. The user wants the skill-name data extended so fewer skills show as ids.

User (2026-10-06): "A skill ID is not detected maybe we should increase our db about these."

## Scope

Covers naming skill 160467011 and a way to find other skill ids that have no name, so the bundled skill-name data can be extended. Collecting names for every unnamed id in the game is not part of this item.

## Evidence notes

Observed (user's screenshot, kept locally, not in the repository): the kill summary's top skill read "Skill 160467011" (14%), listed next to the named skills Combustion and Elemental Fusion.

Expected: the skill shows its name, like the other skills in the list.

Environment: reported 2026-10-06, in game, on the user's local build of branch phase-4-settings with uncommitted changes.

Cause: Unknown. Hypothesis, not confirmed: the bundled skill-name data (skill names under crates/app/assets/game/data) lacks this id.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Skill 160467011 shows its name in the kill summary, not "Skill 160467011".
- [ ] #2 Skill ids that fell back to "Skill <id>" can be listed, so their names can be added to the bundled skill-name data.
<!-- AC:END -->

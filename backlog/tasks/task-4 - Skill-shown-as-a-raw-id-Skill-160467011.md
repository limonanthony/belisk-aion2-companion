---
id: TASK-4
title: Skill shown as a raw id (Skill 160467011)
status: In Progress
assignee: []
created_date: '2026-10-06 14:49'
updated_date: '2026-10-09 10:52'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-06 agent fix, awaiting user test. Cause: not missing data. 160467011 is a damage-over-time id (skill id followed by two digits); the lookup did not try id/100. New crates/core/src/skill_names.rs adds that as the last candidate, so names that resolved before are unchanged. 160467011 now resolves to key 1604670, whose bundled name (from NOIA2's skill data; Kuroukihime's data agrees) is "Attack". AC2: developer-only `cargo run -p packet-explorer -- unnamed-skills <recording|folder>... [--json]` lists unnamed skill ids with hits, damage and class. On the 7 local recordings it found 8 unnamed ids: 16990000, 16000000, 100050, 100020, 100010, 100040 (Elementalist), 201110111, 201110112 (Templar). Independent review accepted. Not observed: the in-game kill summary.

2026-10-08: user: 'I right now see no issues left except the skill.' Skill 16990000 (top Elementalist damage, unnamed in the meter) is still shown as an id; waiting for the name the game shows in Combat Analysis (Ctrl+X).

2026-10-09: 'Skill 16990000' is not a damage skill: it is a record the game sends for each Elementalist spirit just after it appears (target = the spirit itself); we counted it as damage. Fix tracked in TASK-33. Its total matches 98% of the game's Total Heal, so it may be a heal; waiting for the game's Heal tab screenshot.
<!-- SECTION:NOTES:END -->

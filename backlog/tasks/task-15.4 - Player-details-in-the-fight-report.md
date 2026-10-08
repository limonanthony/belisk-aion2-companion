---
id: TASK-15.4
title: Player details in the fight report
status: Done
assignee: []
created_date: '2026-10-07 12:14'
updated_date: '2026-10-08 09:31'
labels:
  - feature
dependencies:
  - TASK-15.1
  - TASK-15.2
parent_task_id: TASK-15
priority: medium
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build the approved player view in the fight report: clicking a player shows what they did in that fight.

## Scope

- Damage per skill.
- Critical hits and the other hit types (back, front, parry, perfect, double).
- Which buffs and skills were active and for how long, in seconds, not only as a percentage.
- Matches the approved mockups.

The existing per-skill detail window for saved fights is the starting point; it already shows hit-type rates and buff coverage as a percentage.

Not included: a hit-by-hit timeline.

Depends on the approved mockups and on the recording of deaths, damage taken and heals.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Clicking a player in the report shows their damage per skill, critical hits and the other hit types.
- [x] #2 The view shows how long each buff and skill was active, in seconds.
- [x] #3 The view matches the mockups the user approved.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Built from the approved design; opens from the boss summary card (also "Fight ended, may resume", not when locked) and from History's boss fights (crown + report button). Checked through the app's off-screen captures in English and French against the approved mockups and independently reviewed. Waiting for the user to try it after a real boss fight.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
User acceptance 2026-10-08: "Most of the tasks you said I've tested and it seems to work correctly (until a new issue appears which is not impossible but for now we can consider them fixed)". Considered fixed on the user's own testing; reopen if a new issue appears.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-15.2
title: 'Record deaths, damage taken and heals for each fight'
status: In Progress
assignee: []
created_date: '2026-10-07 12:14'
updated_date: '2026-10-07 16:45'
labels:
  - feature
dependencies:
  - TASK-15.1
parent_task_id: TASK-15
priority: medium
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
For the fight report to show deaths, damage taken and heals, the meter has to keep them during a fight and save them with the fight.

## Scope

- During a fight the meter keeps, for each player: heals, damage taken, and deaths (who died and what killed them).
- These are saved with the fight, so they are available when the report is opened later.
- Fights saved before this change keep opening normally, without these figures.
- Heals: which effects count as heals, as opposed to shields or life drain, is checked on a recording with a healer in the party (Cleric or Chanter) before it is settled. If this recording cannot be found, that is raised as an open question rather than guessed.

Not included: showing these figures (see the report and player-detail subtasks).

Depends on the mockups subtask because the approved report decides which figures are shown. Related: TASK-14.4 (boss death, buff removal).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After a fight, the meter has each player's damage taken and deaths (who died and what killed them) for that fight, and they are still there when the saved fight is opened later.
- [ ] #2 Heals are recorded for each player and checked against a recording with a healer in the party (Cleric or Chanter).
- [ ] #3 Fights saved before this change still open.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented (agent-tested, not yet user-tested). The meter now keeps deaths (who, when, killer and its skill, last hit), damage taken per player by monster skill, heals per player and skill, and buff uptime for each boss fight, and saves them with the fight; older saved fights still open with these marked not recorded. Healing counts heal skills and heals over time; the small flat recovery that many classes get on each hit is left out (decided 2026-10-07; one switch to include it). Shields cannot be told apart from heals yet. Checked on the user's recordings and their real History file; no party death occurs in the recordings, so the deaths list is covered by tests only.
<!-- SECTION:NOTES:END -->

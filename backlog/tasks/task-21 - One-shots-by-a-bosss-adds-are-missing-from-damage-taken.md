---
id: TASK-21
title: One-shots by a boss's adds are missing from damage taken
status: In Progress
assignee: []
created_date: '2026-10-07 22:53'
labels:
  - bug
dependencies: []
priority: high
type: bug
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When a player is killed by one of a boss's adds, the hit that killed them is missing from the fight's damage taken and from the death's last hit.

**Reported** 2026-10-07 ~23:40: "When someone gets one-shot I don't think it writes the damage taken because my friend who had 14.2k hp got one-shot 3 times and the fight history only showed 14.2k hp dmg taken".

**Observed** (SEEN on the recording): the game recorded the killing hits with their full amounts: 18,576 and 15,963 from the boss's adds, and 14,180 from the boss itself. The meter skipped every hit from a summoned creature so that players' pets would not be counted, and a boss's adds are summoned creatures. Only the boss's own hit counted, so the two deaths caused by adds showed no last hit and their damage was missing from damage taken.

**Fixed** (agent-tested): hits from a monster's summons now count at their full amount; players' pets are still left out. Fights already saved keep their old numbers.

Related: TASK-15 (end-of-fight report shows damage taken and deaths).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 In a fight with adds, a player killed by an add shows that hit in their damage taken and as the last hit of their death.
- [ ] #2 Players' pets are still not counted as damage dealt to players.
- [ ] #3 The user confirms this in a Transcendence run.
<!-- AC:END -->

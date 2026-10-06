---
id: TASK-5
title: The user's own name shown twice on the scoreboard
status: To Do
assignee: []
created_date: '2026-10-06 14:49'
updated_date: '2026-10-06 14:49'
labels:
  - bug
dependencies: []
type: bug
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
On one scoreboard the user's own character was listed twice under the same name and class.

User (2026-10-06): "In one scoreboard my username appeared twice"

## Scope

Covers the user's own character appearing on more than one row of the scoreboard. How summon damage is attributed follows the existing summon rules and is not changed here.

## Evidence notes

Observed (the same screenshot as the missed-boss report, taken mid-dungeon; kept locally, not in the repository):
- The session board listed the user's character twice, with the same name and class:
  - rank 1: 48.10M (40.7%)
  - rank 6: 275.7K (0.2%)
- Several rows below were unnamed "ID:<n>" rows at about 0.1% each.

Expected: the user's character appears once.

Environment: reported 2026-10-06, in game, on the user's local build of branch phase-4-settings. That build includes this week's own-identity change (own actor found by the remaining-HP signal, name carried across connections), which is uncommitted and still being tested in game.

Reproduction: not yet reproduced; seen on one scoreboard.

Cause: Unknown. Hypotheses, none confirmed:
- The own-identity change gave the user's name to a second actor id, possibly the character's summoned spirit or the actor from a previous connection.
- A summon was attributed under its owner's name.

Evidence to gather: aether.log `[meter] identity:` lines with Settings → Capture → Decision log on.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On every scoreboard, the user's character appears on one row only.
- [ ] #2 Damage from the user's own summons is counted on the user's row or shown as clearly theirs, following the existing summon rules.
<!-- AC:END -->

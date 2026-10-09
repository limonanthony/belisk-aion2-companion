---
id: TASK-5
title: The user's own name shown twice on the scoreboard
status: Done
assignee: []
created_date: '2026-10-06 14:49'
updated_date: '2026-10-09 09:48'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-06 agent fix, awaiting user test. Cause: the session board keyed rows by actor id, and the player gets a new actor id in every zone; the earlier id kept its damage and name, giving a second row with the same name and class. Fix (crates/core/src/engine/session.rs): rows of every id the meter established as the local player during the session are folded into one row; name alone never merges, so a namesake from another server keeps its own row. The session skill detail for that row adds up to the board figure. Summon rules unchanged. Independent review accepted after one correction round. Not observed: a live session board after real zone changes. The unnamed "ID:<n>" rows were not investigated. Other players may still show one row per zone they were seen in [inference, not tested].

2026-10-08: user tested and accepted: "Toolbar hub works great and I haven't seen issues with the resources. So you can move all of these to done if we find issues we'll fix them on the go. I right now see no issues left except the skill. You can start 1."
<!-- SECTION:NOTES:END -->

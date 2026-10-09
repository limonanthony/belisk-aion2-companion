---
id: TASK-11
title: Hide unknown players does not hide ID rows on the Target board
status: Done
assignee: []
created_date: '2026-10-06 22:36'
updated_date: '2026-10-09 09:48'
labels:
  - bug
dependencies: []
type: bug
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A player who turns on "Hide unknown players" expects unnamed "ID:<n>" rows to disappear from the meter; on the Target board they still show.

Reported (user, 2026-10-07, verbatim): "I still see ID:{Number} even with the setting on hiding them wqhe nin boss only. It's not the first time something is not working correctly and it's starting to annoy me."

## Scope

Covers where the existing Settings → Behaviour → "Hide unknown players" switch applies: the Target board in both count modes (All targets and Boss only), and every other place that lists players by name (Session board, PvP, fight summary, detail, history).

Does not cover the switch's default value for new installs (TASK-8).

## Evidence notes

Observed:
- The user's settings.json has `behaviour.hideUnknownPlayers` = true and count = `bossOnly`.
- A screenshot from the same night shows the Target board during a boss fight with the rows "ID:1499", "ID:7121" and "ID:16319" next to named players.

Expected: with the switch on, no "ID:<n>" row appears on the Target board; the user's own row stays visible even while unnamed.

Cause (confirmed from the code): the core applies the filter only to the last-target overview list, the Session list and the PvP lists (crates/core/src/engine/calculator.rs:118-125, :164, :272; crates/core/src/engine/session.rs:121). The app builds the Target board from the unfiltered per-target table (`target_view`, crates/app/src/meter/live.rs:154-160), deliberately, so the user's own row survives the core's player cap. The setting therefore never reaches the Target board.

Correction: TASK-8's "Current behavior" states that the switch hides "ID:<n>" rows on the Target board. That statement is wrong for the reason above.

Related: TASK-8 (Hide unknown players on by default for new installs).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With "Hide unknown players" on, no "ID:<n>" row appears on the Target board, in both count modes (All targets and Boss only), including for a boss the meter focuses that is not the last target.
- [ ] #2 The user's own row stays visible while the game has not named them.
- [ ] #3 Every other place that lists players by name (Session board, PvP, fight summary, detail, history) follows the switch the same way, or this task notes why a place is exempt.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08: user tested and accepted: "Toolbar hub works great and I haven't seen issues with the resources. So you can move all of these to done if we find issues we'll fix them on the go. I right now see no issues left except the skill. You can start 1."
<!-- SECTION:NOTES:END -->

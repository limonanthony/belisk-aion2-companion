---
id: TASK-3
title: Dungeon boss not detected in one run
status: To Do
assignee: []
created_date: '2026-10-06 14:49'
updated_date: '2026-10-06 14:49'
labels:
  - bug
dependencies: []
type: bug
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In one dungeon run the meter did not recognise the boss: it showed a raw mob code instead of the boss name and stayed on the session board for the whole fight.

User (2026-10-06): "The boss was again not detected for some reason even though it was detected all the other times."

## Scope

Covers boss detection and boss focus for this dungeon boss (level 54, "Spirit Predator Tiere", mob code 2920133). Other dungeon bosses are detected normally and must stay that way.

## Evidence notes

Observed (user's screenshot, kept locally, not in the repository):
- Fighting the level-54 dungeon boss "Spirit Predator Tiere" for over 30 minutes of session time.
- The meter header showed a raw mob code ("…b 2920133", cut off) instead of the boss name.
- The meter stayed on the session board; it never switched to the boss.

Expected: the meter names the boss and focuses on it, as it does for other dungeon bosses.

Environment: reported 2026-10-06, in game, on the user's local build of branch phase-4-settings with uncommitted changes (own-identity rework, large-packet framing fix).

Reproduction: not yet reproduced; seen in one run.

Cause: Unknown. Hypotheses, none confirmed:
- Mob code 2920133 is missing from the bundled names or boss data (for example, another difficulty variant of a known boss).
- The large-packet framing fix in this build.
- The boss-focus logic.

Evidence to gather:
- aether.log with Settings → Capture → Decision log on, around 2026-10-06 ~16:00–16:10 local time.
- Whether 2920133 appears in the bundled NPC names and boss data (crates/aether-core/src/data).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When this boss (mob code 2920133) is fought, the meter header shows the boss name, not the mob code.
- [ ] #2 During the fight the meter focuses on this boss, as it does for other dungeon bosses.
<!-- AC:END -->

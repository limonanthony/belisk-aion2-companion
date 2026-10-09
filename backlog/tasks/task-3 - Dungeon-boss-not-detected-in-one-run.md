---
id: TASK-3
title: Dungeon boss not detected in one run
status: Done
assignee: []
created_date: '2026-10-06 14:49'
updated_date: '2026-10-09 09:48'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-06 investigation, partly fixed. Finding: mob code 2920133 is not the boss. aether.log pairs every 2920133 with a `summon` line owned by an Elementalist: it is the Elementalist's spirit summon (about 13k HP). Spirit Predator Tiere's codes (2300580, 2300810, 2310810, 2320810) are in the bundled boss data with names. The header showed "Mob 2920133" because the player's skill 16990000 hits her own spirits, which made the spirit the last target. Fixed: a hit on the attacker's own summon no longer makes it the target (crates/core/src/storage/data_storage.rs); damage is still counted. Not reproduced: the boss never being focused. There is no recording of 15:31–16:02, and every Tiere fight in the log in that window engaged correctly. Unconfirmed candidates: a boss whose spawn packet was missed has no mob code and its snapshots are dropped; a stale id-to-code mapping. Needs a recording or a Decision log of a failing fight. The acceptance criteria name 2920133 as the boss; they need the user's correction.

2026-10-06 cause found. The missed fight was 2026-10-05 16:43–16:57 (Europe/Paris), not 2026-10-06. That build kept the town actor id (6812) after entering the dungeon, where the player became actor 7735; boss focus only engages when "you" hit the boss, so Tiere (21766, 2300810) was never engaged (aether.log.1: repeated "identified as Ysmera (actor Some(6812)) but not among the players", no "boss focus: engaged"). Current builds rebind the player's id on zone change (commit d320c26); every Tiere fight on 2026-10-06 engaged. Remaining gap: the rebind waits for the player's HP to change, so a boss fought in that window can still be missed (seen only on hub connections, 23 s to 3 min). User decision 2026-10-06: option A, keep the rebind as the fix with no further change for now; keep Settings → Capture → Decision log on to catch any recurrence. Rejected for now: binding the first actor of a new connection immediately (B), and treating an unconfirmed id as unidentified so anyone's boss engages (C).

2026-10-08: user tested and accepted: "Toolbar hub works great and I haven't seen issues with the resources. So you can move all of these to done if we find issues we'll fix them on the go. I right now see no issues left except the skill. You can start 1."
<!-- SECTION:NOTES:END -->

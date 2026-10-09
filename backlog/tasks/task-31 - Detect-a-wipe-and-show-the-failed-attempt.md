---
id: TASK-31
title: Detect a wipe and show the failed attempt
status: Done
assignee: []
created_date: '2026-10-08 18:15'
updated_date: '2026-10-09 09:48'
labels:
  - feature
dependencies: []
priority: medium
type: feature
ordinal: 90000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When a raid or party wipes against a boss, the meter does not notice: the boss refills, and the player has to refresh the meter by hand to see the next pull. The user wants the wipe detected as the end of that attempt.

## Agreed (user, 2026-10-08)

> "there is no detection when we get wiped and the boss regen. When the boss goes back to 100% hp we should consider this a reset no ? Meaning when all teammates are dead we put another message instead of Defeated in red and the stats of that failed fight ?"

- A wipe is detected from the boss, not from player deaths. After the user pointed out that players can respawn instantly with stones, so "everyone dead at the same time" cannot be known, and no revive event is known, the boss was chosen as the signal. The user: "I'm ok with that no need for mockups, yes there was a wipe but I have the refresh the dps meter to see the new combat".
- A wipe is: the boss is alive and its HP returns to full after it took damage, or it disappears without dying.
- The attempt ends there. The meter shows a red outcome instead of "Defeated", with the boss's HP left and the stats of that attempt.
- The next pull starts a new attempt.
- No mockups (waived by the user).

## Proposed by Main, not objected to by the user (accepted implicitly)

- Wipes are saved in History with their fight report, and marked as wipes.
- Wipes do not count for personal bests.
- Dungeon boss focus stays locked to the detected boss across a wipe.

## Unknown

- Whether the game's data actually shows the boss's HP refill. Being checked on the user's recordings.

## Boundaries

Kills and the existing "Fight ended" outcome behave as before.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After a wipe the meter shows the failed attempt's summary with a wiped outcome (red, instead of Defeated) and the boss's HP left, without a manual reset.
- [ ] #2 The next pull against the boss starts a separate attempt.
- [ ] #3 The wipe appears in History, marked as a wipe, with its fight report.
- [ ] #4 A wipe does not change personal bests.
- [ ] #5 Boss kills and the 'Fight ended' outcome behave as before.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08: Built and independently reviewed, not committed, awaiting the user's test. The game's data confirms the signal: on the user's recording of 2026-10-08 19:56 Talisra of the Void went from 12.8% straight back to full HP (a wipe); in other recordings Atiel disappeared without dying and came back as a new spawn. Rules: the boss's HP back to full after it dropped below 98% (with an unchanged max HP), or the boss removed without dying; a death in the same moment cancels it. The wipe shows a red 'Wiped · N% left' card with the attempt's stats and report, is saved in History with a red mark and its report, is left out of personal bests, and the next pull starts from zero. In dungeons the boss stays locked across the wipe (a respawned boss takes the lock); in the open world it is released.

2026-10-08: user tested and accepted: "Toolbar hub works great and I haven't seen issues with the resources. So you can move all of these to done if we find issues we'll fix them on the go. I right now see no issues left except the skill. You can start 1."
<!-- SECTION:NOTES:END -->

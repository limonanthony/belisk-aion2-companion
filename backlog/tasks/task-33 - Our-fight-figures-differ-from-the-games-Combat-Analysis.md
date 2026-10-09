---
id: TASK-33
title: Our fight figures differ from the game's Combat Analysis
status: In Progress
assignee: []
created_date: '2026-10-09 10:52'
updated_date: '2026-10-09 11:58'
labels:
  - bug
dependencies: []
priority: high
type: bug
ordinal: 92000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Reported 2026-10-09 by the user with two screenshots (kept locally, not in the repository) comparing our fight report with the game's Combat Analysis (Ctrl+X) for one fight ("Fafnir's Poison Blood", Elementalist Ysmera). User: "And here is a comparison between the in game dps meter and ours, no idea what skill 16000000 is..."

## Evidence notes

Observed: total damage matches (2,282,703). The game shows 655 hits, Critical 46%, Perfect 24%, Front 89%, Back 5%, Double 5%, Multi-Hit 24%. Ours showed the same counts over 778 hits (38.8%, 19.8%, 75.2%, 4.0%, 3.9%, 20.6%). The game lists "Jointstrike: Curse" (13 hits, 106,148) and "Ancient Spirit: Destruction" (8 hits, 94,548); ours showed a row "Skill 16000000" (29 hits, 170.0K).

Cause (found in the recording of that fight):

1. We counted damage-over-time ticks as hits. The game adds their damage to the skill but does not count them as hits.
2. "Skill 16000000" grouped four different spirit skills (Fire Spirit: Flame Explosion, Water Spirit: Water Bomb, Earth Spirit: Headbutt, Ancient Spirit: Destruction) under one unnamed id.
3. A game record sent when an Elementalist summons a spirit (shown as "Skill 16990000", TASK-4) was counted as damage on the Session board.

## Open, waiting for more game screenshots from the user

- The fight time: ours 1:48, the game's 2:00, so DPS 21.1K vs 18,874. No rule found yet.
- The game's Total Heal (5,937,798 vs our 8,703). Most of it matches the spirit records above.
- How the game lists other grouped skills (Jointstrike: Corrode, Ancient Spirit: Break, Fire Spirit: Rage Burst).

## Related

TASK-4, TASK-15.4.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Hit counts and hit rates in the fight report and Combat Analysis equal the game's for the same fight (damage-over-time ticks add damage, not hits).
- [ ] #2 Spirit skills show under their own names, not "Skill 16000000".
- [ ] #3 The spirit records shown as "Skill 16990000" no longer count as damage anywhere.
- [ ] #4 Total damage is unchanged.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: user decision on the fight time / DPS difference (ours 1:48 and 21.1K, the game's 2:00 and 18,874): 'For the dps, I think we'll improve this later still (i plan on doing a full pass where I compare the fight with the in game dps meter + ours. If there are small discrepenties for now it's not that much of an issue as long as the user understand what it sees'. The fight time stays first-to-last hit on the boss for now; matching the game's battle time waits for the user's comparison pass. Built since the last note (agent-tested, reviewed, not yet tested by the user): damage-over-time ticks add damage but not hits; spirit skills 16001104/16001108/16001116/16001117 show under their own names; records on the player's own side (the spirit records shown as 'Skill 16990000') no longer count as damage. Replay of the compared fight: 655 hits, Critical 46%, total 2,282,703, Jointstrike: Curse 13 hits 106,148, Ancient Spirit: Destruction 8 hits 94,548.
<!-- SECTION:NOTES:END -->

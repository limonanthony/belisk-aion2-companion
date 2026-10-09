---
id: TASK-32
title: The same buff from the same player shows as several rows
status: In Progress
assignee: []
created_date: '2026-10-09 10:21'
updated_date: '2026-10-09 11:58'
labels:
  - bug
dependencies: []
priority: medium
type: bug
ordinal: 91000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The buffs list shows one buff several times when it comes from the same player. The player expects one row per buff per caster, as in the game.

**Reported** 2026-10-08: "Still have the bug for multiple skills showing up as different skills inthe buffs btw..."

**Observed** (user screenshot of a Buffs list, kept locally, not in the repository): every row was cast by the user's Elementalist Ysmera. "Spirit's Descent" appears twice (0:41 at 58% and 0:35 at 49%) and "Element Unification" appears twice (0:07 at 10% each), next to other buffs that appear once.

**Expected**: one row per buff per caster.

**Cause**: not known yet, under investigation. Hypothesis: several buff ids resolve to the same skill name.

**Direction**: rows that show the same buff name from the same caster are merged. The merged uptime counts overlapping time once (never a sum, never above 100%). The merge happens when the list is shown, so fights already saved also show one row. Every window that lists buffs follows this.

Related: TASK-15.4 (player details in the fight report), TASK-21.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A buff applied under several ids by the same player shows as one row with its name.
- [ ] #2 The merged row's uptime counts overlapping time once and never exceeds 100%.
- [ ] #3 Buffs from different players stay separate rows.
- [ ] #4 Fights saved before the fix also show one row per buff.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: built and reviewed (not yet tested by the user): one row per buff name per caster; new fights count overlapping time once (union); fights saved before show the longest of the merged ids.
<!-- SECTION:NOTES:END -->

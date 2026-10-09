---
id: TASK-25.8
title: Damage-over-time ticks skip the player-skill filter
status: In Progress
assignee: []
created_date: '2026-10-08 12:35'
updated_date: '2026-10-09 14:03'
labels:
  - bug
dependencies: []
parent_task_id: TASK-25
priority: high
type: bug
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Damage-over-time ticks may be counted as damage by the wrong actor. A monster's damage-over-time on a player can be recorded as "damage dealt" by the monster's actor id, because those ticks skip the filter that normal hits go through. Whether this changes what the board shows is not confirmed.

## Why it happens

`parse_dot_packet` (`crates/core/src/capture/parser/damage.rs`, around lines 62-148) first records heal ticks and monster damage ticks correctly through `record_periodic_effect`. It then reads the same bytes a second time by hand, and for any damage tick adds it as damage dealt by the actor, with no player-skill check. The normal hit path (`damage_event`) does check `is_player_skill_code`, and also filters reflected damage. A code comment says the tick "still goes through the damage path below, as before", so the second read is a leftover from before the periodic records existed.

## Evidence

Verified by reading the code (commit 7a48824): the missing filter and the double read. The real impact on the board is an inference: live monster damage-over-time packets (the `05 38` kind) have not been checked. Check with a recording that contains monster damage-over-time before deciding how serious it is.

## Suggested direction (Proposed, not agreed)

One path driven by the parsed periodic record, using the same player-skill filter (and the same Thamon exclusion) as the normal hit path; remove the second hand-written read.

## How to verify

A regression test with a monster-skill `05 38` tick that must not increase damage dealt (it fails before the fix); a replay comparison of totals before and after on a recording that has player damage-over-time.

## Related

TASK-16 (Thamon boss effect counted as player damage, Done): same kind of mistake, handled on the normal hit path. TASK-14.4 (read damage-over-time, Done), TASK-21 (damage taken, Done).

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A damage tick from a monster skill is not counted as damage dealt by a player or by the monster's actor.
- [ ] #2 Damage ticks from player skills are still counted with the same totals as before.
- [ ] #3 Heal ticks and damage taken from monster ticks are unchanged.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: still real at the start of today (a monster's damage-over-time tick on a player was also counted as damage dealt by the monster). Fixed: one reader for these ticks and one rule deciding monster vs player for both damage dealt and damage taken; Thamon absorption excluded in both fields; heal ticks unchanged. Check on 36 recordings: the new reader accepts the same 21,203 ticks as the old one. Reviewed, not yet tested by the user.
<!-- SECTION:NOTES:END -->

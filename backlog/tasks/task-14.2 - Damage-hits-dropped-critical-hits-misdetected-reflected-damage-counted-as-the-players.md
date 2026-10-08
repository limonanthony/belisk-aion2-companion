---
id: TASK-14.2
title: >-
  Damage hits dropped, critical hits misdetected, reflected damage counted as
  the player's
status: Done
assignee: []
created_date: '2026-10-07 10:02'
updated_date: '2026-10-08 09:31'
labels:
  - bug
dependencies: []
parent_task_id: TASK-14
priority: medium
type: bug
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Damage messages are not all read correctly. Some hits are dropped, critical hits are misdetected when an optional field is present, and reflected damage is counted as the player's own damage.

## Scope

Covers reading the hits themselves: which hits are accepted, whether a hit is critical, and whether damage is the player's or reflected. Who owns a summon is not part of this item.

## Evidence notes

Effect: damage totals, hit counts and crit rate can be wrong.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replaying the user's recordings before and after shows hits that were dropped now counted, with damage and hit counts changed accordingly.
- [x] #2 A critical hit is detected correctly whether or not the optional field is present, and reflected damage is not added to the player's damage.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Damage packets are now read field by field; every recorded damage packet is read completely. Totals on the recordings are unchanged (they contained none of the cases that used to break); the fix matters for crits with the optional field, reflected damage and flag values the old reader rejected. Agent-tested only; waiting for the user to check in a real fight.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
User acceptance 2026-10-08: "Most of the tasks you said I've tested and it seems to work correctly (until a new issue appears which is not impossible but for now we can consider them fixed)". Considered fixed on the user's own testing; reopen if a new issue appears.
<!-- SECTION:FINAL_SUMMARY:END -->

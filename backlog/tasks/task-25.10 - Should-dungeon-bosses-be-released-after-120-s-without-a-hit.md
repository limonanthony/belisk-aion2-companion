---
id: TASK-25.10
title: Should dungeon bosses be released after 120 s without a hit?
status: To Do
assignee: []
created_date: '2026-10-08 12:35'
updated_date: '2026-10-12 15:05'
labels:
  - research
dependencies: []
parent_task_id: TASK-25
priority: low
type: task
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Question

Should a dungeon boss be released from focus after 120 seconds without a single hit on it?

## Why it comes up

In `crates/app/src/meter/focus.rs`, a quiet-release timer (`QUIET_RELEASE`, 120 seconds) applies to every focused boss, dungeon bosses included. After two minutes without one hit on the boss the focus is released and a provisional summary is shown.

Locked product rules: a dungeon focus stays on a detected boss until it is dead; while the boss is alive and taking damage, the focus must not be released because the adds are cleared or because a fixed timer ran out. The 120 s release fires only when the boss is not taking damage, so it may be consistent with the rule; it is the user's call. The review found no 15 s timer (the 20 s `SWITCH_AFTER` is for switching from one boss to another, and `MIN_FIGHT_SECS` is about summary eligibility), so that earlier regression is not present.

## Evidence

Verified by reading the code (commit 7a48824). The comment in the code says the provisional summary can follow a pause between phases.

## Decision needed (not decided)

- Option A: keep the quiet release for dungeon bosses as it is.
- Option B: skip the quiet release for dungeon bosses, keeping it for open-world bosses. The game data already maps a boss to its dungeon.

## Suggested direction (Proposed, not agreed)

None from the review beyond Option B being the way to implement a "no release" answer. Record the user's answer in this ticket; if Option B, add the change and a test as part of this ticket.

## How to verify

If Option B: a focus-model test with a dungeon boss and a pause longer than 120 s (it fails before the change), and a manual check by the user in a dungeon with a long phase pause.

## Related

TASK-3 (dungeon boss not detected in one run, In Progress).

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; the meter changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The user's decision is recorded in this ticket.
- [ ] #2 If the decision is to skip the quiet release for dungeon bosses: a dungeon boss that takes no damage for more than 120 s stays focused until it dies, and an open-world boss is still released.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Not reviewed by the review of this date (it is app meter focus code); unchanged. Still an open question for the user.
<!-- SECTION:NOTES:END -->

---
id: TASK-25.1
title: 'The overlay stops updating between hits (names, boss HP, buffs)'
status: To Do
assignee: []
created_date: '2026-10-08 12:35'
updated_date: '2026-10-08 12:35'
labels:
  - bug
dependencies: []
parent_task_id: TASK-25
priority: high
type: bug
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The overlay keeps showing the old frame between hits. A player's name that arrives while nobody is dealing damage does not appear, the boss HP bar and phase look frozen during a pause, and deaths and buff changes are not shown, until someone deals damage again. This breaks the product rule that player names stay visible and tracked through pauses in combat.

## Why it happens

The meter core runs a snapshot thread (`crates/core/src/engine/meter.rs`, snapshot loop around lines 710-728) that builds a frame every 200 ms but sends it to the app only when the total damage differs from the last frame sent. Name packets, boss HP updates, deaths and buffs change no damage total, so those frames are built and then dropped. This behaviour was inherited from the original project (NOIA2, commit `b8893c3`).

Two side effects: the "last snapshot" kept for readers (`get_last_snapshot`) is updated even when the frame is not sent, so such readers can see a newer frame than the overlay does; and the damage calculator (`crates/core/src/engine/calculator.rs`) still keeps a `last_total_damage` field that nothing reads, left over from an older version of this suppression.

## Evidence

Verified by reading the code (commit 7a48824). Line references: `engine/meter.rs` 710-728; calculator `last_total_damage` around lines 14, 25-27, 116-117.

## Decision needed (not decided)

- Option A: always send the frame at the snapshot interval (200 ms). Smallest change; the review recommends it.
- Option B: send only when a change counter moved (damage, names, HP, buffs, activity).

## Suggested direction (Proposed, not agreed)

Option A, and remove the unused `last_total_damage` field. Make the "last snapshot" readers and the overlay see the same frame.

## How to verify

- Replay a recording in which a name packet arrives during a pause in damage (`--replay`): the name appears on the overlay without any new damage.
- A regression test on the core side: a name-only (or HP-only) update produces a frame that reaches the app. It must fail before the fix.

## Related

TASK-25.2 (hide when idle has no effect: same overlay area, different cause). TASK-19 (meter stops reading after a lost packet, Done) is a different cause of a stale overlay.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A player name that arrives while no damage is dealt appears on the overlay without waiting for new damage.
- [ ] #2 Boss HP and death changes during a pause in damage are shown on the overlay.
- [ ] #3 A core-side regression test covers a name-only update reaching the app and fails before the fix.
<!-- AC:END -->

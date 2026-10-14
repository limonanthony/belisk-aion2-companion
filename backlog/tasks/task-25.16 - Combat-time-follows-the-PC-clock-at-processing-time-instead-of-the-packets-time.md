---
id: TASK-25.16
title: >-
  Combat time follows the PC clock at processing time instead of the packet's
  time
status: In Progress
assignee: []
created_date: '2026-10-08 12:39'
updated_date: '2026-10-12 15:05'
labels:
  - bug
dependencies: []
parent_task_id: TASK-25
priority: medium
type: bug
ordinal: 53000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Fight durations and DPS are measured with the PC's clock at the moment a packet is processed, not with the time the packet was captured. Replaying a recording therefore measures how fast the replay ran, not how long the fight was: at any replay speed other than real time the durations (and so DPS) are wrong. The idle timeout and buff coverage use the same clock, and a clock jump (network time step, sleep and resume) distorts live fights too.

## Why it happens

Live ingest stamps each hit with `SystemTime::now()` (`crates/core/src/storage/data_storage.rs`, around lines 383-384 and 1423-1429). Replay feeds the same ingest path, and the captured-at time carried by each recorded packet (`CapturedPacket.captured_at`) is not used at ingest. A fight's duration is `max_last_time - start` with a floor of 1 s (`engine/calculator.rs` around lines 507-509), so an unpaced replay that drains a minutes-long fight in a few seconds produces an inflated DPS. The idle timeout and the buff "active" bit read the same clock (`engine/meter.rs` around lines 78-82 and 691). Calculator and meter take no clock parameter, so tests have to use the `append_damage_at` helper to control time. The example `crates/core/examples/replay.rs` prints the inflated snapshot.

A second, related problem: `start_time_by_target` and `last_time_by_target` are plain maps updated on every hit, never evicted, and cloned in full on every snapshot, while the combat totals (`dps_stats`) are bounded to 512 targets. After 512 distinct targets the DPS rows disappear while times and identity linger, the last target can point at an evicted id and the overview becomes empty. The PvP maps are also unbounded and survive a reset, which is not a stated product rule.

## Evidence

Reported by the review (static reading), not re-verified. The review marks as an inference whether unpaced replay is a path a user can reach (the API exists).

## Suggested direction (Proposed, not agreed)

Pass the packet's captured-at time (or a small clock interface with one test fake) into the ingest path; use a monotonic clock for idle and peek timing and keep epoch seconds only for persisted "when" values. Key the per-target time maps on the same bounded target set or evict them together with `dps_stats`. Do not keep PvP maps across a user reset unless the owner says that is wanted.

## How to verify

A replay of a recording at a speed other than 1 gives the same fight duration and DPS as at speed 1 (independent expected value: the span between the first and last recorded hit). A unit test with a fake clock covers the idle timeout. A test that ingests more than 512 distinct targets checks the time maps do not grow past the same bound.

## Related

Related TASK-25.1 (the overlay stops updating between hits: also a time and snapshot-loop issue, different cause), A4 (TASK-25.15, shared storage). The recordings from TASK-17 are the usual replay input. T1 (tests for the snapshot calculator, to be created later) covers the 1 s floor.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replaying one recording at two different speeds gives the same fight durations and DPS.
- [ ] #2 Idle timeout and buff coverage can be tested with a controlled clock.
- [ ] #3 Time bookkeeping for targets does not grow without limit over a long session of many distinct targets.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Fixed in code, mostly. Hits, heals, deaths and the idle timeout now use the packet's capture time (plus real time elapsed since the newest packet, so a quiet live connection still ages); a fixture test replays a real fight at 50x and 200x and gets the same combat time, DPS and rows (AC #1), and the clock is a data clock a test can control (AC #2). Not changed or not checked: the time bookkeeping per target (AC #3) was reported unbounded by the review and no fix for it was reported; buff apply still uses the PC clock; the data clock follows the last packet, not the newest capture time, so out-of-order flows can rewind it briefly. Moved to In Progress; not tested by the user.
<!-- SECTION:NOTES:END -->

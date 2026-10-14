---
id: TASK-25.19
title: >-
  Capture hot path allocates string keys and repeats copies and scans for every
  packet
status: To Do
assignee: []
created_date: '2026-10-08 12:40'
updated_date: '2026-10-12 15:05'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: low
type: enhancement
ordinal: 56000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Work is done on every captured packet that has nothing to do with combat: string keys are allocated per packet, the same byte scans are repeated, and some helper code is copied. It wastes time on the busiest thread and one leftover causes a visible stale value (ping after a reset).

## Why it happens

Reported by the review:

- The flow identity used as the map key is a formatted string `"{ip}:{port}->{ip}:{port}"` allocated for every packet (`capture/dispatcher.rs` around lines 20-24, 275-281, 538-547), while a proper `TcpFlowKey` already exists for reassembly. Lookups and combat-port compares are string compares.
- The heartbeat bytes and the TLS content-type table are copied in the dispatcher, both capturers, packet-explorer and the rolling filter (see A3).
- Every remaining-HP packet (`00 8D`) clones the whole actor-name map (`actor_id_name_snapshot()`, `parser/summon.rs` around lines 109-112) only to test whether a key exists.
- The ping tracker (`capture/ping_tracker.rs`) scans every reassembled non-web chunk for `03 36 00 00` before combat flows are selected, and appends to a history of 10,000 entries that nothing reads. Its `reset()` has the line that clears the last ping commented out, so the overlay shows the old ping after a fight reset.
- The dispatcher scans every packet for the heartbeat marker.
- A relaxed atomic in the census is fine (no change).

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

Use the directional flow key as the map key; one module for the heartbeat and web-flow checks; ask the storage whether a key exists instead of cloning the map; gate the ping and marker scans to candidate flows; remove the unread ping history and make reset clear the last ping. Measure before and after on a busy recording.

## How to verify

Replay results are unchanged. After a fight reset the ping shown is cleared (a test on the tracker). The packet-processing time of a large recording is no worse, and allocations per packet are lower (measured, not assumed).

## Related

A3 (TASK-25.14, the copies of the marker). TASK-25.7 (dropped packets) and TASK-25.9 (buffer copies) are the larger capture-path costs.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Flow maps use a typed key rather than a formatted string per packet.
- [ ] #2 A remaining-HP packet no longer copies the actor-name map.
- [ ] #3 After a fight reset the overlay's ping value is cleared, or the owner confirms it should persist.
- [ ] #4 Replay totals are unchanged and per-packet cost is measured no worse.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Partly fixed in code this round: the remaining-HP parser no longer copies the actor-name map (AC #2); a fight reset clears the overlay ping and the unread ping history is gone (AC #3); debug log lines are no longer built when debug logging is off. Still present: string flow keys in the dispatcher (AC #1) and per-packet cost not measured (AC #4).
<!-- SECTION:NOTES:END -->

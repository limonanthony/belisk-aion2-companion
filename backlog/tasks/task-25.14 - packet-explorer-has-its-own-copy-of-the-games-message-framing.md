---
id: TASK-25.14
title: packet-explorer has its own copy of the game's message framing
status: To Do
assignee: []
created_date: '2026-10-08 12:38'
updated_date: '2026-10-12 15:05'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: medium
type: enhancement
ordinal: 51000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The packet-explorer tool carries its own copy of the game's message framing. A framing fix in the core does not reach it, and its `parity` command compares against slightly different rules than the meter uses, so it can disagree with the meter.

## Why it happens

`crates/packet-explorer/src/capture/framer.rs` copies the core's `consume_stream`, `unwrap_bundle` and extra-flag prefix handling from `crates/core/src/capture/processor.rs`, but its stall timeout is a fixed 5 s while the core uses its configured value with a 1 s cap. The heartbeat marker bytes (MAGIC, `0x0E 0x00 0x36`) and the check for TLS traffic are also copied in five places: the dispatcher, the capturer, the WinDivert capturer, packet-explorer and the rolling-recorder filter. The explorer reaches into the core through the public `parser::utils`, and `commands/parity.rs` builds a full `StreamAssembler` and `DataStorage` and toggles the process-wide census. `commands/tickets.rs` (about 926 lines) re-walks message types with its own varint reads.

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

One shared framer used by both the core and packet-explorer, and one small module for the heartbeat and TLS helpers instead of five copies. Make `parser::utils` crate-private and give the explorer a narrow framing interface (or a test helper) instead. Keep the explorer away from `DataStorage` except for an explicit `parity` command.

## How to verify

A framing change made in one place changes both the meter and the explorer. The explorer's `parity` and the meter give identical output for the same recording. The five copies of the marker constants are reduced to one.

## Related

A2 (TASK-25.13) restructures the same processor; A1 (TASK-25.12) decides what the explorer may import; A8 (TASK-25.19) removes the per-packet string keys and the same helper copies.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Packet-explorer and the meter frame the stream with the same code, including the stall timeout.
- [ ] #2 The heartbeat marker and the TLS check each exist once.
- [ ] #3 packet-explorer's parity command gives the same result as the meter on the same recording.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Still present per the review (packet-explorer framer: fixed 5 s stall against the core's 1 s cap, heartbeat and TLS checks copied); not touched this round.
<!-- SECTION:NOTES:END -->

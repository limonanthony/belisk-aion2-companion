---
id: TASK-25.9
title: >-
  The stream buffer copies on every parse step, wipes silently at 50 MB, and
  every flow is parsed twice
status: To Do
assignee: []
created_date: '2026-10-08 12:35'
updated_date: '2026-10-08 12:35'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: medium
type: enhancement
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The game stream buffer wastes work on every parse step and, when it grows past 50 MB, throws away everything without a trace and without resetting the framing state, which can leave the reader out of step with the stream. In addition, every game flow is parsed twice.

## Why it happens

- `PacketAccumulator` (`crates/core/src/capture/accumulator.rs`) is a mutex-protected byte buffer even though one thread uses it. `StreamAssembler::process_chunk` (`assembler.rs`, around lines 39-53) copies the whole buffer on every loop iteration before parsing.
- When appending would pass 50 MB, the buffer is cleared and the new data is dropped: no log, and the framing state is not reset. The proper discard path (`discard_buffered`) does reset it.
- The dispatcher keeps two assemblers per flow (`assemblers` and `nickname_assemblers`, `dispatcher.rs`, around lines 400-419). Both parse the same bytes into the same `DataStorage`, so work is done twice and main-actor detection can run twice.

## Evidence

Verified by reading the code (commit 7a48824) for the copy on every step and the silent clear at 50 MB. The double-parsing by two assemblers per flow is reported by the review, not re-verified.

## Suggested direction (Proposed, not agreed)

Let the assembler own the buffer and parse in place; on overflow, take the `discard_buffered` path and log it; keep one assembler per flow (nickname parsing as a mode of it).

## How to verify

A test that overflow resets framing and logs (it fails before the fix); a replay comparison of totals and names before and after; a check that nickname-only flows are still detected.

## Related

TASK-19 (meter stops reading after a lost packet, Done), TASK-25.7 (dropped packets, same capture path). The planned parser/processor split ticket (A2, not yet created) depends on this one.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Parsing a chunk does not copy the whole buffered stream.
- [ ] #2 An overflow past the limit is logged and resets the framing state so parsing resumes on the next valid message.
- [ ] #3 Each flow is parsed once; damage totals and detected players are the same as before on a replay.
<!-- AC:END -->

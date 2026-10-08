---
id: TASK-25.7
title: Capture can silently drop packets while the dispatcher writes recordings
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
ordinal: 44000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The meter can lose game packets without any sign. Lost bytes break the reassembly of the game's TCP stream, so hits are missed or the stream stalls (the same family of symptom as TASK-19, which is done). There is no counter or log that would show it.

## Why it happens

- Both capturers hand each packet to the dispatcher queue with a non-blocking send and ignore a failure (`crates/core/src/capture/capturer.rs`, around line 608; `windivert_capturer.rs`, around line 164). When the queue is full, the packet is dropped with no counter and no log.
- The single dispatcher thread writes the session recording and the rolling recording before it reassembles the stream (`dispatcher.rs`, around lines 258-261). The rolling recorder rotates files and prunes the recordings folder (up to 2 GiB of listing and deleting) on that same thread (`rolling_recorder.rs`, around lines 126-157). A slow disk or a prune stalls the thread, the queue fills, and live game bytes are dropped.
- The Npcap capturer feeds every TCP packet into the queue; WinDivert feeds only the candidate game flows. The queue capacity is 2,000,000 packets, which is not a practical bound (gigabytes of payload).

## Evidence

Verified by reading the code (commit 7a48824). That the queue actually fills in practice is an inference: it has not been observed.

## Suggested direction (Proposed, not agreed)

Put recordings on their own bounded side channel (drop the oldest and count overflows), so the dispatcher never waits for recording; make Npcap forward only candidate flows as WinDivert does; add a drop counter visible in diagnostics; use a smaller queue.

## How to verify

A test or diagnostic showing no drops while an artificially slow recorder is attached; a replay smoke test that damage totals are unchanged.

## Related

TASK-19 (meter stops reading after a lost packet, Done), TASK-17 (session recording, Done). TASK-25.5 (history writes can also stall the dispatcher) and TASK-25.9 (stream buffer, same capture path).

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A slow or blocked recorder does not cause game packets to be dropped.
- [ ] #2 If packets are dropped anyway, the number dropped is counted and visible in diagnostics.
- [ ] #3 Hits are read the same as before in a normal session.
<!-- AC:END -->

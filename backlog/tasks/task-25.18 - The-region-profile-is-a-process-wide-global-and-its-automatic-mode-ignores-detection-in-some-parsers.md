---
id: TASK-25.18
title: >-
  The region profile is a process-wide global and its automatic mode ignores
  detection in some parsers
status: To Do
assignee: []
created_date: '2026-10-08 12:40'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: low
type: enhancement
ordinal: 55000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The region profile (which game server region's packet layout to expect) is a process-wide global, and in the automatic setting the detected region is not used everywhere. Two parsers can disagree about the same packet, parser tests depend on test order, and two meters cannot run in one process (for example the replay example next to a test).

## Why it happens

`crates/core/src/region.rs` (around lines 19-22, 75-86, 139-166) keeps the runtime region as a global; its comment says this is "on purpose" to keep the diff against upstream small. `resolved()` uses the configured region or the server-IP fingerprint and is what the spawn parser uses (`parser/summon.rs` around line 19). But `effective()` and `accepts_server_id()` return only `configured()`, so with the Automatic setting a detected Korea or Taiwan profile is never applied to the nickname server-id check (`parser/nickname.rs` around lines 340-343). Tests that call `observe_server_ip` or `set_configured` change process state.

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

Pass the region in the parser context instead of reading a global. Either make Automatic use `resolved()` everywhere or delete `effective()`. Be careful: region profiles are load-bearing, because the server id is how some fields are located in a packet. Any change needs a replay of recordings from each region that exists in the project's samples.

## How to verify

Replays of recordings from each supported region give the same names and spawns as before. A test can run two parsers with different regions in one process. A recording where Automatic detects a non-default region has its nickname server-id check agree with the spawn layout.

## Related

A11 (TASK-25.11): the global exists because of the upstream-diff rule. A2 (TASK-25.13) introduces the parser context this would use.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The nickname server-id check and the spawn layout use the same region decision for the same packet.
- [ ] #2 The region is passed to the parsers instead of read from a process-wide global.
- [ ] #3 Recordings from each supported region replay with the same results as before.
<!-- AC:END -->

---
id: TASK-25.22
title: Decide whether the meter may list running processes to find the game
status: To Do
assignee: []
created_date: '2026-10-08 12:41'
updated_date: '2026-10-12 15:05'
labels:
  - research
dependencies: []
parent_task_id: TASK-25
priority: medium
type: task
ordinal: 59000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The meter lists every running process on the PC to find the game's process ids, about once a second while the Npcap backend or the rolling recorder is in use. The project's own documents disagree on whether that is allowed, so the owner needs to decide. This is a policy choice about anti-cheat risk, not a bug.

## Why it happens

`crates/core/src/capture/game_connections.rs` (around lines 101-166) calls `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` and walks the process list to find `aion2.exe`, then calls `GetExtendedTcpTable(TCP_TABLE_OWNER_PID_ALL)` to map those process ids to TCP connections. It never calls `OpenProcess` and keeps no handle to the game; the only handle closed is the snapshot itself. (This was verified by reading the code.) The result is used for two things: the Npcap adapter hint (`capturer.rs` around line 771) and the rolling recorder's game-traffic filter (about 1 s refresh, `rolling_recorder.rs`).

Where the documents conflict: `the removed FORK.md (git history)` ("Staying clear of anti-cheat", around lines 542-551) accepts this Toolhelp listing explicitly as allowed because no handle on the game is opened. `AGENTS.md` says there must be no `sysinfo` refresh of all processes, which is the same kind of act (listing all processes), and the removed docs/GPUI_MIGRATION.md (git history) (around lines 320-321) repeats the "never touch a game process" wording. The capture reviewer called the listing a violation of "no process enumeration"; the architecture check found the project docs accept it and that the only other `OpenProcess*` in the core is on the meter itself (`preflight.rs`) and `sysinfo` is only used for the meter's own process id. Nothing was measured about whether any anti-cheat flags a Toolhelp listing.

## Evidence

The code facts above (Toolhelp snapshot plus extended TCP table, no `OpenProcess`) were verified by reading the code at commit 7a48824. The document conflict and the anti-cheat reading are reported by the review, not re-verified.

## Decision needed (not decided)

- Option A: keep the listing. Then the instructions (`AGENTS.md`) must say that a Toolhelp process listing is the accepted exception, so the two documents agree.
- Option B: remove it and identify the game's flows only by traffic (the heartbeat marker plus non-web ports, which the dispatcher and the rolling filter already implement). The extended TCP table could be kept only if it can be used without process ids; otherwise it goes too.

## Suggested direction (Proposed, not agreed)

The review marks this a policy choice and does not recommend one; it suggests the safer reading (Option B) if the owner wants zero process inspection, and Option A with a documentation fix if the current behaviour is acceptable.

## How to verify

The decision is recorded and `AGENTS.md`, `the removed FORK.md (git history)` and the removed docs/GPUI_MIGRATION.md (git history) say the same thing. If Option B: with no process listing the Npcap adapter hint and the rolling recorder still pick the game's flows on a recording from a live session, and the project's rolling-recorder filter tests still pass.

## Related

TASK-25.7 (dropped packets: the Npcap path and the rolling recorder filter are the callers). TASK-17 (whole-session recordings, Done) introduced the rolling recorder filter that uses this.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The decision (keep the process listing, or identify game flows only by traffic) is recorded.
- [ ] #2 AGENTS.md, the removed FORK.md (git history) and the removed docs/GPUI_MIGRATION.md (git history) agree with it.
- [ ] #3 If the listing is removed: the Npcap adapter hint and the rolling recorder still select the game's traffic.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Still present and undecided per the review. The docs (technical-notes, Staying clear of anti-cheat) now describe listing processes and reading the TCP table as allowed (no OpenProcess), but no decision is recorded here.
<!-- SECTION:NOTES:END -->

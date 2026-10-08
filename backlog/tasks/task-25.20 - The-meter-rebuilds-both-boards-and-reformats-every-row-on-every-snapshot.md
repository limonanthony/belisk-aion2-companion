---
id: TASK-25.20
title: The meter rebuilds both boards and reformats every row on every snapshot
status: To Do
assignee: []
created_date: '2026-10-08 12:40'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: low
type: enhancement
ordinal: 57000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The meter's UI does more work than it needs on every snapshot: it rebuilds both the session board and the target board, reformats every row's strings, redraws even if nothing changed, and copies settings and footer text on each pass. This is a cost on the path that runs about five times a second during a fight. It is not a reported slowdown.

## Why it happens

In the app, `meter/model.rs` `accept_snapshot` (around lines 585-619) always stores the snapshot, always calls `rebuild_live()` and always returns "redraw". `rebuild_live` (around lines 723-741) builds the session board and the live board: sorts players and formats damage, compact numbers, percent, duration, display name, shade and header for every row, including the board that is not shown. `footer()` (around lines 286-297) clones its strings each paint. The PvP model rebuilds its dealt and received lists on every meter notification before comparing; the detail model clones the overlay settings on every observe to compare (`detail/model.rs` around lines 43-45).

## Evidence

Reported by the review (static reading), not re-verified. The review says to measure before and after.

## Suggested direction (Proposed, not agreed)

Redraw only when the prepared boards, header or summary actually differ (comparing them is enough; no new cache layer). Build only the active board plus what other windows subscribe to. Store the footer on the model and return a reference. Compare the snapshot id in the PvP model before allocating rows. Pass a settings generation number into the detail inputs instead of cloning settings.

## How to verify

Measure frame preparation time and allocations on a replay of a long fight before and after. The overlay shows the same rows and numbers; switching boards still shows the right content immediately; the existing meter model tests pass.

## Related

TASK-25.1 (the overlay stops updating between hits) changes how often snapshots arrive; if it makes the core send a frame every interval, this ticket matters more. A15 (TASK-25.25, settings changes wake every consumer).

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A snapshot that changes nothing visible does not cause a redraw.
- [ ] #2 Only the board that is shown (and any window that subscribes to the other) is rebuilt per snapshot.
- [ ] #3 Frame preparation cost on a replay of a long fight is measured before and after and is lower.
<!-- AC:END -->

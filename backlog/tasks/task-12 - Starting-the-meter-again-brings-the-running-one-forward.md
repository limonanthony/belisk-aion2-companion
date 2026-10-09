---
id: TASK-12
title: Starting the meter again brings the running one forward
status: Done
assignee: []
created_date: '2026-10-06 23:07'
updated_date: '2026-10-09 12:46'
labels:
  - feature
dependencies: []
type: feature
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Players see each event reminder once, however many times the meter is started. Starting the meter while it is already running brings the running meter forward instead of opening a second copy.

User (2026-10-07), asked "Should starting the meter a second time just bring forward the one that's already running?": "Yes please bring forward the one running."

## Scope

Covers launching the meter while one is already running: no second meter starts, the running meter is brought forward, and the new launch ends.

Does not cover developer and test runs (screenshot captures, probes, replays). They must neither be blocked by a running meter nor block it, so captures keep running safely while someone plays.

## Evidence notes

Observed (2026-10-07): two copies of the meter were running at once (started 22:59 and 00:23:41), so every event reminder appeared twice, stacked. The app has no single-instance guard.

Related: TASK-10 (No Shugo Festival reminder reported for the midnight start), the reminder investigation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Launching the meter while one is running leaves exactly one meter process, and the running meter's window is brought to the front.
- [ ] #2 Only one card appears per reminder when the meter is launched twice.
- [ ] #3 Launching the meter after the previous one has closed (including after a crash) starts normally.
- [ ] #4 Screenshot capture and replay runs start and work while a meter is running, and do not bring it forward.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: user: "Task-12 was probably when the agent was doing test it ran it and I was seeing double notifications (i believe only the dps-meter at the time was hidden but notifications weren't) I believe it was fixed too." The app has a single-instance guard for live runs (crates/app/src/single_instance.rs, lib.rs): a second launch asks the running app to show its toolbar and ends; captures, replays and probes are not guarded.
<!-- SECTION:NOTES:END -->

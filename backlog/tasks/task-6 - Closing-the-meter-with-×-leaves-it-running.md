---
id: TASK-6
title: Closing the meter with × leaves it running
status: To Do
assignee: []
created_date: '2026-10-06 15:07'
labels:
  - bug
dependencies: []
type: bug
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Pressing × on the meter does not end the app: the process keeps running after the window is gone.

User (2026-10-06): "Why when I do the X for the aion dps meter it's still open ?"

## Scope

Covers quitting through the meter's × button: every meter window closes and the process exits. Other ways of leaving the app and what the meter shows while running are not changed here.

## Evidence notes

Observed: after pressing ×, aion-2-dps-meter.exe stayed alive and the user had to end it in Task Manager. While it was still running, the exe stayed locked, so rebuilds failed with "Access is denied".

Expected: pressing × closes the meter and ends the process.

Environment: reported 2026-10-06 on the user's local build.

Reproduction: not yet reproduced by an agent.

Cause: Unknown. Hypothesis from reading the code, not reproduced:
- The × calls the app's quit (crates/app/src/meter/view.rs:413).
- The quit hook stops the capture (crates/app/src/lib.rs:117), which joins the capture threads (crates/aether-core/src/capture/capturer.rs:390 and 565; crates/aether-core/src/capture/windivert_capturer.rs:185).
- A capture thread waiting for the next packet with no read timeout never returns when no game packets arrive, so quitting hangs.

Not fixed yet, by the user's choice: "Just do the tickets don't fix yet i'll test the todos first".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Pressing × closes every meter window and ends the process within about a second, whether or not the game is running or sending traffic.
- [ ] #2 After pressing ×, no aion-2-dps-meter.exe process is left running in Task Manager.
<!-- AC:END -->

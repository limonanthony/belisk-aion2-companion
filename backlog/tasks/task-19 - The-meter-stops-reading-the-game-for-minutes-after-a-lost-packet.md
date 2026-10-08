---
id: TASK-19
title: The meter stops reading the game for minutes after a lost packet
status: Done
assignee: []
created_date: '2026-10-07 19:23'
updated_date: '2026-10-08 09:31'
labels:
  - bug
dependencies: []
priority: high
type: bug
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The meter stopped reading the game for about 9 minutes during play, even though the game kept sending data.

**Observed** on 2026-10-07 (SEEN in the meter's log and in the user's whole-session recording): from 20:51 to 21:00 the game kept sending data on the same connection, but the meter stopped reading it. It reset the fight as idle at 20:54, and only recovered when the game opened a new connection at 21:00.

**Suspected** (INFERRED, being confirmed on the recording): when one piece of network data is lost, the meter waits for it until a large amount of later data has piled up (about 9 minutes at normal traffic) instead of giving up after a few seconds.

**Effect:** damage, names and fights are missing during that time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A lost piece of data costs at most a few seconds of reading.
- [x] #2 The recording of 2026-10-07 20:37–21:07 replays without the 9-minute gap.
- [x] #3 Totals of other recordings are unchanged.
- [x] #4 The user sees no long blind periods in play.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-07 finding: on 22:57-23:18 the Transcendence bosses were not detected for about 6 minutes, twice. Cause: the running exe was the 19:50 release build, which does not have this fix (its log has none of the fix's messages). Replaying the recording with the fix gives continuous data.
Also found: HTTPS downloads (port 443) that happened to contain the game's marker were taken for game traffic (log "capture target detected ... port=443") and probably overloaded the capture. Fixed (agent-tested): ports 443 and 80 are never treated as the game. The user still has to confirm with a build that has the fix.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
User acceptance 2026-10-08: "Most of the tasks you said I've tested and it seems to work correctly (until a new issue appears which is not impossible but for now we can consider them fixed)". Considered fixed on the user's own testing; reopen if a new issue appears.
<!-- SECTION:FINAL_SUMMARY:END -->

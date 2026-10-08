---
id: TASK-17
title: Record whole play sessions for investigating game events
status: In Progress
assignee: []
created_date: '2026-10-07 17:21'
updated_date: '2026-10-07 22:53'
labels:
  - feature
dependencies: []
priority: medium
type: feature
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Automatic recordings last 2 minutes today and missed every Thamon fight; TASK-16 needs a full Thamon recording, and other features need longer recordings to investigate game events. The user asked for longer packet recordings and chose to always record the whole session (rolling), over per-boss-fight recordings, longer unfamiliar-server recordings, or manual-only.

Defaults chosen by Main (the user can change any of them):
- A Settings → Capture switch "Record every session", off by default for new installs.
- Only the game's own traffic is stored; today a recording keeps every TCP packet the network adapter sees.
- One file per 30 minutes.
- The newest files are kept under 2 GB in total; the oldest rolling files are removed first.
- Recordings started by hand and the unfamiliar-server ones are never deleted by this clean-up.
- Files stay on this PC and replay like any other recording.

Related: TASK-16 (needs a full Thamon recording). Not in scope: per-boss-fight recordings, longer unfamiliar-server recordings.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With "Record every session" on, a whole play session is on disk as consecutive files of about 30 minutes each, and the files replay correctly in order.
- [ ] #2 A rolling recording contains only the game's own traffic, not other TCP packets seen by the network adapter.
- [ ] #3 Total disk use of rolling recordings stays under 2 GB, and the oldest rolling files are removed first.
- [ ] #4 Recordings started by hand and unfamiliar-server recordings are never removed by the clean-up.
- [ ] #5 With the switch off, nothing extra is recorded; the switch is off by default for new installs.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-07 finding: two whole-session files stopped at the 512 MB per-file limit after about 20 minutes (22:37-22:59 and 23:07-23:18), instead of covering 30 minutes, because HTTPS downloads (port 443) were recorded as game traffic. Fixed (agent-tested): a whole-session file now continues into a new file when it reaches the limit, and HTTPS traffic is no longer recorded. Not yet confirmed by the user with a new build.
<!-- SECTION:NOTES:END -->

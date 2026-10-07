---
id: TASK-15.1
title: Mockups for the fight report and player details
status: Done
assignee: []
created_date: '2026-10-07 12:14'
updated_date: '2026-10-07 14:40'
labels:
  - feature
dependencies: []
parent_task_id: TASK-15
priority: medium
type: task
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Before anything is built, the user sees and approves how the fight report will look.

## Scope

Mockups for every screen that changes:

- The button's entry point(s). Where it appears is still an open question on the parent (boss summary card only, or also History for past fights); the mockups show the options so the user can choose.
- The fight report: damage and DPS, heals, damage taken and deaths (who died and what killed them) for every player.
- The player detail view: per-skill damage, critical and other hit types, buff and skill uptime in seconds.
- Any existing screen that changes because of the above.

Not included: a boss-health graph or a hit-by-hit timeline (the user ruled these out).

Part of the parent work item. The existing approved designs in `docs/gpui-mockups/approved/` (including the detail and history screens) are the starting point.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The user has approved mockups of every screen that changes: the button's entry point(s), the report, the player detail view, and any changed existing screen.
- [x] #2 The approved mockups are stored in docs/gpui-mockups/approved/.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The mockup round has started (2026-10-07).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The user approved the fight-report mockups on 2026-10-07 ("yeah use a crown, you can start once you're done cleaning up the mockups"). The approved design is in docs/gpui-mockups/approved/: report.html (with report.css and report.js) for the report and player details, meter.html for the 'Open the fight report' button on the summary card, history.html for the report button and crown on boss records, and tokens.md section 'Fight report' for the values. The history of the round stays in docs/gpui-mockups/fight-report/.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-20.3
title: Fill to-do counts from the game
status: In Progress
assignee: []
created_date: '2026-10-07 20:31'
updated_date: '2026-10-08 16:53'
labels:
  - feature
dependencies:
  - TASK-20.2
parent_task_id: TASK-20
priority: medium
type: feature
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build the approved design: to-do rows backed by a game ticket show the game's value and stay current; the rest stay manual.

Part of the parent work item. Waits on the approved mockups.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rows backed by a ticket show the game's value after login and follow spends and refills, as shown in the approved mockups.
- [ ] #2 Rows the game does not send stay manual.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Update 2026-10-08: the core part is built (not committed, not yet reviewed): the meter decodes the game's ticket list and ticket changes and ties them to the player's character. The to-do window part waits for the mockup approval (TASK-20.2).

Built 2026-10-08, not committed, awaiting the user's test: the game's ticket values fill the to-do window and the new Resources window; a horizontal toolbar is the hub (meter, PvP, to-dos, History, reminders, Settings; Odyle and Shugo keys counts; ⚠ dropdown of full resources); only the toolbar opens at launch ("Open the meter at launch" in Settings → Overlay, off by default; one-time prompt); the meter's "…" holds only the boss setting; fights are tracked only while the meter is open (closing it mid-fight ends and saves the fight like the idle timeout); tray icon (Show toolbar / Quit); Alt+Shift+CapsLock shows/hides everything; the daily reset now follows the game's 07:00 UTC across summer/winter time unless the player picked another hour. Independent review accepted after fixes. Open data still to confirm in game: Deus/Arkanis cap (built 4), Conquest tickets (built per dungeon), Balaur/Fafnium ids, Ordeal ticket.
<!-- SECTION:NOTES:END -->

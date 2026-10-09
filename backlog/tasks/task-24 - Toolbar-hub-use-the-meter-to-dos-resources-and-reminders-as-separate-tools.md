---
id: TASK-24
title: 'Toolbar hub: use the meter, to-dos, resources and reminders as separate tools'
status: Done
assignee: []
created_date: '2026-10-08 12:33'
updated_date: '2026-10-09 09:48'
labels:
  - feature
dependencies: []
priority: medium
type: feature
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The app is becoming a multi-tool for Aion rather than only a DPS meter. Today the meter overlay is the hub: Settings, To-dos, History and PvP sit in its "…" menu, and closing it quits the app. Players who only want the to-dos or the resource counts have to run the meter to get them.

Agreed (user, 2026-10-08): "some people might want to have the todo working without the dps-meter"; the app is "moving into a 'swiss multi tool' for aion instead of just a dps meter".

## Toolbar

- A small horizontal floating toolbar. User: "Let's do horizontal only because of the counts".
- Icons for the DPS meter, the PvP meter, To-dos, History, Reminders and Settings.
- The Odyle and Shugo keys counts are always shown. User: "Maybe we could put Odyle, Shugo Keys and show a warning icon with a dropdown with the others that are full and will be replenished soon ?"
- Clicking a count, or "All resources" at the end of the dropdown, opens the resources window (separate item, subtask of TASK-20).

## Launch and the meter

- At launch the toolbar opens alone. User: "It opens alone, maybe we a setting to open the dps meter automatically and a prompt for the first time the user opens the dps meter if he wants to open by default the dps meter when running the program ? (With a path on where he can find the setting ... e.g: Settings -> Overlay -> etc...)".
- The meter's "…" menu keeps only the boss setting. User: "Let's keep the ... with just not a boss, this is a boss and the other option just in case so that if there is an update users don't have to wait for an update for it to work directly". History, To-dos, Settings and PvP move to the toolbar.

## Capture

User: "We can keep the capture but if the dps meter is not open we do not handle dmg capture, etc... We only need to capture damage, heals, damage taken, death if the dps meter is open otherwise we do not need it. Currency should always be up-to-date though when we have the data (unless we also allow deactivating in the settings but I believe it's not very costly)".

## Shortcuts, tray, closing (delegated: "I'll let you decide")

- Alt+CapsLock keeps locking the meter.
- Alt+Shift+CapsLock shows or hides the toolbar and the open tools.
- A tray icon with "Show toolbar" and "Quit".
- Closing the toolbar quits the app.

## Derived

- The approved but never built "Home" design (docs/gpui-mockups/approved/home.html) is superseded in intent by the toolbar.
- Mockups are in progress in docs/gpui-mockups/hub/ and must be approved before building.
- The product name is being changed separately (TASK-23, the user's record); this item does not depend on it.

## Related

TASK-20 (game-filled counts and the resources window), TASK-1 (reminders), TASK-2 (to-dos), TASK-6 (closing the meter with × today).

## Open question

Order of building against TASK-20. Main proposed: shared resource values first, then the to-do rows and the resources window, then the toolbar, tray, optional meter and capture rule. The user approved starting the mockups but has not explicitly confirmed this order.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 At launch only the toolbar opens; the meter opens too only when the automatic-open setting is on.
- [ ] #2 The meter, PvP meter, To-dos, History, Reminders and Settings each open and close from their toolbar icon, and the toolbar never takes keyboard focus from the game.
- [ ] #3 With the meter closed, no fight is tracked (no damage, heals, damage taken, deaths, history entries) while resource counts stay up to date.
- [ ] #4 The meter's "…" menu holds only the boss setting (this is a boss / not a boss / automatic).
- [ ] #5 The first time the user opens the meter a prompt asks whether to open it at startup and names where the setting is; it does not appear again.
- [ ] #6 Alt+CapsLock locks the meter, Alt+Shift+CapsLock shows or hides the toolbar and open tools, and the tray icon offers Show toolbar and Quit.
- [ ] #7 Closing the toolbar quits the app.
- [ ] #8 The user approves the mockups before the toolbar is built.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Built 2026-10-08, not committed, awaiting the user's test: the game's ticket values fill the to-do window and the new Resources window; a horizontal toolbar is the hub (meter, PvP, to-dos, History, reminders, Settings; Odyle and Shugo keys counts; ⚠ dropdown of full resources); only the toolbar opens at launch ("Open the meter at launch" in Settings → Overlay, off by default; one-time prompt); the meter's "…" holds only the boss setting; fights are tracked only while the meter is open (closing it mid-fight ends and saves the fight like the idle timeout); tray icon (Show toolbar / Quit); Alt+Shift+CapsLock shows/hides everything; the daily reset now follows the game's 07:00 UTC across summer/winter time unless the player picked another hour. Independent review accepted after fixes. Open data still to confirm in game: Deus/Arkanis cap (built 4), Conquest tickets (built per dungeon), Balaur/Fafnium ids, Ordeal ticket.

2026-10-08: user tested and accepted: "Toolbar hub works great and I haven't seen issues with the resources. So you can move all of these to done if we find issues we'll fix them on the go. I right now see no issues left except the skill. You can start 1."
<!-- SECTION:NOTES:END -->

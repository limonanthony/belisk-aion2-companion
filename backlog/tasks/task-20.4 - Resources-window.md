---
id: TASK-20.4
title: Resources window
status: In Progress
assignee: []
created_date: '2026-10-08 12:33'
updated_date: '2026-10-08 16:53'
labels:
  - feature
dependencies: []
parent_task_id: TASK-20
priority: medium
type: feature
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A floating window that lists the resources the game fills in (Odyle Energy, event keys, entry tickets) for the character currently being played, so a player can see them at a glance without opening the to-do window.

Agreed (user, 2026-10-08): "move the count of resources outside of this window, some users might want a window only for these resources to be seen easily (e.g: in a floating window like the todos) we can still keep them in todo too". Then: "Yes I think a full resources window would be useful". On the full-pool warning: "That's good then !" (it stays as it is).

## Scope

- A full resources window listing every game-filled resource for the current character.
- The to-do window keeps showing them too; nothing is removed from it.
- Display rules are the ones in TASK-20 "Agreed display": regular pool against its cap, the extra pool beside it when non-zero, a total above the cap is normal, "full, being wasted" only when the regular pool reaches its cap.
- The window opens by clicking Odyle Energy or Shugo keys on the toolbar, or "All resources" in the toolbar's dropdown. The toolbar itself is the separate item "Toolbar hub: use the meter, to-dos, resources and reminders as separate tools".

Not in this item: the toolbar, rows the game does not send (they stay manual in the to-do window, TASK-20).

## Boundaries and dependencies

Needs the game values read in TASK-20.3. Mockups are drawn first (docs/gpui-mockups/hub/ and docs/gpui-mockups/todos-autofill/) and approved by the user before the window is built.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Clicking Odyle Energy or Shugo keys on the toolbar, or "All resources" in the toolbar's dropdown, opens the resources window.
- [ ] #2 The window shows the current character's game values for every game-filled resource, with the pool, extra pool and full-pool warning as in the TASK-20 display rules.
- [ ] #3 Spending or refilling a resource in game updates the window while playing, without reopening it.
- [ ] #4 Each value in the window equals the same resource's value in the to-do window.
- [ ] #5 The user approves the mockups before the window is built.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Built 2026-10-08, not committed, awaiting the user's test: the game's ticket values fill the to-do window and the new Resources window; a horizontal toolbar is the hub (meter, PvP, to-dos, History, reminders, Settings; Odyle and Shugo keys counts; ⚠ dropdown of full resources); only the toolbar opens at launch ("Open the meter at launch" in Settings → Overlay, off by default; one-time prompt); the meter's "…" holds only the boss setting; fights are tracked only while the meter is open (closing it mid-fight ends and saves the fight like the idle timeout); tray icon (Show toolbar / Quit); Alt+Shift+CapsLock shows/hides everything; the daily reset now follows the game's 07:00 UTC across summer/winter time unless the player picked another hour. Independent review accepted after fixes. Open data still to confirm in game: Deus/Arkanis cap (built 4), Conquest tickets (built per dungeon), Balaur/Fafnium ids, Ordeal ticket.
<!-- SECTION:NOTES:END -->

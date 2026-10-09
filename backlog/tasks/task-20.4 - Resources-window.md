---
id: TASK-20.4
title: Resources window
status: In Progress
assignee: []
created_date: '2026-10-08 12:33'
updated_date: '2026-10-09 09:13'
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

Update 2026-10-08 (after the user's in-game checks): rows that limit nothing on Global were removed.
- Conquest dungeons: user, "I think this is for korea, we can do them as much as we have odyle now".
- Deus Research Base / Shattered Arkanis: user, "For these it's the same as long as you have odyle".
- Abandoned Balaur Fortress / Fafnium Laboratory: user, "I only have 1 dungeon in the daily dungeon menu though".
- Daily Dungeon is now a resource row showing tickets left / 14 with the extra pool, +14 weekly (user: "just put daily dungeon with the tickets").
- A full row warns only near the refill it would lose (1 h for Odyle, 3 h for the daily refills, 24 h for weekly); otherwise it shows a calm "Full · refills …" (user: "Otherwise it's annoying to always have warnings even when not near the end").
Built and reviewed, not committed; the release exe is not yet rebuilt.

Update 2026-10-08 (user decision on the recharges in the to-do window): looking at the to-do window's Recharges section, the user said "I think we can remove the recharges from the todos now no ?". This replaces the earlier "we can still keep them in todo too". Odyle Energy, Shugo keys, Invasion keys, Nightmare and the Daily Dungeon now show only in the Resources window (and the toolbar's counts and warning). The to-do window keeps Duty Missions, Ascension Trial, Duty Commands and the player's own to-dos, and counts only those. Also since the last note: the Shugo Festival keys row is renamed "Shugo keys" and the "Account" tag shows as an icon (user: "yeah shugo keys is ok"); weekly rows the game doesn't limit on Global were removed (user: "No cap on transcendence or exploration or Conquest. Not sure for: Subjugation, Sanctuary, Ordeal", with no menu entry for the last three). Built and reviewed, not committed.

Note: the acceptance criterion #4 (each value in the window equals the same resource's value in the to-do window) no longer applies, since these resources are no longer shown in the to-do window. The criteria text is left unchanged.
<!-- SECTION:NOTES:END -->

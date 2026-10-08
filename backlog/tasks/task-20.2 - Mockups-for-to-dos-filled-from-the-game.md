---
id: TASK-20.2
title: Mockups for to-dos filled from the game
status: Done
assignee: []
created_date: '2026-10-07 20:31'
updated_date: '2026-10-08 16:53'
labels:
  - feature
dependencies:
  - TASK-20.1
parent_task_id: TASK-20
priority: medium
type: task
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Before anything is built, the user sees and approves how to-dos filled from the game will look.

## Scope

Mockups for every screen that changes:

- The to-do window.
- Settings → To-dos.
- How a value filled from the game looks next to a manual one.
- What the user sees before the first login has been seen (no game values yet).

Implementation starts only after the user approves the mockups.

Part of the parent work item. Waits on the ticket check, whose answers decide what the screens show.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Mockups cover every changed screen: the to-do window and Settings → To-dos.
- [x] #2 Mockups show a game-filled value versus a manual one, and the state before the first login is seen.
- [x] #3 The user approves the mockups before any implementation starts.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Input for the mockups: agreed display (user, 2026-10-07, on keys above stock because of premium; see the parent work item TASK-20, "Agreed display").

- Show the regular pool against its cap and the extra pool beside it when it is non-zero (for example "7 / 21" and "+3").
- A total above the cap is normal, never an error.
- "Full, being wasted" shows only when the regular pool reaches its cap.
- Typed values accept numbers above the cap (today they are clamped).
- The To-dos membership switch picks the membership cap for every ticket (for example Shugo keys 12 → 21, Odyle 560 → 840).

Update 2026-10-08: mockups made at docs/gpui-mockups/todos-autofill/ — index.html (the to-do window, 14 states), settings.html (Settings → To-dos, 4 states), README.md (lists every decision as Agreed / Proposed / Open). Awaiting the user's approval.

Questions put to the user:
- (a) With premium keys above 21, what does the game itself show: "24", "21 +3" or "24/21"?
- (b) Is a switch to stop filling from the game wanted? (recommended: no)
- (c) Arrows or none on game-filled rows, and "done / target" vs "N left" on weekly rows?

Open data, shown dashed in the mockups: Deus Research Base / Shattered Arkanis cap 4 or 14; which ticket a Conquest entry spends.

Update 2026-10-08: the user approved the mockups ("yup perfect"), after the open questions and drawn defaults were listed. Approved designs: docs/gpui-mockups/approved/ (todos.html, resources.html, toolbar.html, prompt.html, tray.html, meter.html, settings.html, pvp.html, reminders.html; tokens.md "Hub"); rationale in docs/gpui-mockups/todos-autofill/README.md and docs/gpui-mockups/hub/README.md.
<!-- SECTION:NOTES:END -->

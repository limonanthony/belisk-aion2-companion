---
id: TASK-35
title: Toolbar and soul tracker cards end up under the game after a while
status: In Progress
assignee: []
created_date: '2026-10-12 15:02'
updated_date: '2026-10-12 15:05'
labels:
  - bug
dependencies: []
priority: high
type: bug
ordinal: 99000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Found in game on 2026-10-12 on 1.1.0 (the running build had the same code as 1.1.0). After the program has run for a while:

- The toolbar is no longer on screen, although the program is running. The tray's "Show toolbar" brings it back.
- Soul tracker cards stop appearing when the player gains souls.

Expected: the toolbar stays visible for as long as the program runs, and a card appears on every soul gain (both are existing product rules).

## Cause found

- **Toolbar: covered, not hidden.** A live capture of the vanished toolbar (03:57, Belisk running): visible and topmost, 444x36 at (83,56), but below the game's window in the z-order. Above it were a foreground maximised normal window (class `StencilWindow`) and the game's `UnrealWindow` (now maximised, windowed). Windows may put a game's normal window above a topmost window that was only shown, never raised. The tray's "Show toolbar" ends with the same raise the fix uses, and the user reported it restores the toolbar, which supports this cause.
- **Cards: same cause.** The soul tracker card kept its window hidden between cards and reused it. The live window list showed the parked card hidden below the game's window, and revealing it never raised it.
- **The earlier "card takes 10+ s to appear" report most likely has the same cause** (a card alive under the game is only seen again when a new window is created after 10 s without a gain). This is the best fit found by the investigation, not proven: the logs of that evening no longer exist, and the core holding the gain, the game sending it late, and window creation time were each ruled out from the recordings and the code.

## Fix (in code, tested by agents only, not yet by the user)

- Every overlay is put back on top (raised without taking focus) whenever it is shown: first show of the toolbar and meter, a window opened again, a card or reminder that is updated while already visible.
- A once-a-second check re-asserts Belisk's fixed stack when another program's normal window covers an overlay: toolbar, then meter, then tools, then the reminder card, then the soul tracker card on top. It never takes the keyboard focus, and Settings is not part of the stack.
- The soul tracker card closes its window on dismissal like the reminder card (user decision, 2026-10-12); the hidden-and-reused window is gone.
- `belisk.log` now records the Alt+Shift+CapsLock shortcut, the tray's "Show toolbar", every re-raise (naming what covered the overlay), and GPUI's own warnings and errors with file and line.
- The "RefCell already borrowed" ERROR seen in `belisk.log` about 0.5 s after every launch came from showing a window inside a GPUI update (Windows draws the window synchronously during `ShowWindow`). Window shows and raises are now deferred until the update returns. The cause is proven from GPUI's code, not from a new log.

Not checked: whether the game's window really ends up under the stack after the once-a-second check on a real desktop. Only the user's play can show it.

## Related

TASK-34.5 (the soul tracker card; its note says the card's window was kept hidden between cards, which no longer holds), TASK-18 (a reminder card not seen during play; the same raise now applies to the reminder card, not proven to be the same cause), TASK-24 (toolbar). The code review of the same day and its fixes are tracked in the parent work item "Code review follow-ups (review of 2026-10-12)".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The toolbar stays visible over the game through a long play session, including after the game window becomes maximised or another program's window comes to the front; when covered it is back on top within about a second without taking the keyboard focus from the game.
- [ ] #2 A soul tracker card appears on every soul gain, including right after the previous card ended and after a long session; a gain while a card is up updates it and keeps it visible.
- [ ] #3 After launching Belisk, belisk.log contains no 'RefCell already borrowed' ERROR.
- [ ] #4 When an overlay is covered by another program's window, belisk.log names what covered it and that it was put back on top; it also records the Alt+Shift+CapsLock shortcut and the tray's Show toolbar when used.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: fix in code by agents (overlays raised whenever shown, once-a-second restack of a fixed stack, card closes its window, log lines, deferred Win32 calls). Not tested by the user. Related review work: TASK-36.
<!-- SECTION:NOTES:END -->

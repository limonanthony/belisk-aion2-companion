---
id: TASK-39
title: Windows reopen where you put them (meter and every tool)
status: Done
assignee: []
created_date: '2026-10-14 15:10'
updated_date: '2026-10-14 19:09'
labels:
  - feature
dependencies: []
priority: high
type: feature
ordinal: 103000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Today only the toolbar has a remembered position, and the user reports even that does not work (TASK-38). This adds a remembered spot for the meter and every tool window.

## Intent

User, 2026-10-14: "How hard would it be to remember the position where the user puts it's windows ? E.g: the dps-meter window, the toolbar, etc... ? And start them in that position ?" After Main offered A (meter only), B (meter and every tool each remember their own spot) and C (mixed): "Well the toolbar remember is not working on my end which is why I thought it didn't remember, B. would be the best though".

## Agreed (user)

- Option B: the meter, the toolbar and every tool window each reopen where the user last put it, across restarts and within a run.

## Derived (Main, from "the position where the user puts its windows")

- A position is saved when the user ends a drag of that window. A window the user never dragged opens as it does today (meter top-left of the primary monitor; tools beside the meter or under the toolbar; toolbar top centre).
- Tool windows covered: History, detail, fight report, PvP, to-dos, Resources, Pets, and Settings. One spot per window kind.
- A saved spot that is on no monitor any more (monitor unplugged, layout or resolution changed) is ignored and the window opens in its default place; a window that would stick out of its monitor's work area at the saved spot is pulled inside it.
- "Reset settings" clears the saved spots, as it already clears the toolbar's.
- Hide between fights, peeks, the Alt+Shift+CapsLock shortcut, the stacking order and the rule that no window takes focus stay as they are.

## Not in scope

The reminder and start-alert card, the soul tracker card, the first-open prompt and What's new keep their current placement.

## Open question (pending the user)

How the detail window combines with Settings > Overlay "Detail opens" (Beside / Centred): does a remembered spot win over that setting, or does the setting still decide? Until the user answers, the detail window's rule is not built; everything else is.

## Related

TASK-38 (the toolbar's position is not remembered for the user), TASK-24 (toolbar hub, Done).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 After the user drags the meter, the toolbar or a tool window (History, detail, fight report, PvP, to-dos, Resources, Pets, Settings) and reopens it, during the same run or after a restart, it opens where it was left.
- [x] #2 A window the user never dragged opens where it does today.
- [x] #3 A saved place that is no longer on a monitor is ignored, and a window that would stick out of the screen is pulled inside it.
- [x] #4 No window takes focus from the game when it opens.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-14: built and agent-tested: meter, toolbar, History, detail, fight report, PvP, to-dos, Resources, Pets and Settings save their top-left corner at the end of a user drag (Settings at the end of its OS move) as settings.json windows.<name>At; never-dragged windows open as before; a spot on no monitor is ignored; a window is pulled inside the work area at open. Decision resolved (user, 2026-10-14): Settings › Overlay "Detail opens" — "Beside" becomes "Where you left it" (beside the meter until first dragged, then where left; stored value "beside" unchanged), "Centred" stays always centred and its drags are not remembered. Known limits: Settings may show for one frame at the centre before moving to its spot; a meter saved near the bottom edge can stick out once its card grows (both under review). Independent review in progress.

2026-10-14 (user test): within a run the meter and toolbar reopened where left; after a restart both opened at their defaults although settings.json held the saved places (toolbarAt 53,40; meterAt 11,817). Cause: the settings loader split every stored object into fields and refused a field whose default was not an object; window places default to null, so every saved place was dropped when settings.json was read. This is also why the toolbar's place never survived a restart before. Fixed in code by agents (the loader keeps a stored place whole), regression test fails before and passes after, independent review accepted; awaiting the user's restart test.

2026-10-14: user confirmed the meter and the toolbar reopen where left after a restart ('yup it works now'); tool windows and the other criteria not yet confirmed by the user.

2026-10-14: user tested the delivered build in game on 2026-10-14 and, asked whether everything worked (start alert at a :00/:30, soul card after a mid-game start, tool windows reopening in place, clean launch log), answered: "Everything seem[s fine]". Evidence Main observed in the user's belisk.log and data folder: saved places restored after restarts and the tool places (history, pvp, todos, pets) saved in settings.json. Accepted by the user; criteria checked on that basis. Settings and the fight report were not named in the evidence.
<!-- SECTION:NOTES:END -->

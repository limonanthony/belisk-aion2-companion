# To-dos filled from the game: mockups (TASK-20.2)

**Approved on 2026-10-08** with the hub round. After Main listed the open
questions and the drawn defaults, the user answered "yup perfect": every
Proposed item below is agreed as drawn, with no switch to stop filling from
the game, no arrows on game rows and weekly rows as "done / target". The
approved design now lives in `../approved/`: the to-do window in
[todos.html](../approved/todos.html) (which replaces the 2026-10-06 drawing
there), Settings → To-dos in
[settings.html](../approved/settings.html?states=todos,todos:hint,todos:confirm),
the Resources window in [resources.html](../approved/resources.html), and the
"To-do window" (Values from the game) and "Settings → To-dos" values in
[tokens.md](../approved/tokens.md). Build from there; this folder is the
history of how it was chosen and is not updated any more. The open data
below stays open until the user's in-game comparison; the approved pages draw
it dashed, with the build's default.

Reviewed with [../hub/index.html](../hub/index.html), which links these
pages. The hub changed only where the to-do window opens from (the toolbar's
checklist, no longer the meter's "…") and added a Resources window built
from the same rows.

Open in a browser, no server needed:

- [index.html](index.html): the to-do window, 14 states over three game
  scenes, the last one a working window with buttons that play messages from
  the game (nothing is saved).
- [settings.html](settings.html): Settings → To-dos, 4 states.

Built on the approved design, which is not edited:
[../approved/todos.html](../approved/todos.html),
[../approved/settings.html](../approved/settings.html?states=todos) and
[../approved/tokens.md](../approved/tokens.md) ("To-do window", "Settings →
To-dos"). The pages load `../approved/shared.css`, `overlay.css`, `app.css`,
`kit.js` and `app.js` as they are, plus the bundled Barlow Semi Condensed and
class icons from `crates/app/assets/`.

| File | What it is |
|---|---|
| `index.html` | The to-do window page: states, behaviour notes |
| `autofill.js` | The window: a copy of the approved page's script, trimmed to what these states need, plus the game values, the open-data switches and the working window |
| `todos.css` | The approved page's `<style>` block, copied unchanged |
| `autofill.css` | Only what this feature adds: the source line, the figure without arrows, the extra pool, the pencil, two-line tooltips; page-only tags and dashed open-data marks |
| `settings.html` | Settings → To-dos with the changes; the approved section's styles copied in its `<style>` |

Selectors at the top of index.html: **Game rows** (figure only / keep the
arrows) and **Weekly rows** (done of target / entries left) switch between a
proposal and its alternative; **Open data** switches the two data questions
(the page reloads with `?deus=` and `?conquest=`). URL parameters also work:
`?states=live,typed`, `?scenes=dark`, `?arrows=keep`, `?weekly=left`.

Counts, times and characters are invented. Caps and refills come from the
game's ticket table as in our notes. Clock: Monday 21:48; Mirelle logged in at
19:02; membership on.

## States

To-do window: filled from the game, live · pointer on a game value · full,
being wasted · this week from the game · a game value next to a manual one ·
from yesterday's login · typing over a game value · after typing over it ·
just logged in · before the first login is seen · new character, server
already seen · open data, drawn as placeholders · membership off, the game
shows more · try it.

Settings → To-dos: to-dos (membership wording) · membership off, the game
shows more · reset to-dos completely (dialog) · option: a switch to stop
filling from the game.

## Decisions

### Agreed (2026-10-07)

1. A recharge shows its regular pool against the cap and the extra pool beside
   it when not zero: "7/21" and "+3".
2. A total above the cap is normal: no error, no colour (Invasion keys 2/7 +12).
3. "Full, being wasted" / "Full, spend by …" only when the regular pool
   reaches its cap; extras never trigger it.
4. Typed numbers may be above the cap; nothing cuts them.
5. The membership switch picks the membership cap of every ticket (Shugo
   Festival keys 12 → 21, Odyle Energy 560 → 840); its description says so.
6. Shugo Festival keys: cap 12 (21 with membership), +3 a day, extra pool up
   to 30.
7. Duty Missions and Duty Commands stay manual. Shugo keys, Invasion keys and
   the Unknown Fissure are shared by the characters of one server.

### Agreed (2026-10-08, hub round)

8. The full-pool warning stays as drawn ("Full, being wasted" / "Full, spend
   by …", the "1 full" chip): "That's good then !"
9. The to-do window opens from the toolbar (its checklist icon); the meter's
   "…" keeps only the boss setting. Settings → To-dos' lead says "The to-do
   window, opened from the toolbar." The counts stay in this window too, and
   also get a Resources window of their own ([../hub/](../hub/README.md)).

### Agreed (2026-10-08, "yup perfect": accepted as drawn)

1. **No arrows on a game value.** The figure stays where a manual figure
   sits; the extra pool takes the one-more arrow's place, in the same width,
   so nothing else in the row moves. Arrows stay on what you count yourself
   (Duty Missions, Duty Commands, your own, every row before the game has
   filled it). Not chosen: keep the arrows.
2. **Where the values come from**: one line under the character selector, in
   the meter's hint-line style: "From the game, live · logged in at 19:02";
   "From the game at login, Sun 22:40 · refills since estimated"; "No game
   values yet. Log in with Sefa while Kisk runs."; "Tavrin not seen at
   login yet · Account rows from Mirelle". The figure's tooltip repeats it
   for that row ("125 at your login Sun 22:40. +120 since: eight refills of
   15, estimated.").
3. **Between sessions** the window keeps the last game values and adds the
   refills the table knows since, up to the cap; spends are never guessed; a
   weekly row seen before Wednesday's reset starts over at the reset.
4. **Typing over a game value**: a click on the figure (Enter) opens the
   approved field with the regular pool in it; Enter keeps (above the cap
   too), Esc cancels. Your number takes the figure's place with a pencil in
   the gutter; the extra pool stays the game's. The game's next value for that
   ticket (spend, refill, item, login) replaces yours. In the field, × or an
   empty field goes back to the game's value (instead of "stop tracking").
5. **At login** the snapshot fills every row it covers and replaces what you
   typed there; a good note for 4 s: "Filled from the game at login. Values
   you typed were replaced."
6. **Weekly rows**: the game sends entries left; the row keeps the approved
   "done of target", done = target − left (9 left of 14 reads 5/14), done
   at 0 left, extra entries beside it (+7) without stopping the row from
   being done. A final-boss tick is done when its ticket reaches 0.
   Not chosen: "9 left".
7. **"Account"** on Shugo Festival keys, Invasion keys and the Unknown Fissure
   (today only Duty Missions); one character's login fills them for every
   character of that server.
8. **A typed total**: before the game fills a row, a typed number is what you
   have in all, so at or above the cap the row counts as full (as approved);
   once the game fills it, the split is exact.
9. **Refill hour of the keys**: Shugo keys +3 and Invasion keys +1 at the
   daily reset (the table's daily clock, 16:00 KST = 09:00 CEST), not 16:00.
10. **Membership off again** keeps every value (approved: an amount above
    560 was cut to 560).
11. **Membership hint**: with the switch off and a game regular pool above
    the cap (18 Shugo keys > 12, only possible with a membership), a quiet
    line under the membership description asks "Do you have a membership?".
    No automatic switch. While the switch is off, such a row is not drawn
    as full (its real cap is higher).
12. **Settings lead**: "Counts and amounts fill from the game when you log in
    while Kisk runs." (hub round: the game's values come in whether the meter
    is open or not; was "with the meter running".)
13. **Start over** empties what you typed; the game's values stay. The
    description says "every count and amount you typed", the dialog adds
    "Values from the game stay."

### Agreed (2026-10-08, was open)

- **No switch to stop filling from the game** (was drawn dashed in
  Settings): typing over a value covers a wrong one, and the next login
  fixes it.

### Open data (to confirm in game)

Built with the drawn default; settled by your in-game comparison:

1. **Deus Research Base / Shattered Arkanis cap**: 4 (ticket table) or 14
   (our list before). Built: 4; with 4, a 4/4 is full.
2. **Which ticket a Conquest entry spends**: its dungeon's own ticket (six
   rows, 14, +2 a day: the row becomes a folding group like Sanctuary) or a
   shared pool (one row; drawn with the Normal pool, 21, +1 every 8 h).
   Built: per-dungeon tickets. Needs a recorded run.
3. **What the game shows for keys above 21** (premium items): one number
   ("24"), the two pools ("21 +3") or "24/21"? It decides whether item 8
   above (a typed number is your total) is right. Built: item 8.
4. Data that changes nothing on screen, for the same comparison: which of
   tickets 102/103 is Balaur Fortress or Fafnium Laboratory, the exact reset
   hour, and Ordeal's ticket (matched by name only).

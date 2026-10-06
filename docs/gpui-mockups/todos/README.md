# Daily and weekly to-dos: window proposals

**Approved on 2026-10-06: direction E with its Settings section** (user: "All
good on my end", with every recommendation below). The approved design now
lives in `../approved/`: [the to-do window](../approved/todos.html), the
To-dos section of [Settings](../approved/settings.html?states=todos) and the
"To-do window" and "Settings → To-dos" values in
[tokens.md](../approved/tokens.md). Build from there; this folder is the
history of how it was chosen and is not updated any more.

Open [index.html](index.html) in a browser (no server needed);
[e.html](e.html) is the approved proposal as it was drawn, with its Settings
section in [settings-todos.html](settings-todos.html); [d.html](d.html) is
E's base. E and D show every state over three game scenes, the overlay's
opacity and scale controls, and a working window as their last state ("Try
it": nothing is saved). The pages use `../approved/shared.css`,
`../approved/overlay.css`, `../approved/kit.js` and the bundled Barlow Semi
Condensed and class icons from `crates/app/assets/`; the Settings page uses
`../approved/app.css` and `../approved/app.js` with a copy of the approved
sidebar. E's code is [e.js](e.js) and D's [d.js](d.js); their styles are the
"Direction E" and "Direction D" sections of [todos.css](todos.css) (E builds
on D's).

## E, struck through when done (approved 2026-10-06)

The user asked for a variant of D "with overlined on the done item and the
item muted instead of checkboxes", leaving the done control open. E is D's
window and model with no checkboxes and no edit mode. On 2026-10-06 the
user chose the check button as the done control ("I like the alternative
with just the v to check") and kept your own to-dos, with a way in Settings
to bring the defaults back.

- **Done**: the name (and its small grey part) is struck through and muted
  where it stands; rows never move. Names take the box's place and start
  under the section labels.
- **Tick rows**: a click anywhere on the row marks it done or not done.
  Under the pointer (or with keyboard focus) a 20 px check button appears
  at the row's end, an undo arrow on a done row; its tooltip and name are
  "Mark done" and "Undo". Space or Enter on a focused row does the same; a
  screen reader hears "Legion check-in, not done", then "Legion check-in:
  done". The check and the × are not tab stops.
- **Counts** keep D's arrows and the click on the figure; at the limit they
  get the same struck-through look ("5 of 5, done"). No check on counts.
- **Amounts** are unchanged from D (empty "—", refill estimate once typed,
  yellow "Full, being wasted").
- **Hiding**: a small × at the end of the row, only while the pointer is on
  that row (appears on enter, goes on leave; its slot is kept so nothing
  shifts). Delete on a focused row does the same. Each section ends with a
  faint "2 hidden · Show"; Show lists them in place, faded, each with a
  put-back button. Your own to-dos are removed instead, with Undo for four
  seconds.
- **Adding**: "+ Add a to-do" ends the list and opens a small form in place
  (name; Tick or Count of N; Daily or Weekly). Enter adds, Esc cancels.
- **Opening**: "To-dos" in the meter's "…" menu, between PvP meter and
  Settings, with "1 full" while an amount you typed is full.
- The reset hour ("Change" in the footer) and the membership live in
  Settings, To-dos.

States: fresh week, under way, pointer on a to-do (the check, "Mark done"),
pointer on a done to-do ("Undo"), a count reaching its limit, hidden rows,
after Show, adding a to-do, removing your own, everything done, keyboard
focus on a row, full and being wasted, opening it from the meter's "…"
menu, one alternative (a line over the name instead of through it), and the
working window.

## Settings, To-dos

[settings-todos.html](settings-todos.html): a new section after Reminders,
drawn on the approved Settings page (rows, Switch, Select, buttons, badge,
Dialog).

- **Daily reset**: a select of whole hours, 09:00 by default, "Reported, not
  confirmed. Check the countdown in game." **Weekly reset**: shown, not set:
  Wednesday at the same hour.
- **Membership**: one switch, Odyle Energy up to 840 instead of 560.
- **Restore default to-dos**: every hidden built-in to-do comes back; your
  own to-dos and all progress stay. No dialog; a "6 to-dos back" badge for
  4 s; disabled while nothing is hidden.
- **Reset to-dos completely**: removes your own to-dos, brings everything
  back and empties every count and amount, after a confirm dialog (Cancel
  focused). The reset hour and membership stay.
- Both apply to all characters (decided 2026-10-06).

States: defaults, the hour list open, restored, the confirm dialog, after
the complete reset.

### Choices in E and its Settings, all decided 2026-10-06

The user accepted every recommendation ("All good on my end"); reasons are on
[index.html](index.html). The line over the name (drawn once in e.html) was
not chosen. Also decided then: counts are done at N, with no check; a click on
a count's figure adds one and Odyle Energy steps 10; "Daily Dungeon · Unknown
Fissure" sits under This week; Duty Commands start hidden; Growth dungeons
are left out; the "+1?" suggestions from boss kills come later, not in this
build; the complete reset keeps the reset hour and the membership.

1. A line through the name (recommended), or over it.
2. No check on counts (recommended).
3. × removes your own to-dos, with Undo (recommended).
4. "Show" lists hidden rows faded with put-back buttons (recommended).
5. The check and the × are not tab stops; Space, Enter and Delete on the row
   (recommended).
6. After a hide, the next row's × waits for the pointer to move
   (recommended).
7. Restore default to-dos without a dialog (recommended).
8. The reset hour by whole hours (recommended).
9. Membership: one switch for every character; off again cuts amounts above
   560 (recommended).
10. To-dos after Reminders in the Settings sidebar (recommended).
11. Restore and Reset apply to all characters (recommended).

## D, you set the counts (E's base)

The packet research found that the meter cannot read any of these counts from
the game, so the model the user chose is manual: "make the counts manual but
empty by default". Drawn on B's window with A's one-line rows (370 px, one
list per character, the selector following the character in game; folding
Today, Recharges and This week, each with its own countdown).

- **Tick**: done or not done (Abyssal Forge's final boss, a to-do of your
  own). A click on the row.
- **Count**: "9/14" with one-less and one-more arrows; a click on the figure
  adds one. Stops at 0 and at the limit; back to 0 at the daily reset (Today)
  or on Wednesday (This week).
- **Amount** (recharges that bank up to a cap): empty by default, a quiet
  dash that is ignored everywhere. Type what you have (Enter keeps, Esc
  cancels); the meter then adds the refill on schedule and says "Full in
  1 d 2 h", or turns the row yellow at the cap ("Full, being wasted";
  "Full, spend by Tue 16:00"). Correct it after spending with the arrows or by
  retyping. Empty the field, or click its ×, to go back to the dash.
- Sanctuary and Exploration fold into one row each ("2 of 6 done").
- "Daily Dungeon · Unknown Fissure" sits under This week: the game calls it
  a daily dungeon, but its 14 entries reset on Wednesday.
- The reset hour is a setting, "Daily reset 09:00 · Change" in the footer
  (09:00 reported by a fan site, 17:00 also seen); never printed as fact.
- Duty Missions are marked Account (shared by your characters on a server).
- Edit mode: show or hide each row, add your own (Tick or Count of N, Daily
  or Weekly), and a Membership box that raises Odyle Energy to 840.
- Option, off by default: when the meter sees a known dungeon's last boss
  fall, that count offers "+1?".

States: fresh week, under way, an amount entered, typing an amount, full and
being wasted, everything done, edit mode, pointer over an arrow, pressing an
arrow, pointer over a figure, keyboard focus on a figure, the "+1?"
suggestion, the character selector, and the working window.

### Choices in D, decided with E

Each was settled as recommended (edit mode itself was replaced by E's hover
× and "+ Add a to-do", and the membership moved to Settings). Reasons are on
[index.html](index.html).

1. A click on a count's figure adds one (recommended: keep).
2. "Daily Dungeon · Unknown Fissure" as the row's name (recommended).
3. Default reset hour 09:00, changeable (recommended until checked in game).
4. One "Conquest dungeons" amount row; separate rows for the other recharges
   (recommended).
5. Sanctuary and Exploration as folding groups (recommended).
6. Odyle Energy's arrows step 10 (recommended; everything else 1).
7. A Membership box for Odyle Energy only (recommended; Shugo's membership
   cap is conflicted).
8. "+1?" from boss kills: off by default, built later (recommended).
9. Duty Commands hidden by default; Growth dungeons left out (recommended).
10. Typing an amount takes the keyboard until Enter, Esc or a click outside,
    like the PvP name field (recommended).
11. Opening: the "To-dos" item in the meter's "…" menu (drawn in A).

### Default list in D

- Today: Duty Missions (5, per account and server).
- Recharges (amounts, empty by default): Odyle Energy (+15 every 3 h, up to
  560; 840 with a membership), Shugo Festival keys (+4 at 16:00, up to 28),
  Invasion keys (+1 at 16:00, up to 7), Nightmare (+2 a day, up to 14),
  Conquest dungeons (+2 a day each, up to 14), Deus Research Base and
  Shattered Arkanis (+2 a day, up to 14), Abandoned Balaur Fortress and
  Fafnium Laboratory (+1 a day, up to 7).
- This week: Daily Dungeon · Unknown Fissure (14), Ascension Trial (3),
  Subjugation (3), Sanctuary (Abyssal Forge 4 attempts and its final boss;
  Corroded Facility and Chalice of Muspel 4 attempts and 2 final bosses
  each), Transcendence final boss kills (28), Conquest final boss kills (35),
  Cradle of Nihility (7), Dying Dramata's Nest (7), Ordeal final boss kills
  (7), Exploration (7 runs in each of ten dungeons), Duty Commands (20,
  hidden).

## Earlier explorations: A, B and C

Drawn before the packet research; kept for reference. Their Deus Research
Base and Shattered Arkanis cap (4) was wrong: it is 14.

| | A, tabs ([a.html](a.html)) | B, one list per character ([b.html](b.html)) | C, all characters ([c.html](c.html)) |
|---|---|---|---|
| Width | 340 px | 370 px | 520 px |
| Layout | Daily \| Weekly tabs, one line per to-do | One list: Today, Recharges, This week; sections fold; two lines per to-do | Tabs; a grid, one column per character |
| Countdowns | One line under the tabs | On each section label | Beside the tabs |
| Recharges | "Used up" click, then an estimate ("Full in 2 d 16 h") | Same as A | Counters the meter refills, corrected in a popover |
| Ticks | Manual | Manual, plus suggested ticks from boss kills, confirmed by a click | Manual |
| Characters | One list | Selector; follows the character in game | All at once (three most recent) |

## Not settled by the research

- The reset hour: 09:00 Berlin is reported by a fan site, 17:00 also seen.
- Per character or per account for everything except Duty Missions
  (inference: per character).
- Conquest and Transcendence each have two refill rules in MetaBot's tables
  (+2 a day per dungeon up to 14, and +1 every 8 h up to 21; +2 a day up to
  14, and +1 every 12 h up to 14). D uses the daily rule.
- When Odyle Energy's 3-hour refill ticks (D's estimate assumes 00:00,
  03:00, …).
- Unlocks for Ordeal, Abandoned Balaur Fortress and Fafnium Laboratory.

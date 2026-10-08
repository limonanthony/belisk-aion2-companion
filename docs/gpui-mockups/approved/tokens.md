# AION 2 DPS Meter design tokens

The approved visual language for the AION 2 DPS Meter's GPUI overlays and main
window. Sources of truth: [meter.html](meter.html) (round 1, D2 with bar end d
and rank b; the round 2 combat-power fix; round 4, layout D with the Session
and Target boards), [pvp.html](pvp.html),
[detail.html](detail.html) and [history.html](history.html) (round 2), and
[gate.html](gate.html), [home.html](home.html) and
[settings.html](settings.html) (round 3), [reminders.html](reminders.html)
and the Reminders section of settings.html (2026-10-05), and
[todos.html](todos.html) and the To-dos section of settings.html
(2026-10-06), and [report.html](report.html) with the summary card's
report button in meter.html and the boss records in history.html
(2026-10-07). The shared overlay classes live in [overlay.css](overlay.css),
the fight report's in [report.css](report.css), the main-window components
in [app.css](app.css). Later rounds extend this file only with values the
user has approved.

Colours are sRGB. `a` is the user's background opacity (0–255 in Settings,
default 150, so `a = 150/255 ≈ 0.59`).

## Type

| Role | Family | Size / weight |
|---|---|---|
| All overlay text | Barlow Semi Condensed, embedded in the exe (requested in GPUI as `Barlow`) | — |
| Hero figure (summary DPS) | | 36 px / 600 |
| Capsule DPS | | 17 px / 600 |
| Detail key figures | | 16 px / 600 |
| Boss name in summary, player in detail header | | 14.5 px / 600 |
| Row DPS, fight time | | 14 px / 600 (13.5 in PvP, detail and history rows) |
| Window title (PvP, history) | | 13.5 px / 600 |
| Body, player names, meter header name | | 13 px / 500 (you: 600; header name 600) |
| Values in footer and key-value lists, menu items, inputs | | 12.5 px / 500–600 |
| Percent column | | 12 px / 400 |
| Gauge %, section labels, tabs, buttons | | 12 px / 600 (Session \| Target switch: 11 px / 600) |
| Secondary (CP, server, labels, system figures) | | 11–11.5 px / 500 |

- Figures are tabular (`tnum`).
- Labels are sentence case. No uppercase transforms, no letter spacing.
- Long names cut off with an ellipsis; numbers never wrap.
- The mockups were drawn with Bahnschrift SemiCondensed. Stock GPUI cannot
  select that width (docs/GPUI_MIGRATION.md, phase 2a), so the overlays ship
  Barlow Semi Condensed (SIL OFL 1.1, `crates/app/assets/fonts/`): a
  semi-condensed DIN-like face with static 400, 500, 600 and 700 weights.
  It has no 650, so "you" is 600.

## Colour

| Token | Value | Use |
|---|---|---|
| `card` | `rgba(16, 14, 12, a)` | Overlay card and windows |
| `card-border` | `rgba(255, 236, 214, 0.10)`, 1 px | Card outline |
| `card-shadow` | none | Overlay cards have no drop shadow. An overlay window must end at the card (outside it, a transparent window still takes clicks while unlocked), so a shadow is clipped at the window edge; over a bright scene in game that showed as a faint hard-edged dark rectangle (2026-10-04). Main-window dialogs, menus and popovers keep their shadows |
| `track` | `rgba(0, 0, 0, 0.34)` | Bar track, record plate, dark plate under text |
| `soft` | `rgba(255, 240, 225, 0.08)` | Tonal button fill, notes |
| `hot` | `rgba(255, 240, 225, 0.13)` | Active tool button, selected tab, highlighted menu item |
| `text` | `rgba(255, 246, 236, 0.96)` | Primary text |
| `secondary` | `rgba(255, 246, 236, 0.82)` | Combat power and server beside meter names |
| `muted` | `rgba(255, 246, 236, 0.66)` | Secondary text, % column, section labels, the mob's name in the meter header on the Session board |
| `faint` | `rgba(255, 246, 236, 0.44)` | Labels, counts, placeholders, table headers, the "/s" unit on Session DPS |
| `you-badge` | `rgba(255, 246, 236, 0.90)` bg, `#17120d` text | Your rank badge |
| `you-rim` | `rgba(255, 246, 236, 0.45)`, 1 px inside the track | Your bar |
| `good` | `#86e3a8` | Ahead of PB, defeated, good ping, live, success notes |
| `warn` | `#f2c66d` | Fight ended (provisional), history mode |
| `bad` | `#f7a8a8` | Behind PB, delete confirmation, deaths in the fight report (at 0.10 for a player's deaths plate) |
| `hp` | `#e8735f` | Boss gauge fill; health bars and the fight report's damage-taken bars as a 0.16 → 0.55 gradient; the crown on boss records in History |
| `gauge-empty` | `rgba(255, 240, 225, 0.12)` | Empty gauge segment |
| `pb-new` | `rgba(250, 204, 21, 0.17)` bg, `#fde68a` text | New personal best chip |
| `chip-good` / `chip-warn` / `chip-bad` | `good`, `warn`, `bad` at 0.13–0.14 bg | Status chips and notes |
| `danger-fill` | `rgba(247, 168, 168, 0.14)` bg, `bad` text | Confirming button ("Delete?") |
| `pending-rim` | `rgba(247, 168, 168, 0.55)`, 1 px | The record a delete confirmation refers to |
| `neutral-fill` | cream `rgba(255, 240, 225, 0.04 → 0.16)` | Bars where no class is known (PvP kill stats, meter rows before the class is seen) |
| `tag-attacker` | `rgba(232, 115, 95, 0.18)` bg, `#f6b4a6` text | "Last attacker" tag |
| `popover` | `rgba(30, 26, 22, 0.98)`, border `rgba(255, 236, 214, 0.14)`, shadow `0 10 24 rgba(0, 0, 0, 0.5)` | Menus, dropdowns, suggestions, the meter's tooltip over the Session \| Target switch (drawn inside the card, so its shadow is not clipped) |
| `input` | `rgba(0, 0, 0, 0.34)` bg, border `rgba(255, 240, 225, 0.12)`; focus border `rgba(255, 246, 236, 0.55)` | Text fields and selects |
| `scroll-thumb` | `rgba(255, 240, 225, 0.28)`, 3 px wide | Scrolling lists |
| `full-fill` | `rgba(242, 198, 109, 0.12)` (`warn` at 0.12) | A to-do amount at its cap, being wasted: the whole row, also under the pointer |
| `press` | `rgba(255, 240, 225, 0.20)` | A to-do count's arrow or figure while pressed |

Class colours are the only strong hues. Bars use them, and so do the capsule
badge, skill rows (the player's class) and buff coverage (the caster's
class):

| Class | RGB |
|---|---|
| Assassin | 168, 85, 247 |
| Chanter | 52, 211, 153 |
| Cleric | 251, 191, 36 |
| Elementalist | 34, 211, 238 |
| Fighter | 248, 113, 113 |
| Gladiator | 251, 146, 60 |
| Ranger | 163, 230, 53 |
| Sorcerer | 232, 121, 249 |
| Templar | 56, 189, 248 |

**Class shades.** When several players share a class, each gets a shade of
it so friends can be told apart without reading names; no new hues. The
first player of a class seen since the last reset keeps the colour above; the
next ones step the HSL lightness by 0.10 per step, in this order, then the
order starts over:

| Class | Steps after the class colour |
|---|---|
| Assassin | +1, −1, +2, −2 |
| Cleric, Ranger | −1, −2, −3 |
| Chanter, Fighter | +1, −1, −3, −4 (no −2: neither badge ink reaches 4.5:1 there) |
| All others | +1, −1, −2, −3 |

A lighter step is used only where the white figures on the bar's bright end
keep 4.5:1 over a dark scene, so the already bright cleric and ranger only
darken. A player keeps their shade, by name (by id until named), on both
boards until the meter is reset: it follows the player, not the rank. The
shade colours the bar gradient and your capsule badge; summary skill bars
keep your plain class colour. Your row keeps its cream rim. The capsule
badge's number is `#17120d` ink, or cream `rgb(255, 246, 236)` on the darker
shades where the ink would fall below 4.5:1.

## Shape

| Element | Value |
|---|---|
| Card radius | 14 px |
| Capsule | fully rounded pill, 30 px tall |
| Bar track and fill | 5 px radius; track clips the fill; 22 px tall (24 px skill rows, 18 px summary skill rows) |
| Bar fill | two-stop horizontal gradient, class colour at 0.14 → 0.50 (brightest at the leading edge); width = value ÷ top value, floor 6% |
| Thin coverage bar (buffs) | 4 px tall, 2 px radius, same gradient |
| Rank badge | 18 × 18 px, 4 px radius, `rgba(255, 240, 225, 0.09)`, text `muted` 11.5 px / 600; you: `you-badge` |
| Capsule rank badge | 22 × 22 px, 5 px radius, your class colour, `#17120d` text 13 px / 700 |
| Gauge | 10 segments, 5 px tall, 3 px radius, 3 px gaps, in a fixed 22 px block |
| Tool button | 22 × 22 px, 7 px radius, icon 13 px, stroke 2 (20 px inside rows, icon 12 px) |
| Text button | 20–22 px tall pill, `soft` fill, icon 12 px; quiet variant has no fill |
| Status chip | 20–22 px tall pill |
| Tabs (segmented) | track `rgba(0, 0, 0, 0.30)`, 8 px radius, 2 px inset; tab 22 px tall, 6 px radius, selected `hot` |
| Session \| Target switch (meter footer) | 100 × 18 px, fixed width; track `rgba(0, 0, 0, 0.30)`, 6 px radius, 2 px inset; two equal options, 4 px radius, active `hot` with `text`, inactive `muted` (pointer over: `soft` fill, `text`) |
| Pin dot | 4 × 4 px, fully rounded, `currentColor`, 3 px before the active option's label: the board is pinned by hand |
| Meter tooltip | ≤ 230 px wide, 8 px radius, 6 × 9 px padding, `popover` colours; title 12 px / 600, line 11.5 px / 500 `muted`; 8 px from the card's right edge, 30 px from its bottom |
| Input / select | 26 px tall, 7 px radius; label above at 11 px `faint` |
| Popover | 9 px radius, 4 px padding; items 26 px tall, 6 px radius |
| Plates (key figures, record) | 7–8 px radius, `track` fill; opened record `rgba(0, 0, 0, 0.40)` with a 1 px `rgba(255, 246, 236, 0.18)` rim |
| Expanded row (skill figures) | 6 px radius, `rgba(255, 240, 225, 0.05)`, four columns of label over value |
| Collapsible panel (buffs) | 8 px radius, `rgba(0, 0, 0, 0.20)`; header 28 px with a chevron |
| Class icon | 16 px, 8 px radius clip (20 px in the detail header) |
| Skill / buff icon | 18 px with 4 px radius in rows; 24 px with 5 px radius in buff rows |

No bands, no single-edge accent stripes, no inner highlights. Borders: the
card outline, the 1 px "you" rim, the opened-record rim, inputs and popovers.

## Spacing

| Element | Value |
|---|---|
| Card padding | 8 px sides and top, 6 px bottom (windows: 6 px top, 8 px bottom) |
| Widths | meter 310 px (340 with total damage); PvP 340; history 370; detail 520; fight report 520 |
| Row gap | 3 px |
| Badge to bar | 3 px margin + 5 px row gap = 8 px, matching the card's 8 px inset |
| Inside a bar | 4 px left, 9 px right, 6 px between icon, name and figures |
| Percent column | 38 px, right-aligned |
| Window header | 28 px; meter header 22 px |
| Section label row | 24 px, 6 px above |
| Footer row | 20 px (meter), 26 px (windows) |
| Meter footer | always shown, 5 px above; 4 px gaps; left: figures; right: ping (auto margin), then the switch 6 px after; no right inset, so the switch ends on the bars' right edge |
| Meter hint line ("not identified yet") | 20 px, 4 px above the footer, 4 px inset, Lucide `map` 12 px in `faint`, 11.5 px `muted` text |

## Interaction patterns

- **Meter header**: the target's name on the left at full width, cut off
  with an ellipsis only when it runs out of room; `text` on the Target board,
  `muted` on the Session board (the mob being hit right now). At rest the
  time sits on the right (the session's combat time on Session).
- **Meter under the pointer**: the tools (reset, capsule, lock, copy
  report, "…", close; 22 px tool buttons, evenly spaced 2 px apart) take the
  time's place at top right; the name gives way. Copy turns into a check for
  1.5 s. "…" (tooltip "Boss setting") opens the boss setting for the
  header's target, and nothing else (see "Hub", Meter "…" menu and ×). A
  click elsewhere in the meter, a second click on "…" or the pointer leaving
  the card closes the menu. × closes the meter only. Locked (click-through)
  shows no tools. The window never resizes on hover.
- **Meter footer**: always shown. Session board: Damage (session total) and
  the number of targets fought (Lucide `target` 12 px in `faint`, then the
  number in `muted`). Target board: the pace against your personal best for
  this boss once the fight is 10 s old (`good` ahead, `bad` behind, trend
  icon, "−6.8% vs PB"); without a personal best, or before 10 s, the party's
  Damage on this target. Then ping and the Session | Target switch.
- **Board choice**: Target while a boss is engaged, Session otherwise. A boss
  is engaged only once you have engaged it yourself: you hit it, or it hit
  you (before the game has named you, any boss target counts, and it is
  released again if it turns out not to be yours). The Target board then
  stays on that boss, whatever else is hit, until it dies or it goes two
  minutes without a hit. Another boss takes over only after you have hit only
  it for 20 s (a hit on the first restarts the count; before the game has
  named you, it must stay your target for 20 s), and a reset releases it too.
  (A zone change would release it too, but the game does not re-send the
  player's identity on a zone, so in practice the two-minute rule lets go
  after you leave.) Clicking the
  inactive side pins it (pin dot); clicking the active side
  again returns to automatic; a reset or the next boss also clears the pin. A
  pinned Target board follows your current target. Locked, the switch shows
  but does not react. Tooltips: "Show the target board / Follows your target
  until reset or the next boss." and "Pinned to Session / Click again to
  switch by itself. Reset and the next boss also unpin."
- **Session board columns**: rank, class icon, name (+ combat power), DPS
  quiet in `muted` with a `faint` "/s", damage bold at 14 px / 600, then the
  share of session damage in the 38 px percent column. Sorted by damage. DPS
  counts each player's combat time. No gauge line.
- **Not identified yet**: the game names your character at login, so until
  it has, rows show `ID:<n>`, nobody is marked as you, the capsule shows the
  party's figure without a rank badge, and the hint line says "Start before
  login, or relog via character select" (the long form: start the meter
  before logging in, or relog via character select). It goes away once you
  are named.
- **Product name**: "AION 2 DPS Meter" in the main window; "DPS Meter" where
  an overlay has no target to name (waiting card, idle capsule). "Kisk"
  where the hub names the app (the toolbar's quit tooltip, the first-open
  prompt, the tray, the new Settings descriptions); the other names change
  with the rename (TASK-23).
- **Sorting**: the active column is `text` with a down arrow; other sortable
  columns are underlined at 0.25.
- **Delete confirmation**: the button turns into a `danger-fill` "Delete?"
  (or "Delete all 14 records?") with a quiet Cancel, and reverts after a few
  seconds. The record concerned gets the `pending-rim`.
- **Expandable rows**: a chevron at the row's end; one row open at a time.
- **Status feedback**: a `chip-good` note in the window ("Deleted 14
  records. Personal bests are kept."), shown for four seconds.
- **Empty states**: a 14 px / 600 title and one muted sentence saying what
  will fill the space.

## Motion

- Bars grow and shrink over 240 ms (ease-out); rows trade places over 240 ms.
- Figures swap in place, no counting.
- Hover swaps (meter header: time and tools) cross-fade in 100 ms.
- Board change (automatic, pin, unpin): the board cross-fades in 150 ms and
  the switch's highlight moves across in 120 ms. The card's height follows the
  board (the Target board adds the 22 px gauge line), never the pointer.
- Chips and notes fade in over 150 ms. Nothing flashes or pulses.
- The capsule grows into the card over 180 ms on pointer over and returns
  after a 350 ms grace period.
- The reminder card fades in over 150 ms and fades out over 150 ms, on
  time-out and on dismissal alike; no slide, no scale. Its time-left line
  needs about one frame per pixel (10 per second at 100%), only while the
  card is up.
- The to-do window's check and × appear and go at once, with no fade, on the
  row's pointer enter and leave only; nothing in that window animates.
- The fight report's row lift and chevron appear on the row's pointer enter
  and go on leave, never on movement inside the row.
- The toolbar's tools and counts light on pointer enter and go on leave,
  never on movement inside; its tooltips appear after 500 ms, as the
  meter's. Its dropdowns open and close at once. The first-open prompt's
  good note stays 4 s, then the card fades out over 150 ms, as the reminder
  card.
- Frames are drawn only on a snapshot or while a tween runs.

## Reminder card

A small overlay window of its own before timed events
([reminders.html](reminders.html)). It uses the overlay values above: `card`
at the user's opacity, `card-border`, no shadow, Barlow Semi Condensed, the
meter's content scale.

| Element | Value |
|---|---|
| Placement | 6 px above the meter's top edge, left edges aligned. If it does not fit above the meter on that monitor: 6 px below the meter's bottom edge, following the meter's height. Capsule layout: 6 px above the capsule, on its left edge. While the meter is closed (hub, 2026-10-08): 6 px under the toolbar, left edges aligned, 310 px; above the toolbar when it sits low on the screen. A first-open prompt on screen: the card stacks 6 px above it |
| Width | The meter's: 310 px, 340 with total damage; capsule layout a fixed 220 px. All times the content scale |
| Height | Hugs the content, border included, at 100%: 1 event 84 px, 2 events 123, 3 events 162 |
| Card | 14 px radius; 8 px sides and top, 6 px bottom |
| Header | 22 px, 4 px left inset, 6 px gaps: Lucide `bell` 13 px in `muted`; "In 1 min" 12.5 px / 600 `text`; a `faint` "·"; the start time 12.5 px / 500 `muted`. Unlocked, the × is the meter's 22 px tool button (7 px radius, `muted`; pointer over the card: `hot` fill, `text`), flush right |
| Event block | 23 px left inset, so names line up with the header text; 4 px right. 4 px under the header, 7 px between events. Name 13 px / 600 `text`, 17 px line; one line of fact 11.5 px / 500 `muted`, 15 px line; both cut off with an ellipsis |
| Time-left line | 2 px tall, 1 px radius, 4 px inset on both sides, 8 px under the last event; track `gauge-empty`, fill `faint`; drains right to left, linear, over 30 s. Inset and rounded like the gauge, not an edge stripe |

- **When**: the lead time set in Settings (1 min by default) before each
  event switched on. "In 1 min" is rounded up to the minute and does not tick
  while the card is up; the clock time beside it never changes. The test
  reminder's header reads "Test reminder", without a time.
- **Same minute**: every event due at that minute shares one card and one
  sound, the rarer first: weekly, daily, every 3 hours, hourly. Up to three
  events meet (11:00, 16:00, 19:00 with everything on).
- **Lifetime**: 30 s, then the fade out. The pointer over the card pauses the
  30 s (the line stops) and lights the ×. A newer reminder replaces one still
  on screen.
- **Unlocked**: a click anywhere on the card, or on ×, dismisses it at once.
- **Locked**: click-through like the meter: no ×, nothing reacts to the
  pointer, and it runs out by itself.
- **Focus**: the window never activates (the meter's no-activate, tool-window
  styles); the game keeps keyboard and mouse focus. No Windows notification.
- Not built yet, decided with that feature: whether Alt+E also dismisses
  it.

## To-do window

An overlay window of its own for what resets and is lost if not done
([todos.html](todos.html)), approved on 2026-10-06 (direction E); filled
from the game and opened from the toolbar, approved on 2026-10-08 (see
"Values from the game" below). It uses the overlay values above: `card` at
the user's opacity, `card-border`, no shadow, Barlow Semi Condensed, the
meter's content scale, the window header and footer.

| Element | Value |
|---|---|
| Window | 370 px wide; card padding 6 px top, 8 px sides and bottom. Height hugs the content; the list scrolls past 452 px, so the window is at most 558 px (590 while a 4 s note shows). All times the content scale |
| Header | The 28 px window header: Lucide `list-checks` 13 px in `muted`, "To-dos" 13.5 px / 600, then "9 left" (or "all done") 11.5 px / 500 `faint`; while an amount is full, a `chip-warn` "1 full" 18 px tall, 11 px text, Lucide `triangle-alert` 11 px; the close tool button (22 px) flush right |
| Character selector | 2 px above, 6 px below; the 26 px input at 28 px; 16 px class icon; name 13 px / 600; "Ranger · Ossia" 11.5 px `muted`; "Playing" 11 px / 600 `good` after a 6 px dot, pushed right; Lucide `chevron-down` 13 px `muted`. Open: a popover over the full width, 30 px items, the count left at 11 px `faint` (`warn` / 600 for "1 full"), a 1 px divider at 0.18 and an 11 px `muted` note |
| Section label | 24 px, 6 px above (none above the first), 4 px left and 6 px right inset, 6 px radius, 6 px gaps; Lucide `chevron-down` / `chevron-right` 12 px `faint`; title 12 px / 600 `muted` (pointer over: `text`); count 11 px / 500 `faint` ("done" in `good`); the countdown pushed right, 11 px / 500 `faint` with the time in `muted` / 600 |
| Row | At least 26 px, 2 px apart, 6 px radius; 4 px left and 3 px right inset; a 12 px gutter (a group's chevron, a full amount's `triangle-alert` 12 px in `warn`, else empty), 6 px gaps, so names start 22 px in, under the section label's text. A group's rows: 22 px left inset (names 40 px in) |
| Name | 13 px / 500 `text`, cut off with an ellipsis; its second part ("final boss", "Unknown Fissure") 11 px / 500 `faint`, 5 px after; "Account" 11 px / 500 `faint` with Lucide `users` 11 px, 6 px after; "Yours" as the second part |
| Pointer on a row | Tick and group rows (a click acts on the whole row): `soft` fill. Count and amount rows: no fill; their arrows go from `muted` to `text` |
| Done | The name in `muted` with a line through it: 1 px, the text's own colour (GPUI `StrikethroughStyle`); its second part at `rgba(255, 246, 236, 0.36)`, struck too; "Yours" is not struck. A count's figure turns `muted`. Rows never move; no icon, no other colour. A group whose rows are all done is struck through and its "6 of 6 done" turns `good` |
| Check button | Tick rows only: 20 × 20 px, 5 px radius, Lucide `check` 12 px at stroke 2.5 in `text` on `soft`; pointer on it: `hot`. On a done row, Lucide `undo-2` 12 px at stroke 2.25 in `muted`, no fill; pointer on it: `soft`, `text`. 6 px before the × slot. Tooltip under it: "Mark done" / "Undo", one line, 11.5 px / 500 `text`, 4 × 8 px padding, 6 px radius, `popover` colours |
| × slot | 20 px on every row, kept when empty so nothing shifts. The button: 20 × 20 px, 5 px radius, Lucide `x` 12 px in `muted`; pointer on it: `soft`, `text`. On a hidden row listed by Show: Lucide `eye`, always shown |
| Hover × rule | Check and × are shown while the pointer is in the row (enter shows, leave hides; moves inside change nothing) or while the row has keyboard focus. After a hide or removal, the row now under the pointer keeps them hidden until the pointer moves |
| Count | Lucide `chevron-down`, the figure, Lucide `chevron-up`, 1 px apart. Arrows 20 × 20 px, 5 px radius, 12 px at stroke 2.25, `muted`; pointer on one: `soft`, `text`; pressed: `press`; at 0 or N the arrow is `faint` at 0.45. Figure 46 × 20 px, 5 px radius, centred, "9" 12.5 px / 600 `text` and "/14" 11 px / 500 `faint` 1 px after; pointer on it: `soft`; pressed: `press` |
| Amount | Empty: "—" 12.5 px / 500 `faint` in the figure's place, 20 px gaps for the arrows, the name `muted`. Tracked: "Full in 2 d 16 h" 11.5 px / 500 `muted` before the count's controls; the figure reads as a field under the pointer (`track` fill, 1 px inset `rgba(255, 240, 225, 0.12)`). Full: the row on `full-fill`, the warning glyph in the gutter, "Full, being wasted" / "Full, spend by Tue 09:00" in `warn` / 600, the figure `warn` with "/840" at `warn` 0.7 |
| Amount field | Replaces the arrows and figure: 112 × 22 px, 6 px radius, `track` fill, 1 px `rgba(255, 246, 236, 0.55)` border, 7 px left padding; the typed figure right-aligned, 12.5 px / 600; "/21" 11 px `faint`; an 18 × 18 px clear button (4 px radius, Lucide `x`; on a game value it goes back to the game's value, else it stops tracking) |
| Group row | Chevron in the gutter, the name, then "2 of 6 done" 11.5 px / 500 `muted` (`good` when all done) at the row's end, before the × slot |
| Hidden line | Ends each section (and an open group) with hidden rows: 20 px, 11.5 px / 500 `faint`, 22 px in (40 in a group): "2 hidden · Show". Show / Hide again: `muted` / 600, underlined at `rgba(255, 246, 236, 0.25)`, 3 px offset; pointer over: `text`. Hidden rows listed by Show: everything but the put-back button at 0.42 opacity |
| Add line | The list's last line, 6 px above: 26 px, 6 px radius, Lucide `plus` 12 px `faint` in the gutter, "Add a to-do" 12.5 px / 500 `muted`; pointer over: `soft` fill, `text` |
| Add form | In the add line's place: `rgba(255, 240, 225, 0.05)` plate, 6 px radius, 6 px padding, 6 px between its two lines. Line 1: the 26 px input ("Name" placeholder), the small text button "Add" (22 px, Lucide `plus` 11 px; 0.4 opacity while the name is empty), Cancel as a 20 px × button. Line 2: segmented Tick \| Count, "of" with a count stepper (figure 26 px wide), then Daily \| Weekly pushed right. Segmented: track `rgba(0, 0, 0, 0.30)`, 7 px radius, 2 px inset and gap; options 20 px, 5 px radius, 11.5 px / 600 `muted`, selected `hot` with `text` |
| Notes | The overlay note, 6 px under the character selector, for 4 s: "Removed Legion check-in." with an Undo link at its end (Lucide `trash`), or a `good` note "Added Arena of Tactics wins to This week." (Lucide `check`) |
| Footer | The 26 px window footer, 5 px gaps: Lucide `clock` 12 px `faint`, "Daily reset" with "09:00" in `text` / 600, a `faint` "·", "Change" as a link (as Show), then "Weekly on Wednesday" in `faint` pushed right |
| Scrolling | The list scrolls inside the window with the 3 px `scroll-thumb`, 4 px from the right; 11 px right padding while it overflows. 3 px of room around the rows so a focus ring is never clipped |
| Focus ring | `focus-ring`, 2 px, 1 px outside the focused control (the row for tick and group rows, the figure on counts and amounts, a button), so a row's ring fits the 2 px gap between rows. Text fields show focus with their brighter border, no ring |
| Opening it | The toolbar's checklist (Lucide `list-checks`), lit while the window is open; the meter's "…" menu no longer has it |

- **One list per character**; Duty Missions, Shugo Festival keys, Invasion
  keys and the Unknown Fissure are counted per account and server (shared by
  your characters there, "Account"). The selector follows the character in
  game.
- **Focus**: the window never activates, except while a field takes the
  keyboard (an amount, a value typed over the game's, the new to-do's
  name), as the PvP name field does.

### Values from the game

Approved on 2026-10-08 (the to-do autofill round,
[../todos-autofill/README.md](../todos-autofill/README.md)). When you log in
while Kisk runs, the meter open or not, the game sends every ticket (the
regular pool it refills and an extra pool from items), then each change.
Caps and refills come from the game's ticket table.

| Element | Value |
|---|---|
| Source line | Under the character selector (2 px into its 6 px gap), 4 px above the list: the meter's hint line, 20 px, 4 px inset, 6 px gap, Lucide `gamepad-2` 12 px `faint`, 11.5 px / 500 `muted`, times and names `text` / 600, "·" in `faint`, cut off with an ellipsis. "From the game, live · logged in at 19:02"; "From the game at login, Sun 22:40 · refills since estimated"; "No game values yet. Log in with Sefa while Kisk runs."; "Tavrin not seen at login yet · Account rows from Mirelle" |
| Game figure | No arrows: a 14 px gap where the one-less arrow was, the 46 px figure ("7" 12.5 px / 600, "/21" 11 px `faint`), the extra pool in a 26 px slot after it, 1 px apart, so the block keeps the stepper's 88 px and nothing else in the row moves. Under the pointer it reads as a field (`track` fill, 1 px inset `rgba(255, 240, 225, 0.12)`); a click or Enter types over it |
| Extra pool | "+3" 11.5 px / 600 `muted`, left-aligned in its 26 px slot ("+380" fits); never yellow, never part of the cap or of "Full in …" |
| Weekly rows | Done of target: done = target − entries left ("5/14" for 9 left of 14); done (struck through) at 0 left; extra entries in the extra-pool slot ("+7"). A final-boss tick is done when its ticket reaches 0 |
| Typed over | The typed number in the figure's place, Lucide `pencil` 12 px `faint` in the gutter (the full warning wins the gutter when both apply); the extra pool stays the game's. The game's next value for that ticket replaces it |
| Figure tooltip | The control tooltip allowed two lines, up to 250 px: the source and time in `text`, the detail in `faint` ("125 at your login Sun 22:40." / "+120 since: eight refills of 15, estimated."; "Typed by you. The game said 7." / "Its next value replaces yours: a spend, a refill or your next login.") |
| At login | A good note for 4 s under the source line: "Filled from the game at login. Values you typed were replaced." |
| Above the cap | Normal, never an error or a colour. A regular pool above the cap in force (membership off): shown as is, never full, no estimate |
| Conquest dungeons | A folding group like Sanctuary, one row per dungeon (14, +2 a day); the group row says "10–13 of 14 each", or "1 full" in `warn` / 600. To confirm in game |
| Refill hour | Shugo Festival keys +3 and Invasion keys +1 at the daily reset (the ticket table's daily clock, 16:00 KST = 09:00 CEST) |

- **Caps**: Shugo Festival keys 12 (21 with membership), +3 a day, extra
  pool up to 30; Odyle Energy 560 (840 with membership), +15 every 3 h;
  Invasion keys 7, +1 a day. The membership switch picks the membership cap
  of every ticket that has one; off again, nothing is cut.
- **Yours to count**: Duty Missions, Duty Commands and your own to-dos keep
  the arrows, and so does every row before the game has filled it. Before
  the game fills a row, a typed number is your total: at or above the cap it
  counts as full. Typed numbers may be above the cap; nothing cuts them.
- **Between sessions**: the last game values plus the refills the table
  knows since, up to the cap; spends are never guessed. A weekly row seen
  before Wednesday's reset starts over at the reset.
- **Start over** (Settings) empties what you typed; the game's values stay.
  There is no switch to stop filling from the game.
- **To confirm in game** (drawn dashed, built with the drawn default): the
  Deus Research Base and Shattered Arkanis cap (4); Conquest dungeons as one
  ticket per dungeon; what the game shows for keys above 21 (premium items),
  which decides whether a typed number is your total (built as drawn); which
  of tickets 102 and 103 is Abandoned Balaur Fortress or Fafnium
  Laboratory; the exact reset hour; Ordeal's ticket (matched by name only).

## Fight report

The end-of-fight report for boss fights ([report.html](report.html)),
approved on 2026-10-07: one table with every figure, a player's details
inside the same window, opened from the boss summary card
([meter.html](meter.html), "Summary" states) and from boss records in
[history.html](history.html). Boss fights only; other fights never offer
it. It uses the overlay values above: `card` at the user's opacity,
`card-border`, no shadow, Barlow Semi Condensed, the meter's content scale,
the window header and footer, the meter's bars and rank badges.

### Report window

| Element | Value |
|---|---|
| Window | 520 px wide (the detail window's); card padding 6 px top, 8 px sides and bottom. Height hugs the content; the player table scrolls past 8 rows, the deaths list stays under it. All times the content scale |
| Header | The 28 px window header: Lucide `file-text` 13 px in `muted`, "Fight report" 13.5 px / 600; the outcome chip pushed right (20 px pill, 11.5 px / 600: `chip-good` "Defeated" with Lucide `check`, or `chip-warn` "Fight ended" with Lucide `hourglass` and "may resume" 11 px / 500 `muted`); the close tool button (22 px) flush right |
| Boss block | 4 px inset, 2 px above: the boss 14.5 px / 600, cut off with an ellipsis; the fight time 14 px / 600 at the end, never cut. Under it, 3 px apart, one line 11.5 px `muted`: where (with the difficulty), when, "6 players", joined by " · " (a field boss has no place) |
| Key figures | The detail window's plate: 6 px above, `track`, 8 px radius, 7 × 10 px padding; five equal cells 4 px apart, Damage, Party DPS, Healing, Damage taken, Deaths; label 11 px `faint` above the value 16 px / 600, 4 px apart, both at the left of the cell |
| Player table header | 6 px under the plate; the 20 px table header, 11 px / 500 `faint`: "Player" and the count in `faint` 6 px after, starting on the names (52 px in); the five column labels over their figures, right-aligned, ending 27 px from the right (35 px while the table scrolls) |
| Columns | Damage 54, DPS 46, Healing 52, Taken 50, Deaths 40 px, 6 px apart, figures right-aligned; the name keeps the rest (about 150 px: a 26-character name with a server tag) and is cut off with an ellipsis |
| Sorting | A click on a column header sorts by it, highest first (Deaths: most deaths, then damage). Active header `text` / 600 with Lucide `arrow-down` 10 px at stroke 2.5, 2 px after; the other sortable headers underlined at `rgba(255, 246, 236, 0.25)`, 3 px offset. Damage by default; the sort is kept while the window is open, also across a player's details |
| Player row | The meter's row: 22 px, 3 px apart; the 18 px rank badge (`you-badge` for you) with 8 px to the bar; the track (4 px left, 9 px right inset), 16 px class icon, the name 13 px / 500 (you 600; with "Server" on, the tag 11 px `muted` 5 px after), the five figures, the 12 px chevron slot. Bar: the player's class shade, scaled to the top value of the sorted column, floor 6%; no bar at zero. Rank follows the sorted column |
| Figures | Sorted column `text` 13.5 px / 600; the others `muted` 12.5 px / 500. Nothing in a column: "—" in `faint`, no bar, the name kept. Deaths: Lucide `skull` 11 px and the count in `bad` / 600, 3 px apart; zero "0" in `faint` / 500 |
| Pointer on a row | Enter: a `rgba(255, 240, 225, 0.07)` layer over the whole track (fill included), 5 px radius, and Lucide `chevron-right` 12 px in `muted` in the chevron slot; leave: both go. Never on movement inside the row. The chevron slot is kept on every row, so nothing shifts. The whole row is the click target |
| Scrolling | Past 8 rows the table is 197 px tall (8 rows) and scrolls with the 3 px `scroll-thumb`, 4 px from the right; the rows' right edge and the header's end move 8 px left (an 8 px gutter for the thumb), so each label stays over its figures. A list too short to scroll has no gutter. 3 px of room around the rows so a focus ring is never clipped |
| Deaths section | The section label (24 px, 6 px above): Lucide `skull` 12 px `faint`, "Deaths" 12 px / 600 `muted`, the count 11 px `faint`. Then a 20 px header: "Time" right-aligned over the times, "Player" over the names, "Killed by", "Last hit" at the end |
| Death row | 24 px, 3 px apart, `track`, 5 px radius, 6 px left and 9 px right inset, 6 px gaps: the time into the fight (m:ss) 12.5 px / 600 `muted`, right-aligned in 32 px; 128 px for the 16 px class icon and the name 13 px / 500 (you 600, ellipsis); "by" 11.5 px `faint`, 4 px, the monster 13 px `text`, its skill 11.5 px `muted` 5 px after, cut off with an ellipsis; the last hit 12 px `muted` at the end, right-aligned. In fight order. Pointer on it (enter and leave only): `rgba(40, 36, 32, 0.60)`; a click opens that player |
| Nobody died | One 24 px line, 6 px inset: Lucide `check` 12 px and "Nobody died" 12 px in `good` |
| Footer | The 26 px window footer: Lucide `mouse-pointer` 12 px and "Click a player for their skills, buffs, healing and damage taken", 11.5 px `faint`, 5 px apart |
| Focus ring | `focus-ring`, 2 px, 1 px outside the row or death row (6 px radius), so it fits the 3 px gap |

### Player details

| Element | Value |
|---|---|
| Place | Inside the report window, replacing the report; opened by a click on a player's row or death row |
| Header | The 28 px header with no left inset: "‹ Report", the quiet text button (22 px pill, no fill, 6 px left and 9 px right padding, Lucide `chevron-left` 12 px, "Report" 12 px / 600 `muted`; pointer over: `hot` fill); the 20 px class icon (10 px radius), the name 14.5 px / 600 (ellipsis), the server 12 px `muted`, 7 px apart; combat power with its 14 px icon, 12 px `muted`; the close tool button flush right |
| Fight line | 18 px, 4 px inset, 6 px gaps, 11.5 px `muted`: Lucide `file-text` 12 px, the boss, a `faint` "·", when, "·", the fight time |
| Key figures | The report's plate: Damage, DPS, Healing, Damage taken, Deaths; a zero ("0") in `faint` |
| Hit types | A second plate 4 px under it, 6 × 10 px padding: Critical, Back, Front, Double, Perfect, Parry, Multi-hit; label 11 px `faint`, the count 13 px / 600 `text` and the rate 11 px / 500 `muted` 4 px after ("665 41.9%"), at the left of each cell |
| Deaths plate | Only for a player who died: 4 px under the hit types, `bad` at 0.10 (`rgba(247, 168, 168, 0.10)`), 8 px radius, 6 × 10 px padding, 18 px lines 3 px apart, 12 px: Lucide `skull` 12 px and "Died at 2:47" in `bad` / 600, the monster in `text` and "· its skill" in `muted` (ellipsis), "last hit 52.3K" 11.5 px `muted` at the end |
| Tabs | The segmented tabs, 8 px above: Damage, Buffs, Healing, Damage taken, each with its count 11 px `faint`. Opens on Damage; the tab is kept when the next player opens. Not recorded: Healing and Damage taken at 0.4 opacity, no count |
| Damage tab | Header: "Skill" and the count, then Hits 40, Crits 40, Crit % 46, Damage 92 px (active, with the arrow) and a 14 px chevron column; starts 30 px in, ends 9 px from the right (17 px while it scrolls). Rows 24 px, 3 px apart, the player's class colour; 18 px skill icon (4 px radius; none in the game data: a quiet `rgba(255, 240, 225, 0.10)` square), the name, Hits and Crits and Crit % `muted`, Damage 13.5 px / 600 with its share 11 px / 500 `muted` 4 px after, Lucide `chevron-down` 12 px in `faint` (`text` under the pointer). Sorted by damage; scrolls past 9 rows (an opened row counts as 4) |
| Opened skill row | One at a time; the expanded-row block (6 px radius, `rgba(255, 240, 225, 0.05)`, 8 × 10 px padding, 3 px above, 2 px below), four columns, 8 px between lines: Perfect, Double, Front, Back, Parry, Multi-hit as count and rate, Multi-hit damage, Spec, Min, Max, Average, Total; label 11 px `faint`, value 12.5 px / 600. Spec: five 7 px squares, 2 px radius, 3 px apart, taken in cream, free at `rgba(255, 240, 225, 0.16)`. The chevron turns to `chevron-up` |
| Buffs tab | Header 20 px, 11 px `faint`, 4 px left and 12 px right inset: "Buff and caster", "Up for" over 62 px and "of 5:12" over 40 px, right-aligned. The list always scrolls (286 px) with the thumb: "On Mirelle", then "On Basilus the False" (24 px labels, 12 px / 600 `muted`, count 11 px `faint`). Rows 7 px apart: 24 px buff icon (5 px radius), the name 13 px / 500 (ellipsis), the caster 11 px `muted` after a 12 px class icon; under them the 4 px coverage bar (2 px radius, `track`, the caster's class gradient); then the time up as m:ss ("4:01") 13.5 px / 600 right-aligned in 62 px and the share of the fight 12 px `muted` right-aligned in 40 px. Longest first |
| Healing tab | The Damage tab's table: Heals 40, Crits 40, Crit % 46, Healed 92 px; no chevron (its slot kept). No overheal |
| Damage taken tab | The table without icons (plain track, 8 px left inset; header 13 px in): the monster skill, the monster 11 px `muted` 5 px after; Hits 40, Largest 46, Damage 92 px with the share of this player's damage taken. Bars in `hp`, `rgba(232, 115, 95, 0.16 → 0.55)` |
| Empty tab | The empty state: "No healing" / "No damage taken" 14 px / 600, then one `muted` sentence ("Saebrin cast no heals in this fight.", "Nothing hit Ferelith in this fight.") |
| Focus ring | As in the report: 2 px, 1 px outside the tab or row, 6 px radius |

### Not recorded

Fights saved before the report existed have no healing, damage taken or
deaths. The report still opens (from History):

- The plates show "--" in `faint` for Healing, Damage taken and Deaths.
- The table's Healing, Taken and Deaths cells show "--" in `faint`; their
  headers are plain `faint`, not underlined, and cannot be clicked.
- The overlay note under the plate (6 px above, Lucide `info`): "Healing,
  damage taken and deaths were not recorded for this fight: it was saved
  before the meter kept them. Damage, skills and buffs are complete."
- Deaths: the label without a count, then the quiet line "Not recorded for
  this fight" in `muted`.
- Player details: the same "--", the note 4 px above the tabs, Healing and
  Damage taken tabs dimmed, opening to "Not recorded / This fight was saved
  before the meter kept healing." (or "damage taken").

### Figure alignment

Plates left, tables right (decided 2026-10-07). The key-figure and hit-type
plates keep label above value at the left of each cell, as the detail
window does. Every table puts its figures and its headers at the right of
each column, so decimal points and the K and M suffixes line up: the player
table, the deaths list's time and last hit, the skill, Healing and Damage
taken tables, and the Buffs tab's "Up for" and share. Names, skill names,
icons, rank badges, bars and chevrons stay where they are. A header always
sits over its figures: a list that scrolls has the 8 px thumb gutter in its
rows and its header; a list that does not scroll has neither.

### Opening it

| Element | Value |
|---|---|
| Summary card button | "Open the fight report" (French "Ouvrir le rapport de combat"): full width under the skills (under the hero on the party card), 8 px above; 24 px tall, 7 px radius, `soft` fill (pointer over: `hot`), Lucide `file-text` 12 px and 12 px / 600 `text`, centred, 6 px apart. On the final and the provisional ("Fight ended, may resume") card, personal or party. Makes the summary card 32 px taller. Locked (click-through): no button, as no Copy and no × |
| History report button | On boss records only, before Delete (6 px apart): the 20 px record button, 7 px radius, Lucide `file-text` 12 px in `muted`; pointer over: `soft` fill, `text`, and the tooltip "Fight report" under it (one line, 11.5 px / 500 `text`, 4 × 8 px padding, 6 px radius, `popover` colours). Mob records keep the 20 px slot empty |
| Crown | Boss records only: Lucide `crown` 12 px at stroke 2 in `hp` (`#e8735f`), in a 12 px slot at the record's left edge, 5 px before the name; the name keeps its ellipsis. Not a button, no tooltip; the record's accessible name starts with "Boss,". Mob records keep the empty slot, so every name starts at the same edge |

- **Window**: an overlay window of its own like History and the detail
  window (transparent, no-activate, tool window). One report window at a
  time: opening it again brings it forward; a report for another fight
  replaces it. The summary card stays; × on the card still returns to the
  meter.
- **Fight ended, may resume**: the report shows what the meter has, with the
  `chip-warn` outcome; if the fight resumes, the window keeps the figures it
  opened with and the next summary offers a new report.
- **Back**: "‹ Report" returns to the report with its sort and scroll as they
  were. Esc does not reach a non-activating window.
- **Keyboard** (accessibility tools only): report: the sort headers, the
  rows, the death rows; Up and Down move, Enter opens; each row is a button
  named "Saebrin, open details", a death row "Saebrin died at 2:47, killed by
  Basilus the False, Sword Aura Blast". Player details: "‹ Report", the tabs
  (Left and Right), the rows; Enter opens a skill row.
- **Icons**: Lucide `file-text` is in `crates/app/assets/icons/` already;
  `crown`, `skull`, `chevron-left` and `mouse-pointer` are added from Lucide
  (ISC) when the report is built, with their `assets.rs` entries, as the
  other icons were. `chevron-right`, `chevron-down`, `chevron-up`,
  `arrow-down`, `check`, `hourglass`, `info`, `x` are there already.
- To confirm on a recording before building: which effects count as heals
  (shields, life drain such as Blood Absorption; the drawing counts direct
  heals, heals over time and self-heals), and that a death can be tied to
  the hit before it.

## Hub

Approved on 2026-10-08 ([../hub/README.md](../hub/README.md) is the record of
how it was chosen). Kisk is a set of tools over the game: at launch only the
toolbar opens, and every tool opens and closes from it. Pages:
[toolbar.html](toolbar.html), [resources.html](resources.html),
[prompt.html](prompt.html), [tray.html](tray.html), the meter's "…" menu and
× in [meter.html](meter.html), Settings › Overlay › Window in
[settings.html](settings.html). Styles: `hub.css`; drawing: `hub.js`.

### Toolbar

| Element | Value |
|---|---|
| Window | An overlay card of its own: `card` at the user's opacity, `card-border`, no shadow; 4 px padding, 12 px radius (the card's 14 px reads as a pill at this height); one row, 2 px gaps; the content scale. Horizontal only |
| Order | Grip; DPS meter (Lucide `swords`), PvP meter (`crosshair`), To-dos (`list-checks`), Fight history (`crown`), Reminders (`bell`), Settings (`settings`); a rule; Odyle Energy (`zap`), Shugo Festival keys (`key-round`), the warning; a rule; fold (`chevrons-left`), quit (`x`) |
| Grip | 14 × 26 px, Lucide `grip-vertical` 12 px in `faint`; the bar's empty space drags too |
| Tool | 26 × 26 px, 7 px radius, icon 14 px in `muted`. Pointer over: `soft`, `text`. Lit (the tool is open): `hot`, `text`, the icon 3 px higher and a 4 × 4 px dot (`currentColor`, the pin dot's size) 2 px above the bottom edge. Reminders paused: Lucide `bell-off` in `faint` |
| Fold and quit | 20 × 26 px, icon 12 px in `faint`; pointer over: `soft`, `text` |
| Rule | 1 × 16 px, `rgba(255, 240, 225, 0.18)` (the capsule's), 5 px each side |
| Count | 26 px tall, 7 px radius, 7 px sides, 5 px gaps: icon 13 px `muted`; the regular pool 12.5 px / 600 `text`, "/840" 11 px / 500 `faint` 1 px after; the extra pool "+380" 11.5 px / 500 `muted` 3 px after. Icons only (no names). Pointer over: `soft`. Resources open: `hot`. Before any game value: "—" in `faint` / 500 |
| Count, full | Regular pool at its cap and a refill coming: `full-fill`, the icon and figure `warn`, "/21" `warn` at 0.7, the extra pool `muted`; pointer over: `warn` at 0.20 |
| Warning | Only while other resources are full and a refill is coming: 26 px, 7 px radius, 7 px sides, 4 px gap, `full-fill`; Lucide `triangle-alert` 13 px and the number 12.5 px / 600 in `warn`; pointer over or open: `warn` at 0.20 |
| Folded | Grip, unfold (`chevrons-right`, as fold) and, when anything is full (Odyle and Shugo keys included), the warning with that number; a click on it unfolds |
| Tooltip | `popover` colours, 4 × 8 px padding, 6 px radius, 11.5 px / 500 `text`, a second line in `muted`; 6 px under the control, its left edge on the tool's (its right edge on a count's, the warning's, fold's and quit's); after 500 ms |
| Focus ring | `focus-ring`, 2 px, 1 px outside the button (accessibility tools; Left and Right move along the bar) |

- **Tooltips**: "DPS meter / Fights are tracked while it is open." (open:
  "Click to close. Fights stop being tracked."), "PvP meter", "To-dos",
  "Fight history", "Reminders" ("Reminders / Paused"), "Settings", "Fold the
  toolbar", "Unfold the toolbar", "Quit Kisk / Closes every tool."; a count:
  its name, then "245 of 840, +380 extra · full in 2 d 14 h. Click for all
  resources." ("Full, spend by Tue 09:00", "Full, being wasted"); before any
  game value "No value yet. Log in while Kisk runs."; the warning "2 more
  full".
- **Clicks**: a tool opens or closes its window; a count opens or closes
  the Resources window; the warning opens its dropdown; quit ends the
  process at once, no confirmation.
- **Window**: no-activate, tool window, like the reminder card; never
  click-through (Alt+CapsLock locks the meter only). Opens where you left
  it (top centre the first time); tools keep their own places.
- **Shortcuts**: Alt+Shift+CapsLock hides the toolbar and every open tool,
  and shows them again; the tray icon's "Show toolbar" too.
- **What is tracked when**: capture always runs; fights (damage, heals,
  damage taken, deaths, fight history) only while the meter is open;
  resources always.

### Toolbar dropdowns

| Element | Value |
|---|---|
| Place | Drawn inside the toolbar's window, which grows downward while one is open (upward when the toolbar sits low on the screen); 6 px under the button. The warning's: right edges aligned, 268 px. The bell's: left edges aligned, 236 px |
| Frame | The `popover` (9 px radius, 4 px padding, 26 px items, 6 px gaps), items 500; pointer on an item: `hot`; the 1 px divider at 0.18, 4 px inset |
| Label | 24 px, 8 px inset, 6 px gap, 11 px / 500 `faint` with its 12 px icon: "Full, a refill is coming" (`triangle-alert`), "Next reminders" (`bell`) |
| Warning rows | The name (ellipsis), then right-aligned columns: the figure in 42 px ("7" 12.5 px / 600 `warn`, "/7" 11 px / 500 `warn` 0.7) and the refill in 74 px ("refills" 11 px `faint`, the time `muted` / 600). After the divider: Lucide `package` in a 12 px lead slot, "All resources", Lucide `chevron-right` 12 px. A click on any item opens the Resources window |
| Bell rows | The next three reminders switched on, the name and the time (`muted` / 600) right-aligned; the divider; "Pause reminders" (a checkbox item, the 12 px lead slot; the Settings master switch); "Reminder settings" with Lucide `settings` and `chevron-right` (opens Settings › Reminders) |
| Closing | A click elsewhere in the toolbar, a second click on its button, or the pointer leaving the toolbar and the dropdown |

### Resources window

| Element | Value |
|---|---|
| Window | The to-do window's frame: 370 px, the 28 px header with Lucide `package` 13 px in `muted` and "Resources", the "2 full" `chip-warn`, the close tool button; the character selector; the source line; the list 2 px under it; the 26 px footer |
| Rows | The to-do window's Recharges rows, in the same order, the same display and "Values from the game"; no × slot (nothing to hide), so names get 26 px more. Weekly entries stay to-dos |
| Footer | Lucide `clock` 12 px `faint`, "Daily reset" with "09:00" in `text` / 600, then "Membership caps" (or "Caps without membership") in `faint` pushed right |
| Behaviour | No-activate except while a field takes the keyboard; one at a time; opens where you left it. The toolbar's counts read lit while it is open; a click on a count closes it. Same data as the to-dos: a change shows in both windows |

### First-open prompt

| Element | Value |
|---|---|
| Place | Its own overlay window, placed as the reminder card: 6 px above the meter, left edges aligned (below it when there is no room); a reminder that comes meanwhile stacks 6 px above it |
| Card | The meter's width (310 px; 220 px over the capsule); 8 px padding; `card`, `card-border`, 14 px radius |
| Header | The reminder card's: 22 px, 4 px left inset, 6 px gap, Lucide `swords` 13 px `muted`, "Open the meter at launch?" 12.5 px / 600 (ellipsis) |
| Body | 23 px in (under the title), 4 px right, 3 px under the header: "Kisk starts with the toolbar alone. Open the DPS meter each time Kisk starts?" 11.5 px / 500 `muted`, 15 px line |
| Path | 6 px under the body, 23 px in, wrapping: Lucide `settings` 12 px `faint`, then "Settings › Overlay › Open the meter at launch" 11.5 px / 500 `text` (the last step 600), Lucide `chevron-right` 10 px `faint` between the steps, 4 px gaps |
| Buttons | 9 px under the path, right-aligned, 6 px apart: "Not now" (quiet text button) and "Yes, at launch" (tonal, `soft`); pointer over either: `hot`, `text` |
| After Yes | The good overlay note "The meter opens at launch from now on." (Lucide `check`) in the body's place, the path under it 6 px apart; 4 s, then the card fades out over 150 ms |

- **When**: once, the first time the meter is opened from the toolbar.
  Either answer is final. No answer: the card stays while the meter is open;
  closing the meter without answering asks again next time.
- **Window**: no-activate, clickable even while the meter is locked.

### Meter "…" menu and ×

| Element | Value |
|---|---|
| "…" | Keeps its place among the meter's tools; tooltip and accessible name "Boss setting"; `hot` while the menu is open |
| Menu | The `popover`, 240 px wide, 30 px from the card's top, 8 px from its right; the window grows downward while it does not fit. Items 26 px, 12.5 px / 500: "Treat Thornback Boar as a boss" (the header's target, ellipsis), "Not a boss", "Automatic"; a 12 px lead slot with Lucide `check` 12 px `muted` on the current choice, which is 600 |
| Nothing hit yet | A 24 px hint line, 8 px inset, 11.5 px `muted`: "Hit a mob to set whether it is a boss"; "Treat as a boss" and the other items in `faint` |
| × tooltip | Under the button, right edges aligned (32 px from the card's top, 6 px from its right): `popover`, 4 × 8 px padding, 6 px radius, "Close the meter" 11.5 px / 500 `text`, "Fights are tracked only while it is open." in `muted` |

- × closes the meter only: the toolbar stays and its meter icon goes dark.
  Fight history, PvP meter, To-dos and Settings are in the toolbar.

### Tray icon

| Element | Value |
|---|---|
| Icon | The app's icon (`crates/app/windows/icon.ico`); tooltip "Kisk" |
| Menu | Windows' own: "Show toolbar" (the default item, bold), a separator, "Quit" |
| Clicks | A left click does what "Show toolbar" does: brings back the toolbar and every tool that was open, unfolds a folded toolbar and brings it in front. "Quit" ends the process, as closing the toolbar does. No Windows notification |

### Not decided by the mockups

- The app's other names (title bars, About, the waiting card's "DPS Meter")
  change with the rename (TASK-23).
- French: the toolbar, its dropdowns and the prompt were drawn in French to
  check the room; the real text goes through i18n.
- Icons to add from Lucide (ISC) when built: `crosshair`, `grip-vertical`,
  `key-round`, `zap`, `bell-off`, `chevrons-left`, `chevrons-right`,
  `package`, `settings`, `gamepad-2`, `pencil`; `swords`, `bell`, `crown`,
  `list-checks`, `triangle-alert`, `chevron-right` are in
  `crates/app/assets/icons/` already.

## Main window (GPUI Kit)

The startup gate, Home and Settings use GPUI Kit components dressed in the
same colours, so the window and the overlays read as one product. Dark only,
opaque, no artwork or video.

### Type

| Role | Family | Size / weight |
|---|---|---|
| Body text, descriptions, labels | Segoe UI Variable Text | 13 px / 400–500 (row label 13.5 / 500, description 12) |
| Page title | Bahnschrift | 21 px / 600 (gate 19) |
| Large button, figures (elapsed time, slider values, best DPS), Kbd | Bahnschrift | 13–22 px / 600 |
| Group title | Segoe UI Variable Text | 12.5 px / 600, `muted` |

French strings run 20–30% longer; Home was checked in French at these sizes.

### Colour

| Token | Value | Use |
|---|---|---|
| `win-side` | `#0e0c0b` | Title bar and sidebar |
| `win-canvas` | `#141210` | Content area |
| `win-surface` | `#1b1815` | Settings groups, cards, status tiles |
| `win-raised` | `#241f1b` | Menus, tooltips, dialogs |
| `win-line` | `rgba(255, 240, 225, 0.08)`; strong `0.14` | Group borders and row dividers; button and input borders |
| `primary` | `#f3ebe1` bg, `#17120d` text | The one bright button per view (Start meter, Open the meter, Continue now) |
| `selected` | `rgba(255, 240, 225, 0.11)` | Sidebar item, segmented option |
| `soft` | `rgba(255, 240, 225, 0.07)` | Secondary button fill, sidebar status line |
| `danger` | `rgba(247, 168, 168, 0.15)` bg, `bad` text | Stop meter, Stop recording, Clear cache, reset confirmation |
| `blocking` | `rgba(247, 168, 168, 0.07)` bg, `rgba(247, 168, 168, 0.35)` border | Gate check that blocks start-up |
| `scrim` | `rgba(8, 7, 6, 0.62)` | Behind a dialog |
| `focus-ring` | `rgba(243, 235, 225, 0.90)` (`primary` at 0.90) | Keyboard focus around any control |

Text, `good`, `warn` and `bad` are the overlay values.

### Components

| Component | Kit piece | Values |
|---|---|---|
| Window | borderless window | 960 × 620 (gate 600 × 470, not resizable), 9 px radius, 1 px `rgba(255, 236, 214, 0.12)` border |
| Title bar | `TitleBar` | 36 px; 20 px cream logo square (6 px radius) + "AION 2 DPS Meter"; minimise, maximise, close 46 × 36 (gate: close only) |
| Sidebar | `Sidebar` | 208 px; Meter, then a "Settings" label and nine sections (Reminders after Personal bests, To-dos after Reminders); items 32 px, 7 px radius; the meter's status line (dot + "Running · 12:41") at the bottom |
| Settings group | styled div | title above; 10 px radius, `win-surface`, 1 px `win-line`; rows ≥ 54 px, 11 × 16 px padding, 1 px dividers |
| Settings row | styled div | label and one-sentence description on the left (≤ 58ch), control on a stable right edge |
| Settings row, dimmed | styled div | While the section's master switch is off (Reminders): label and description at 0.5 opacity, the control at 0.4 and inert; every control keeps its own value |
| Focus ring | custom | `focus-ring`, 2 px, 3 px outside the control, radius the control's + 3 px; drawn only when the focus came from the keyboard |
| Button | `Button` | 30 px, 7 px radius, 13 px / 600; variants primary, secondary (soft + strong border), ghost, danger; small 26 px; large 46 px, 10 px radius, Bahnschrift 16 px |
| Switch | `Switch` | 36 × 20; off `rgba(255, 240, 225, 0.16)` with a cream knob; on cream with a dark knob |
| Slider | `Slider` | 4 px track `rgba(255, 240, 225, 0.14)`, cream range, 16 px cream thumb with a 2 px dark ring; value right-aligned in Bahnschrift |
| Select and menu | `Dropdown` / `Select` | 30 px, 7 px radius, dark fill; open border `rgba(255, 246, 236, 0.55)`; menu `win-raised`, 9 px radius, 28 px items, tick on the current one |
| Segmented control | `Tabs` (pill style) or custom | track `rgba(0, 0, 0, 0.32)`, 8 px radius; 26 px options, `selected` fill |
| Kbd | `Kbd` | 22 px, 5 px radius, `rgba(255, 240, 225, 0.06)` with a 0.16 border, Bahnschrift 12 px / 600, joined by a faint "+" |
| Shortcut recorder | custom | 34 px field holding Kbd keys; recording: focus border, "Press the keys…", caret, "Esc cancels · Backspace turns it off"; conflict: rose border and an inline message; a ghost reset-to-default button beside it |
| Badge | `Tag` / `Badge` | 22 px pill; neutral, good, warn, bad; outlined 20 px variant ("Optional", "Upstream") |
| Alert | `Alert` or styled div | 9 px radius, tinted fill (bad / warn / good at 0.10), icon, bold first line |
| Status tile | custom | `win-surface`, 10 px radius; 30 px tinted icon square; label, value, detail |
| Table | `Table` | 34 px header in `faint`, 40 px rows with 1 px dividers, figures right-aligned in Bahnschrift |
| List row | styled div | ≥ 46 px; icon, two-line text, actions on the right (recordings, credits) |
| Tooltip | `Tooltip` | `win-raised`, 8 px radius, 12 px text, ≤ 250 px wide |
| Dialog | `Modal` / `Dialog` | 400 px, 12 px radius, `win-raised`, shadow `0 24 60 rgba(0, 0, 0, 0.6)` over `scrim`; actions right: Cancel, then the action |
| Overlay preview | custom | 262 px column beside the Overlay settings: the real meter view with sample data over a game-like backdrop |

### Settings → Reminders

| Part | Value |
|---|---|
| Place | Sidebar item after Personal bests, Lucide `bell`; no preview column (the test button shows the real card) |
| Lead | "A reminder before timed events, so you can get there in time. Times follow this PC's clock and are set for the European servers, which run on Berlin time." |
| How you are reminded | Event reminders (master switch, on); Remind me (select, 130 px minimum: 1, 2, 3 or 5 min before, 1 by default); Sound (switch, on: Windows' Chimes sound, once per card); Test reminder (secondary button "Show a test reminder", Lucide `play`) |
| Events | One switch per event in groups by how often it comes: Every hour (Shugo Festival, Dimensional Invasion), Every 3 hours (Rift portals, Watcher Kaira), Weekly (Artifact Siege, Abyss field bosses, Guardian Lord Nahma), Daily (Arena of Tactics opens, Reward keys refill). On by default: Shugo Festival, Dimensional Invasion, Rift portals |
| Master switch off | Every row under it dimmed (Settings row, dimmed); no card and no sound |
| Test shown | A good badge "Shown above the meter" with a check, 2 px before the button, for 4 s (Settings may cover the meter) |

### Settings → To-dos

Approved on 2026-10-06 with the to-do window; lead, Membership and Start
over changed on 2026-10-08 with the values from the game.

| Part | Value |
|---|---|
| Place | Sidebar item after Reminders, Lucide `list-checks`; no preview column |
| Lead | "The to-do window, opened from the toolbar. Counts and amounts fill from the game when you log in while Kisk runs." |
| Resets | Daily reset (Select, 130 px minimum: every whole hour 00:00 to 23:00, 09:00 by default; the list shows eight 28 px items at a time, opens scrolled to the chosen hour, 3 px thumb); Weekly reset (not a control: "Wednesday 09:00" in display type 13.5 px / 600 `muted`, following the daily hour) |
| Recharges | Membership (Switch, off; one for every character): "Higher caps on every character: Odyle Energy 840 instead of 560, Shugo Festival keys 21 instead of 12." |
| The game shows more | With the switch off and a game regular pool above the cap: the field message under the description, in `muted`, Lucide `info`, 5 px above: "The game shows **18 Shugo Festival keys** on Mirelle, more than 12. Do you have a membership?" (the figure and name in `text` / 600). No dialog, no automatic switch |
| Default to-dos | Hidden to-dos: secondary button "Restore default to-dos" with Lucide `rotate-ccw`, disabled while nothing is hidden. Start over ("…empties every count and amount you typed."): danger button "Reset to-dos completely…" |
| Restored | A good badge "6 to-dos back" with a check, 2 px before the button, for 4 s; no dialog |
| Confirm dialog | The Dialog: "Reset the to-dos completely?", two paragraphs 6 px apart (the first ends "Values from the game stay."), Cancel (focused, the focus ring drawn) then the danger "Reset to-dos" |
| After the reset | A good badge "To-dos reset" with a check before the button, for 4 s |

### Settings → Overlay, Window

Changed on 2026-10-08 with the hub.

| Row | Value |
|---|---|
| Open the meter at launch | First in the group; Switch, off: "When Kisk starts. Off: only the toolbar opens, and the meter opens from its icon." The first-open prompt's Yes turns it on |
| Lock overlay | Adds "The toolbar always takes clicks." |
| Show or hide everything | Alt + Shift + CapsLock as read-only Kbd keys (4 px apart), the stacked row: "Hides the toolbar and every open tool, and brings them back. The tray icon's Show toolbar brings them back too." Each shortcut is described on the row it acts on (the build has no Shortcuts page) |

### Custom work beyond GPUI Kit

Kit availability is from memory of gpui-component; check it against the
pinned version before planning.

- The shortcut recorder (capture a combination, Esc, Backspace, conflicts).
- The Settings overlay preview (render the meter view with sample data).
- Status tiles and the gate's check rows (styled divs).
- Recording and replay rows.
- The segmented control, if Kit's Tabs cannot take the pill style.
- An inline Alert, if the Kit version has none.
- The spinner, if Kit has no Indicator: a rotating Lucide `loader-circle`
  (`with_animation`).

### To confirm during implementation

- Clear cache: what is reset and what is kept depends on the Rust storage;
  the dialog copy follows that.
- WinDivert is bundled but no licence text for it is in the repository; About
  names it without a licence until one is added.

## GPUI notes

- Everything above maps to `bg`, `border_1`, `rounded`, `shadow`,
  `linear_gradient` (two stops) and `overflow_hidden`.
- Popovers are drawn inside the overlay window; the window must be tall
  enough to hold them while open.
- Text inputs need a GPUI text input (built for the overlays or taken from
  GPUI Kit).
- Bahnschrift SemiCondensed is a named instance of a variable font, and GPUI
  cannot select it by family name (checked in phase 2a); the overlays use the
  embedded Barlow Semi Condensed instead. `Segoe UI Variable` must be named
  with its optical size (`Segoe UI Variable Text`, `Display`, `Small`).
- Icons are single-colour Lucide SVGs; class icons are the PNGs in
  `crates/app/assets/game/class/`, skill and buff icons the PNGs in
  `crates/app/assets/game/skills/`.
- The reminder card is a second overlay window with the meter's styles
  (transparent, no-activate, tool window, click-through while locked), sized
  to the card and placed from the meter window's bounds in device pixels. The
  fade is the card's opacity inside the window; the window appears at full
  size, transparent.
- The to-do window is a third overlay window with the meter's styles
  (transparent, no-activate, tool window), sized to its content up to the
  list's maximum height. A done name is drawn with `StrikethroughStyle`; the
  check and × are drawn on every row and their visibility changes on the
  row's hover enter and leave (`on_hover`), never on mouse move.
- The fight report is an overlay window with the meter's styles
  (transparent, no-activate, tool window), 520 px wide and sized to its
  content. The row lift and chevron change on the row's hover enter and
  leave (`on_hover`), never on mouse move; the chevron is drawn on every row
  and only its visibility changes. The player table and every list that can
  overflow scroll inside the window with the thumb gutter; the player
  details replace the report's content in the same window, and the report's
  sort and scroll are kept while they show.
- The toolbar, the Resources window and the first-open prompt are overlay
  windows with the meter's styles (transparent, no-activate, tool window).
  The toolbar is never click-through and draws its tooltips and dropdowns
  inside its own window, which grows while a dropdown is open. The
  Resources window takes the keyboard only while a field is open.

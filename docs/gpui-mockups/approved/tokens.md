# Aether design tokens

The approved visual language for Aether's GPUI overlays and main window.
Sources of truth: [meter.html](meter.html) (round 1, D2 with bar end d and
rank b, plus the round 2 header and combat-power fixes), [pvp.html](pvp.html),
[detail.html](detail.html) and [history.html](history.html) (round 2), and
[gate.html](gate.html), [home.html](home.html) and
[settings.html](settings.html) (round 3). The shared overlay classes live in
[overlay.css](overlay.css), the main-window components in [app.css](app.css).
Later rounds extend this file only with values the user has approved.

Colours are sRGB. `a` is the user's background opacity (0–255 in Settings,
default 150, so `a = 150/255 ≈ 0.59`).

## Type

| Role | Family | Size / weight |
|---|---|---|
| All overlay text | Bahnschrift, SemiCondensed width | — |
| Hero figure (summary DPS) | | 36 px / 600 |
| Capsule DPS | | 17 px / 600 |
| Detail key figures | | 16 px / 600 |
| Boss name in summary, player in detail header | | 14.5 px / 600 |
| Row DPS, fight time | | 14 px / 600 (13.5 in PvP, detail and history rows) |
| Window title (PvP, history) | | 13.5 px / 600 |
| Body, player names | | 13 px / 500 (you: 650) |
| Values in footer and key-value lists, menu items, inputs | | 12.5 px / 500–600 |
| Percent column, gauge %, section labels, tabs, buttons | | 12 px / 600 |
| Secondary (CP, server, labels, system figures) | | 11–11.5 px / 500 |

- Figures are tabular (`tnum`).
- Labels are sentence case. No uppercase transforms, no letter spacing.
- Long names cut off with an ellipsis; numbers never wrap.

## Colour

| Token | Value | Use |
|---|---|---|
| `card` | `rgba(16, 14, 12, a)` | Overlay card and windows |
| `card-border` | `rgba(255, 236, 214, 0.10)`, 1 px | Card outline |
| `card-shadow` | `0 10 28 rgba(0, 0, 0, 0.42)` | Card drop shadow |
| `track` | `rgba(0, 0, 0, 0.34)` | Bar track, record plate, dark plate under text |
| `soft` | `rgba(255, 240, 225, 0.08)` | Tonal button fill, notes |
| `hot` | `rgba(255, 240, 225, 0.13)` | Active tool button, selected tab, highlighted menu item |
| `text` | `rgba(255, 246, 236, 0.96)` | Primary text |
| `secondary` | `rgba(255, 246, 236, 0.82)` | Combat power and server beside meter names |
| `muted` | `rgba(255, 246, 236, 0.66)` | Secondary text, % column, section labels |
| `faint` | `rgba(255, 246, 236, 0.44)` | Labels, counts, placeholders, table headers |
| `you-badge` | `rgba(255, 246, 236, 0.90)` bg, `#17120d` text | Your rank badge |
| `you-rim` | `rgba(255, 246, 236, 0.45)`, 1 px inside the track | Your bar |
| `good` | `#86e3a8` | Ahead of PB, defeated, good ping, live, success notes |
| `warn` | `#f2c66d` | Fight ended (provisional), history mode |
| `bad` | `#f7a8a8` | Behind PB, delete confirmation |
| `hp` | `#e8735f` | Boss gauge fill; health bars as a 0.16 → 0.55 gradient |
| `gauge-empty` | `rgba(255, 240, 225, 0.12)` | Empty gauge segment |
| `pb-new` | `rgba(250, 204, 21, 0.17)` bg, `#fde68a` text | New personal best chip |
| `chip-good` / `chip-warn` / `chip-bad` | `good`, `warn`, `bad` at 0.13–0.14 bg | Status chips and notes |
| `danger-fill` | `rgba(247, 168, 168, 0.14)` bg, `bad` text | Confirming button ("Delete?") |
| `pending-rim` | `rgba(247, 168, 168, 0.55)`, 1 px | The record a delete confirmation refers to |
| `neutral-fill` | cream `rgba(255, 240, 225, 0.04 → 0.16)` | Bars where no class is known (PvP kill stats) |
| `tag-attacker` | `rgba(232, 115, 95, 0.18)` bg, `#f6b4a6` text | "Last attacker" tag |
| `popover` | `rgba(30, 26, 22, 0.98)`, border `rgba(255, 236, 214, 0.14)`, shadow `0 10 24 rgba(0, 0, 0, 0.5)` | Menus, dropdowns, suggestions |
| `input` | `rgba(0, 0, 0, 0.34)` bg, border `rgba(255, 240, 225, 0.12)`; focus border `rgba(255, 246, 236, 0.55)` | Text fields and selects |
| `scroll-thumb` | `rgba(255, 240, 225, 0.28)`, 3 px wide | Scrolling lists |

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
| Widths | meter 310 px (340 with total damage); PvP 340; history 370; detail 520 |
| Row gap | 3 px |
| Badge to bar | 3 px margin + 5 px row gap = 8 px, matching the card's 8 px inset |
| Inside a bar | 4 px left, 9 px right, 6 px between icon, name and figures |
| Percent column | 38 px, right-aligned |
| Window header | 28 px; meter header 22 px |
| Section label row | 24 px, 6 px above |
| Footer row | 20 px (meter), 26 px (windows) |

## Interaction patterns

- **Meter under the pointer**: the seven controls take the gauge's 22 px
  line; the header keeps the boss name and fight time. The window never
  resizes on hover.
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
- Hover swaps (meter tool row) cross-fade in 100 ms.
- Chips and notes fade in over 150 ms. Nothing flashes or pulses.
- The capsule grows into the card over 180 ms on pointer over and returns
  after a 350 ms grace period.
- Frames are drawn only on a snapshot or while a tween runs.

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
| Group title | Segoe UI Variable | 12.5 px / 600, `muted` |

French strings run 20–30% longer; Home was checked in French at these sizes.

### Colour

| Token | Value | Use |
|---|---|---|
| `win-side` | `#0e0c0b` | Title bar and sidebar |
| `win-canvas` | `#141210` | Content area |
| `win-surface` | `#1b1815` | Settings groups, cards, status tiles |
| `win-raised` | `#241f1b` | Menus, tooltips, dialogs |
| `win-line` | `rgba(255, 240, 225, 0.08)`; strong `0.14` | Group borders and row dividers; button and input borders |
| `primary` | `#f3ebe1` bg, `#17120d` text | The one bright button per view (Start meter, Open Aether, Continue now) |
| `selected` | `rgba(255, 240, 225, 0.11)` | Sidebar item, segmented option |
| `soft` | `rgba(255, 240, 225, 0.07)` | Secondary button fill, sidebar status line |
| `danger` | `rgba(247, 168, 168, 0.15)` bg, `bad` text | Stop meter, Stop recording, Clear cache, reset confirmation |
| `blocking` | `rgba(247, 168, 168, 0.07)` bg, `rgba(247, 168, 168, 0.35)` border | Gate check that blocks start-up |
| `scrim` | `rgba(8, 7, 6, 0.62)` | Behind a dialog |

Text, `good`, `warn` and `bad` are the overlay values.

### Components

| Component | Kit piece | Values |
|---|---|---|
| Window | borderless window | 960 × 620 (gate 600 × 470, not resizable), 9 px radius, 1 px `rgba(255, 236, 214, 0.12)` border |
| Title bar | `TitleBar` | 36 px; 20 px cream logo square (6 px radius) + "Aether"; minimise, maximise, close 46 × 36 (gate: close only) |
| Sidebar | `Sidebar` | 208 px; Meter, then a "Settings" label and seven sections; items 32 px, 7 px radius; the meter's status line (dot + "Running · 12:41") at the bottom |
| Settings group | styled div | title above; 10 px radius, `win-surface`, 1 px `win-line`; rows ≥ 54 px, 11 × 16 px padding, 1 px dividers |
| Settings row | styled div | label and one-sentence description on the left (≤ 58ch), control on a stable right edge |
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
- Bahnschrift SemiCondensed is a named instance of a variable font; whether
  GPUI can select it by family name is unverified.
- Icons are single-colour Lucide SVGs; class icons are the PNGs in
  `public/aion2/class/`, skill and buff icons the PNGs in
  `public/aion2/skill/`.

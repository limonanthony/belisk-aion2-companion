# Kisk as a multi-tool, the hub: mockups

**Approved on 2026-10-08.** After Main listed the open questions and the
drawn defaults, the user answered "yup perfect": every Proposed item below
is agreed as drawn, the Reminders bell is the dropdown, the PvP icon is the
crosshair. The approved design now lives in `../approved/`: the
[toolbar](../approved/toolbar.html), the
[Resources window](../approved/resources.html), the
[first-open prompt](../approved/prompt.html), the
[tray icon](../approved/tray.html), the meter's "…" menu and × in
[meter.html](../approved/meter.html?states=menu-mob,menu-boss,menu-corrected,menu-waiting,close-tip),
Settings › Overlay › Window in
[settings.html](../approved/settings.html?states=overlay:window,overlay:on),
the to-do window opened from the toolbar in
[todos.html](../approved/todos.html?states=opening), the PvP header's
crosshair in [pvp.html](../approved/pvp.html), and the "Hub" values in
[tokens.md](../approved/tokens.md). Build from there; this folder is the
history of how it was chosen and is not updated any more. The open data
below stays open until the user's in-game comparison.

The round covered every screen the hub changes: the floating toolbar, the
new Resources window, the to-do window opened from the toolbar, the meter's
"…" menu, the first-open prompt, Settings → Overlay and the tray icon.

Open in a browser, no server needed: start at [index.html](index.html), which
links every page and every state (`?states=<id>` opens one state alone).

| Page | What it shows |
|---|---|
| [index.html](index.html) | Every page and state, the behaviour that is not drawn, what to answer |
| [toolbar.html](toolbar.html) | The toolbar: 19 states and a working one ("Try it"); selector **Counts** (icons only / with names) |
| [resources.html](resources.html) | The Resources window: 9 states; open-data selectors as in the to-do round |
| [../todos-autofill/index.html](../todos-autofill/index.html) | The to-do window filled from the game (drawn earlier today, folded into this round) |
| [meter.html](meter.html) | The meter's "…" menu: 5 states and today's menu for comparison |
| [prompt.html](prompt.html) | The first-open prompt: 5 states |
| [settings.html](settings.html) | Settings → Overlay: 3 states |
| [tray.html](tray.html) | The tray icon: tooltip, menu, pointer on Quit |

Built on the approved design, which is not edited: the pages load
`../approved/shared.css`, `overlay.css`, `app.css`, `kit.js` and `app.js` as
they are, `../todos-autofill/todos.css` and `autofill.css` (the to-do window's
rows, which the Resources window shares), and the bundled Barlow Semi
Condensed, class icons and app icon from `crates/app/`.

| File | What it is |
|---|---|
| `hub.js` | Drawing shared by the pages: toolbar, dropdowns, Resources window, meter with its menu (the approved meter's functions, trimmed), prompt, reminder card; invented values; Lucide icons not in `kit.js` |
| `hub.css` | Only what this round adds (values from tokens.md; new ones marked "new"), the reminder card's classes copied from `../approved/reminders.html`, page-only chrome |
| `meter.css` | `../approved/meter.html`'s `<style>` block, copied unchanged, to draw the approved meter |

Values, times and characters are invented. Caps and refills are the game's
ticket table as in our notes, as in the to-do round. Clock: Monday 21:48;
Mirelle logged in at 19:02; membership on.

## Decisions

### Agreed

1. **A toolbar as the hub.** "a small floating toolbar with icons with like a
   weapon for the dps meter, a checklist icon for the todos and in that small
   toolbar we also put currencies ? We could also put icons for fight
   history, settings, etc..." Tools: DPS meter (crossed swords), PvP meter,
   To-dos (checklist), Fight history (crown: crown = history and boss
   records, skull = deaths), Reminders (bell), Settings (gear). A lit icon
   means the tool is open; a click opens or closes it.
2. **Horizontal only.** "Let's do horizontal only because of the counts"
3. **Launch.** "It opens alone, maybe we a setting to open the dps meter
   automatically and a prompt for the first time the user opens the dps
   meter if he wants to open by default the dps meter when running the
   program ? (With a path on where he can find the setting ... e.g: Settings
   -> Overlay -> etc...)" → at launch only the toolbar opens; Settings →
   Overlay → "Open the meter at launch", off by default; a one-time prompt
   the first time the meter is opened from the toolbar, naming that path,
   with two buttons.
4. **Counts and the warning.** "Maybe we could put Odyle, Shugo Keys and
   show a warning icon with a dropdown with the others that are full and
   will be replenished soon ?" → Odyle Energy and Shugo Festival keys always
   shown; a warning with a number only while other resources are full and a
   refill is coming; its dropdown lists them and ends with "All resources".
   A click on Odyle or Shugo keys opens the Resources window.
5. **Resources window.** "move the count of resources outside of this
   window, some users might want a window only for these resources to be
   seen easily (e.g: in a floating window like the todos) we can still keep
   them in todo too" and "Yes I think a full resources window would be
   useful" → a floating window of its own for every game-filled resource of
   the current character; the to-do window keeps its counts.
6. **The full-pool warning in the to-do window stays as drawn.** "That's good
   then !"
7. **Display rules** (2026-10-07, from the to-do round): regular pool against
   its cap, the extra pool beside it when not zero ("7/21", "+3"); above the
   cap is normal, never an error; "Full, spend by …" / "Full, being wasted"
   only when the regular pool is at its cap and a refill is coming; the
   membership switch picks the membership caps (Shugo keys 12 → 21, Odyle
   560 → 840); Shugo keys, Invasion keys and the Unknown Fissure are shared
   by the characters of a server.
8. **The meter's "…".** "Yup" (only meter actions), then "Let's keep the ...
   with just not a boss, this is a boss and the other option just in case so
   that if there is an update users don't have to wait for an update for it
   to work directly" → "Treat … as a boss", "Not a boss", "Automatic";
   History, To-dos, Settings and PvP move to the toolbar.
9. **A multi-tool, and what is tracked when.** "some people might want to
   have the todo working without the dps-meter" and "We can keep the capture
   but if the dps meter is not open we do not handle dmg capture, etc... We
   only need to capture damage, heals, damage taken, death if the dps meter
   is open otherwise we do not need it. Currency should always be up-to-date
   though when we have the data" → capture always runs; fights (damage,
   heals, damage taken, deaths, history) are tracked only while the meter is
   open; resources always stay up to date.
10. **Shortcuts.** "I'll let you decide" → Alt+CapsLock keeps locking the
    meter; Alt+Shift+CapsLock shows and hides the toolbar and every open
    tool; the tray icon's "Show toolbar" too.
11. **Tray icon and quitting.** The tray menu holds "Show toolbar" and
    "Quit"; closing the toolbar quits Kisk (project rule: closing the main
    window ends the process).
12. **The name.** "Kisk" (TASK-23, not renamed yet) where this round shows
    the app's name: the prompt, the toolbar's quit tooltip, the tray
    tooltip and the new Settings descriptions.

### Agreed (2026-10-08, "yup perfect": accepted as drawn)

1. **Toolbar look.** An overlay card at your opacity, no shadow, 4 px
   padding, 12 px radius; 26 × 26 px tools with 14 px icons (the meter's are
   22 and 13); lit = the hot fill and a 4 px dot under the icon; a 1 px rule
   between groups; counts as "245/840 +380" (icon, figure, faint cap, muted
   extra) on a 26 px button; full = amber fill and figure. Order: grip,
   meter, PvP, to-dos, history, reminders, settings | Odyle, Shugo keys,
   warning | fold, quit. Counts: icons only (the "with names" selector was
   not chosen).
2. **Never takes the keyboard.** The toolbar, its tooltips and dropdowns are
   no-activate like the reminder card, and never click-through (Alt+CapsLock
   locks the meter only). Dropdowns are drawn inside its window, which grows
   downward while one is open (upward when the toolbar sits low on the
   screen). A dropdown closes on a click elsewhere in the toolbar, a second
   click on its button, or the pointer leaving.
3. **Pointer.** Tools and counts light on pointer enter and go on leave,
   never on moves inside; tooltips after 500 ms. The meter's tooltip says
   "Fights are tracked while it is open." (open: "Click to close. Fights stop
   being tracked.").
4. **The warning's dropdown.** "Full, a refill is coming", then each one
   with its figure and refill time right-aligned in columns ("Invasion keys
   7/7 · refills 09:00"), a divider, "All resources". A click on a row or on
   "All resources" opens the Resources window.
5. **Before any game value**: a faint dash in place of the figure; the
   tooltip says "No value yet. Log in while Kisk runs."
6. **Fold.** « at the end folds the bar to a handle: grip, unfold (») and,
   when anything is full (Odyle and Shugo keys included), the warning with
   that number.
7. **Quit.** × after fold quits at once (tooltip "Quit Kisk / Closes every
   tool"), no confirmation.
8. **Place.** The toolbar opens where you left it (top centre the first
   time), dragged by the grip or any empty part; tools keep their own places.
9. **The reminder card while the meter is closed**: 6 px under the toolbar,
   left edges aligned (above it when the toolbar is low); above the meter
   while the meter is open, as approved.
10. **Resources window rows**: the to-do window's Recharges rows, same order,
    same display; weekly entries (Unknown Fissure, Sanctuary…) stay to-dos.
    Same data as the to-dos: typing over a value, the pencil, tooltips and
    "from yesterday's login" work as proposed in the to-do round, and a
    change shows in both windows.
11. **Resources window frame**: the to-do window's 370 px, Lucide package and
    "Resources", the "2 full" chip, character selector, source line; no ×
    slot on rows (nothing to hide); footer: the daily reset and which caps
    are in force ("Membership caps"). No-activate except while a field takes
    the keyboard; one at a time; the toolbar's counts read lit while it is
    open, and a click on a count closes it again.
12. **The to-do window** opens from the toolbar's checklist; the meter's "…"
    no longer has it. Settings → To-dos' lead becomes "The to-do window,
    opened from the toolbar." (strings `settings-todos-lead` en and fr).
13. **The meter**: "…" keeps its place among the tools, labelled "Boss
    setting"; × closes the meter only (the toolbar stays, its icon goes
    dark), with the tooltip "Close the meter / Fights are tracked only while
    it is open."
14. **The prompt is a card above the meter**, placed as the reminder card
    (6 px above, the meter's width, 220 px over the capsule, below the meter
    when there is no room), no-activate, clickable even while the meter is
    locked. Copy: "Open the meter at launch?", "Kisk starts with the toolbar
    alone. Open the DPS meter each time Kisk starts?", Lucide settings then
    "Settings › Overlay › Open the meter at launch" with Lucide chevron-right
    between the steps; buttons "Not now" (quiet) and "Yes, at launch"
    (tonal).
15. **Prompt answers**: Yes turns the setting on and shows a good note with
    the path for 4 s, then fades out; Not now closes at once. Either answer
    is final. No answer: the card stays while the meter is open; closing the
    meter without answering asks again next time. A reminder that comes
    meanwhile stacks 6 px above it.
16. **Settings → Overlay → Window**: "Open the meter at launch" first ("When
    Kisk starts. Off: only the toolbar opens, and the meter opens from its
    icon."); "Lock overlay" adds "The toolbar always takes clicks."; a new
    row "Show or hide everything" with Alt + Shift + CapsLock as read-only
    Kbd keys ("Hides the toolbar and every open tool, and brings them back.
    The tray icon's Show toolbar brings them back too."). Settings has no
    shortcuts page in the build, so each shortcut is described on its row.
17. **Tray**: tooltip "Kisk"; "Show toolbar" is the default item (bold), and
    a left click on the icon does the same (it also unfolds and brings the
    toolbar in front); no Windows notification.
18. **Draft French** for the new copy (toolbar tooltips and dropdown, the
    prompt) checks the room; the real text goes through i18n.

### Agreed (2026-10-08, were open)

1. **The Reminders bell** has no window of its own: a small dropdown with
   the next three reminders switched on, "Pause reminders" (the bell turns
   to Lucide bell-off while paused, the Settings master switch) and
   "Reminder settings" (Settings → Reminders). Not chosen: the bell opening
   Settings → Reminders.
2. **The PvP icon** is Lucide crosshair (the crossed swords are the DPS
   meter's now), and the PvP window's header changes to match.
3. From the to-do round (see
   [../todos-autofill/README.md](../todos-autofill/README.md)): no switch to
   stop filling from the game; no arrows on game rows; weekly rows read
   "done / target".

### Open data (to confirm in game)

Built with the drawn default and settled later by the user's in-game
comparison (dashed on the approved pages):

1. Deus Research Base / Shattered Arkanis cap: 4 (the ticket table) or 14
   (our earlier list). Built: 4.
2. Which ticket a Conquest entry spends: its dungeon's own (six rows, 14, +2
   a day) or a shared pool. Built: per-dungeon tickets.
3. What the game shows for keys above 21 (premium items). Built: as the
   to-do round's item 8 (a typed number is your total).
4. Which of tickets 102/103 is Balaur Fortress or Fafnium Laboratory, the
   exact reset hour, and Ordeal's ticket (matched by name only).

## Not decided by the mockup

- The app's other names (title bars, About, the waiting card's "DPS Meter")
  change with TASK-23.
- Icons to add from Lucide (ISC) when built: `crosshair`, `grip-vertical`,
  `key-round`, `zap`, `bell-off`, `chevrons-left`, `chevrons-right`,
  `package`, `settings`; `swords`, `bell`, `crown`, `list-checks`,
  `triangle-alert`, `chevron-right` are in `crates/app/assets/icons/`
  already.

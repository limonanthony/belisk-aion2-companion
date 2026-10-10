# Soul tracker card layout: mockups

**Approved on 2026-10-10: A, with the reminder card following, plus one
addition from the user: 8 px between the pet's name and its Lv line too,
so the name, the Lv line and the bar are evenly 8 px apart.** The reminder
card follows A's alignment (its events start at the bell's edge) and keeps
its fact line ("3 min to join") right under the event name, as before.

Measured on the approved pages:

- Both cards' bodies start at the header icon's left edge, 13 px from the
  card's outer edge (the `.rm-ev` inset went from 23 px to 4 px).
- Soul tracker card: name line to Lv line 8 px (was 0), Lv line to bar
  8 px (was 4). Heights 104 / 165 / 226 px for 1 / 2 / 3 pets (was
  92 / 141 / 190; 61 px per pet). The bar is 14 px from the bottom edge.
- Reminder card: heights unchanged, 81 / 120 / 159 px; the fact line has no
  gap under the name.
- **Corrected the same day**: the user's "8 px" is the visible space. The
  13 px name letters sit in the middle of their 18 px line, so an 8 px box
  margin showed about 12 px. The name → Lv line box margin is now 4 px, so
  the visible gaps are 8 px (name letters → badge) and 8 px (badge → bar);
  heights 100 / 157 / 214 px for 1 / 2 / 3 pets (57 px per pet). The values
  above (8 px margin, 104 / 165 / 226 px) are superseded.
- 14 px from the outer edge to the content on all four sides is kept on
  both cards, as are the timer ring and the level badge colours.

Where each part went in `../approved/`:

- `hub.css`: `.rm-ev` starts at the icon's edge (4 px). Both cards use it,
  so the reminder card under the toolbar
  ([toolbar.html](../approved/toolbar.html?states=reminder)) and in
  [pets.html](../approved/pets.html?states=pReminderClosed,pReminderOpen)
  follow.
- [reminders.html](../approved/reminders.html): its own copy of `.rm-ev`
  at 4 px; title and notes.
- `pets.css`: `.pc-fact` 8 px under the name line, `.pc-bar` 8 px under
  the Lv line; the card's comment. [pets.html](../approved/pets.html): the
  "Look" note. `pets.js` and `hub.js` are unchanged (layout only).
- [tokens.md](../approved/tokens.md): "Reminder card" › Event block;
  "Soul tracker card" › Pet block, Bar, Height.

Build from there. This folder is the record of how it was chosen and is not
updated any more: B stays here only. Since the approval the approved cards
draw the new layout themselves; `layout.css` restores the old values for
the "Today" cards so this page still shows the comparison as it was.

*Before the approval:* one mockup round, 2026-10-10. The user tried the
soul tracker card in game ("It works perfectly now") and asked, about the card
showing "Sparkie / Lv 2 34/75 → Lv 3 / the bar": "how hard would it be to
move sparkie at the same level as the icon + put bigger bottom margin
between the lv 2 and the bar ? could you try that in a mockup side by side
with the current visual ?"

Open [index.html](index.html) in a browser, no server needed. Each tile
shows today's approved card, A and B side by side at the same scale. The
tiles cover one, two and three pets, a level reached (chip), Lv 3 · 75/75,
a long name, locked, the card stacked with the reminder card, and the
reminder card on its own. Each tile is drawn over the bright, busy and dark
scenes; the page controls set the overlay opacity and scale.

The page uses the approved files as they are (`../approved/shared.css`,
`overlay.css`, `todos.css`, `hub.css`, `pets.css`, `kit.js`, `hub.js`,
`pets.js`): the cards are `Pets.card()` and the approved reminder markup,
with this round's classes from `layout.css` added. `layout.js` draws the
page.

## The options

"At the same level as the icon" can be read two ways. Both readings get the
bigger gap above the bar.

| | Today (approved) | **A. At the icon's edge (recommended)** | B. Name in the header |
|---|---|---|---|
| Pet block | 23 px in, under the "Pet souls" text | 4 px in: the name, Lv line and bar start at the paw icon's left edge (measured 13 px from the card's outer edge, as the icon) | One pet: the name replaces "Pet souls" in the header (13 px / 600, beside the paw); a level chip moves into the header before the ring; the Lv line and bar stay 23 px in. Two or three pets: "Pet souls" stays and the pets are drawn as in A |
| Lv line to bar | 4 px | 8 px | 8 px |
| Height, 1 / 2 / 3 pets | 92 / 141 / 190 px | 96 / 149 / 202 px | 78 / 149 / 202 px |
| Gains | — | Exactly the request, one rule for any number of pets; the bar runs the card's full inner width | The smallest single-pet card (−14 px) |
| Costs | The two bars look close together | +4 px per pet; the body no longer lines up under the title | Two layouts depending on the pet count, so the card changes shape when a second pet arrives; the header loses the card's identity ("Pet souls"); a long name shares the row with the chip, ring and × |

All the other approved values stay: 14 px from the outer edge to the
content on every side (the bottom measured 14 px under the bar in all
three), the timer ring, the level badge colours.

How hard: A changes two values (`padding-left` of the pet block, the bar's
`margin-top`). B also needs a second header layout for the single-pet case
and has to switch layouts when a second pet arrives.

## The reminder card

The reminder card has the same indented body under its header. The last
two tiles draw it today and with A's indent (the reminder has no bar, so
only the indent changes; heights stay 81 / 120 px). When the two cards
stack under the toolbar and only the pet card changes, their bodies start
at different edges. If the reminder card follows, they line up again.

## Recommendation

**A, with the reminder card following.** It is the literal reading of the
request, keeps one layout for any number of pets, and doubles the gap above
the bar so the Lv line and the bar no longer read as one block. Changing
the reminder card too keeps the two stacked cards aligned.

## Decisions for the user

1. A (recommended), B, or keep today's card.
2. The gap between the Lv line and the bar: 8 px (drawn), or another value.
3. Whether the reminder card follows (its event block at the bell's edge).

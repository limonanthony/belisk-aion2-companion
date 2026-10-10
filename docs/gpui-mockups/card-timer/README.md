# The cards' timer: mockups

**Approved on 2026-10-10.** The user: "I'm all good with your recommended,
the alternatives were great too but in the end I think what I proposed is
better (I really liked the whole card being the timer but it's too
distracting imo)". Agreed: option A, the 12 px hollow ring left of the ×
(2 px stroke, track `gauge-empty`, arc `muted`, the arc turning `text`
while paused; locked, the ring takes the ×'s slot; the 2 px time-left line
removed), on both cards: the reminder card (30 s) and the soul tracker card
(10 s). The approved design now lives in `../approved/`:

- the ring's styles in `hub.css` (`.rm-timer`, `.rm-ring`, the `rm-ring`
  run, and the reminder card's `.rm-x`), its drawing in `hub.js`
  (`Hub.ring()`, also used by `Hub.reminder()`, so the reminder card under
  the toolbar in [toolbar.html](../approved/toolbar.html?states=reminder)
  and [pets.html](../approved/pets.html?states=pReminderClosed,pReminderOpen)
  has it);
- the [reminder card](../approved/reminders.html) (its own copy of the
  styles and drawing; states pointer over, last seconds, locked and live
  show the ring);
- the [soul tracker card](../approved/pets.html?states=cGain,cHover,cLast,cRestart,cLocked,try)
  (`pets.js`, `pets.css`: the ring runs 10 s; new states "the last seconds"
  and "a new gain restarts it");
- [tokens.md](../approved/tokens.md): "Shape" (Timer ring), "Motion",
  "Reminder card" (Header, Height, Timer ring, Ring paused, Ring locked,
  Ring last seconds, Lifetime, Locked), "Soul tracker card" (Header, Timer
  ring, Height, Lifetime) and "GPUI notes". The Time-left line rows are
  gone.

Build from there. This folder is the record of how it was chosen and is not
updated any more: options B to E and the pet-card-only variant stay here.
Since the approval the approved cards draw the ring themselves; `timer.js`
strips it, and `timer.css` keeps a copy of the old line, so this page still
shows each option as it was compared.

The table below says the ring sits "2 px before the ×"; as drawn (and
approved) it is 8 px, the header's 6 px gap plus those 2 px, and locked it
sits centred where the × was. `tokens.md` has the measured values.

*Before the approval:* one mockup round, 2026-10-10, after the user tried the
soul tracker card in game ("very clean") and wrote: "the bar timer we have
below the cards for notifications and this were fine but now it shows 2
bars, do you think a small circle (with the empty middle) on the left of the
close button would be better ? … show me an example of how you interpret
what I want for the new timer (and do not hesitate to try other ideas you
have !)".

The problem: on the soul tracker card each pet has a 4 px progress bar,
and the card's 2 px time-left line sits 8 px under the last one. That makes
two bars of the same kind, one about the pet and one about the card. With
three pets there are four bars. The reminder card has no other bar, so its
line reads clearly as time.

Open [index.html](index.html) in a browser, no server needed (it also works
served from the repository root, `/docs/gpui-mockups/card-timer/`).

| Section | What it shows |
|---|---|
| 1. Today | The pet card (one and three pets) and the reminder card (one and three events) with the time-left line, and both under the toolbar |
| 2. A. The user's idea | The ring left of the ×, on both cards: running, three pets and three events, paused under the pointer, locked, the last seconds, a gain before and after, under the toolbar, and the variant with the ring on the pet card only |
| 3. A over the game | Both cards over the dark, bright and busy scenes at opacities 60, 150 and 255 |
| 4. Close-up at 200% | The header at twice the size: A at 12 px and 14 px, A paused, B, C, E |
| 5. Other ideas | B, C, D, E, each running (pet card), paused (reminder card) and locked (pet card) |
| 6. Try it | Both cards running their real timers, 10 s and 30 s: the pointer over a card pauses it, a click dismisses it, buttons play soul gains, a reminder and the lock; a selector switches between every timer, today's line included |

Built on the approved design, which is not edited. The page loads
`../approved/shared.css`, `overlay.css`, `todos.css` (the bundled Barlow Semi
Condensed), `hub.css` (the reminder card's classes), `pets.css`, `kit.js`,
`app-name.js`, `hub.js` and `pets.js`. The pet card is the approved
`Pets.card()` with its time-left line swapped for the timer under study;
the reminder card is the approved markup from `reminders.html`; the toolbar
is `Hub.toolbar()`.

| File | What it is |
|---|---|
| `timer.js` | The timers (ring, outline, seconds), both cards dressed with each one, the page's sections and the working tile |
| `timer.css` | Only what this round adds (new values marked "new"); the reminder card's × copied from `../approved/reminders.html`; page chrome |

## The options

Every option keeps today's rules. The lifetimes stay 10 s after the pet
card's last update (any gain starts it again) and 30 s for a reminder. The
pointer over a card pauses it and lights the ×. A click or × dismisses the
card. Locked, the card is click-through with no × and runs out by itself.
Nothing changes colour at the end, nothing flashes, and the card fades out
over 150 ms. Every option except "keep the line" removes the time-left line,
which also makes each card 10 px shorter.

| | What it is | Gains | Costs |
|---|---|---|---|
| **A. A ring left of the × (the user's idea, recommended)** | 12 px hollow ring, 2 px stroke (8 px hole), in a 16 px slot 2 px before the ×. Track `gauge-empty` `rgba(255, 240, 225, 0.12)`, arc `muted` `rgba(255, 246, 236, 0.66)`. It empties clockwise from 12 o'clock, like a clock hand. Paused, the arc turns `text`. Locked, with no ×, the ring takes the ×'s 22 px slot so its centre does not move | One bar per pet again. The timer sits where the eye goes to close the card. Same shape on both cards. Small, it does not compete with the content | An arc is read less exactly than a line, but nobody needs the exact time. The ring is new to the visual language. GPUI has no ready-made arc: it needs a path built from line segments, or a small custom element (canvas path) |
| B. A ring around the × | The × becomes round, inside a 22 px ring at 1.5 px | The timer says what it closes; no extra slot | Busy at 22 px, and the ring and the pointer's fill sit on the same circle. Locked, the ring stands alone where the × was |
| C. A ring around the header's icon | The paw or the bell inside an 18 px ring; the icon shrinks to 9 px | The × is untouched; the timer sits on the card's identity | The icon gets small. The ring is far from the × that ends the card early. On the reminder card the bell already means "time" |
| D. The card's outline draining | The 1 px card border becomes the track; a 1.5 px cream arc runs round the card and empties clockwise from the top-left corner | No new element; works at any card height | Reads as decoration, or as a selected card. It changes the approved border (the only card outline), and is the hardest to draw in GPUI (a path along a rounded rectangle) |
| E. Seconds left | "8 s" 11.5 px `faint` before the ×, counting down | Exact and plain | A number ticking every second draws the eye; it is text, not a glance cue. On the reminder card, "28 s" sits next to "In 1 min · 22:00", two different times |
| Keep the line | Today's | Nothing to build | The two bars stay |

Variant of A: **the ring on the pet card only**, with the reminder card
keeping its line (section 2, last tile; selector value "A on the pet card,
the line on the reminder"). The reminder card has no other bar, so its line
is not confusing. The cost is two timer languages for two cards that share
the same place, rules and look.

## Recommendation

**A, on both cards, at 12 px.** It is the user's idea, it removes the
second bar where the confusion is, and it sits next to the × the user
already reaches for. Using it on both cards keeps one timer language for
two cards that stack together under the toolbar (section 2, "Under the
toolbar").

Over the game (section 3) the 12 px ring reads at opacity 150 and 255 in
every scene. At opacity 60 over the bright scene it is faint, but no
fainter than the card's own text. 14 px (section 4) starts to match the
× icon's weight and competes with it.

When paused, the arc turning `text` beside the lit × gives two signs that
the pointer holds the card. Leaving the arc as it is would be quieter;
both are fine.

## Decisions for the user

1. Which timer: A (recommended), B, C, D, E, or keep the line.
2. Both cards (recommended), or the ring on the pet card only.
3. The ring's size: 12 px (recommended) or 14 px.
4. While paused: the arc turns `text` (drawn), or stays `muted`.

## Not decided by the mockups

- The drawing in GPUI: an arc as a path (`PathBuilder`, line segments along
  the circle) or a custom element. It redraws only while the card is up,
  about one frame per degree (10 s: 36 frames a second at most; 30 s: 12).
  The approved line needed about one frame per pixel.
- After approval, the time-left line rows in `../approved/tokens.md`
  ("Reminder card", "Soul tracker card") and both pages change together.

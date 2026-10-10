# Pets window and soul tracker: mockups

**Approved on 2026-10-10.** The user approved every recommendation of both
rounds: window layout A (meter rows), card design A (the reminder card's
twin), the name "Pets" (French "Familiers"), level colours variant (a) (the
badge only), and Settings › Pets as drawn, with the "197 of 290" count and
the test card's rule. The approved design now lives in `../approved/`: the
[Pets window and the soul tracker card](../approved/pets.html) (every
window and card state, the card's places, a working one; `pets.js`,
`pets.css`), the Pets tool in the
[toolbar](../approved/toolbar.html?states=pets,petsOpen) (drawn by `hub.js`),
[Settings › Pets](../approved/settings.html?states=pets,pets:off,pets:test,pets:none)
(the sidebar item in `app.js`), and the "Pets" values and rules in
[tokens.md](../approved/tokens.md) (with "Settings → Pets" under Main
window). Build from there. This folder is the record of how it was chosen
(TASK-34.3, part of TASK-34) and is not updated any more: layouts B and C,
cards B and C, level colours (b) and the dropped items (the toolbar's
pinned-pets count, typed-in counts, Special pets left out of All, with the
menu's "Special pets in All" tick) stay here only. Since the approval the
approved toolbar and Settings shell draw the Pets tool and sidebar item
themselves; this folder's pages replace or skip them so they still show
what was drawn.

Open [index.html](index.html) in a browser, no server needed (it also works
served from the repository root, e.g. `python3 -m http.server`, then
`/docs/gpui-mockups/pets/index.html`). [settings.html](settings.html) is
Settings › Pets on its own; index.html embeds its states.

| Section of index.html | What it shows |
|---|---|
| 1. Level colours in the window | Round 2: (a) the badge only, (b) the badge and the fill, and round 1's look, on Fera (92 pets, 25 at Lv 3) |
| 2. Level colours over the game | (a) and (b) side by side over the dark, bright and busy scenes at opacities 60, 150 and 255 (fixed; the selectors do not change this section) |
| 3. Level colours in the card | Card A in (a) and (b): a first unlock, a gain, a pet at Lv 3, a level reached beside another pet; dark and bright, 150 and 60 |
| 4. The window's states | 13 states in layout A, in the level colours picked in the selector ((a) by default) |
| 5. The card's states | 11 states in card A, same selector, with the new test card |
| 6. Where the card shows | Above the meter, under the toolbar with the meter closed, with a reminder card, toolbar low |
| 7. Opening the window from the toolbar | The pets tool and the window open |
| 8. Settings › Pets | settings.html embedded: defaults, Soul tracker off, test card shown, collection not read yet; English or French |
| 9. Try it | A working toolbar, window and card in the selected level colours |

The selectors at the top: **Level colours** ((a), (b), round 1's none),
the window's name, English and French, the real overlay settings
(background opacity, content scale) and the game scene; for reference, the
window layout and card design (round 1's B and C). They are kept in the
address (`?lvc=b&lang=fr`). **`index.html?compare`** brings back round 1's
comparison: layouts A, B and C, cards A, B and C, and the dropped items
(the pinned-pets count, typed-in counts, Special pets left out of All).
B and C do not draw level colours (B has no badge, C has pips).

Built on the approved design, which is not edited: index.html loads
`../approved/shared.css`, `overlay.css`, `todos.css` (the bundled Barlow Semi
Condensed and the to-do window's rows and source line), `meter.css`,
`hub.css` (the toolbar and the reminder card), `kit.js`, `app-name.js` and
`hub.js`; settings.html loads `shared.css`, `overlay.css`, `app.css`,
`app.js` and `app-name.js`. The toolbar is the approved `Hub.toolbar()`
drawing with the pets tool inserted after To-dos (as the updates round
inserted its slot); the Settings shell is the approved `App` shell with
Pets inserted after To-dos (the shell's section list lives in `app.js`,
which is not edited).

| File | What it is |
|---|---|
| `pets.js` | The invented pet list, the level rule, the strings (en, fr), the window, the card, the toolbar insert, the page's controls and sections, the working tile; Lucide icons not in `kit.js` / `hub.js` (paths from gpui-kit-assets 0.6.6) |
| `pets.css` | Only what this round adds; values from `../approved/tokens.md`, new ones marked "new"; the level colours at the end; page-only chrome |
| `settings.html` | Settings › Pets in the approved Settings shell and widgets, English and French; a few rules copied from `../approved/settings.html`'s `<style>` |

**Invented data.** 290 pets: Cogni 58, Fera 92, Natura 64, Varian 52,
Special 24. Seven names come from public sources (Fossa, Kalgolem,
Splendent Armadon, Hill Coradon, Forest Spirit, Slime, Krall Laborer); the
rest are public creature words combined to fill the rows, and which family
each sits in is a guess. Levels and souls are random but fixed. French pet
names are drafts for five pets only. The pet catalogue (TASK-34.2) supplies
the real names, families and order. Clock: Monday 21:48, logged in at
19:02 on Ossia.

## Round 2 (answered 2026-10-10: (a), the values and the Settings wording as drawn)

1. **Level colours: (a) or (b).** The user's idea: Lv 0 grey, Lv 1 to
   Lv 3 shades of green, strongest at Lv 3, so higher levels read at a
   glance in a long list; "Lv N" stays, so colour is never the only cue.
   - **(a) The badge only (recommended).** The level badge takes the
     level's colour; the bar's fill stays the neutral cream and means only
     progress toward the next level.
   - **(b) The badge and the fill.** The progress fill takes the level's
     green as well.
   - Why (a): the badges form one column at the row's left edge, so levels
     are scanned down that column. In (b) every Lv 3 row is a full green
     bar: on Fera (25 of 92 at Lv 3) the list becomes a green wall in which
     the pets still to farm, the short low bars, are the hardest rows to
     see; and a green fill would say "level" by its colour while its length
     says "progress". (b) is drawn in sections 1 to 3 so this can be
     judged.
2. **The colour values** (all from the approved `good` token, `#86e3a8`;
   no new hue):

   | Level | Badge fill | Badge text | Fill in (b) |
   |---|---|---|---|
   | Lv 0 | `rgba(255, 240, 225, 0.09)` over `track` (grey, the rank badge's neutral) | `muted` `rgba(255, 246, 236, 0.66)` | cream `0.04 → 0.16` (unchanged) |
   | Lv 1 | `good` at 0.10 over `track` | `good` at 0.66 | `good` `0.04 → 0.14` |
   | Lv 2 | `good` at 0.28 over `track` | `good` | `good` `0.06 → 0.24` |
   | Lv 3 | `good` at 0.86 | ink `#17120d` | `good` `0.08 → 0.34` |

   `track` is `rgba(0, 0, 0, 0.34)`: Lv 0 to Lv 2 lay their tint over the
   track's dark plate, as the names sit on the track. Without it, "Lv 1"
   and "Lv 2" washed out over the bright scene at opacity 60 (seen in the
   first pass); with it, every badge reads there. Lv 3 is a filled badge
   with ink text, as "you" is on the meter, so it reads over any scene.
   In GPUI each badge's fill is one precomputed colour (the tint over
   `track`) or a `track` div holding the tint.
   In the card, the fact line's "Lv 2" becomes the same badge (16 px tall,
   4 px radius, 6 px sides, 11 px / 600) and the "·" after it goes; (b)
   also tints the card's 4 px bar (Lv 1 `good` 0.10 → 0.36, Lv 2 0.14 →
   0.50, Lv 3 0.20 → 0.66).
   Lv 1 and Lv 2 differ less from each other than from Lv 0 and Lv 3; the
   step that matters most in a long list is "maxed or not".
3. **Settings › Pets wording** (contents agreed, wording proposed):

   | Row | English | French (draft) |
   |---|---|---|
   | Page | Pets: "The Pets window opens from the toolbar. The card works with the meter open or closed." | Familiers : « La fenêtre Familiers s'ouvre depuis la barre d'outils. La carte fonctionne compteur ouvert ou fermé. » |
   | Group | Soul tracker card | Carte de suivi des âmes |
   | Soul tracker (on) | "A small card when you pick up a pet's souls, with its progress to the next level. It goes 10 s after the last pickup." | Suivi des âmes : « Une petite carte quand vous ramassez des âmes d'un familier, avec sa progression. Elle disparaît 10 s après le dernier ramassage. » |
   | Only pinned pets (off) | "Shows the card only for the pets you pinned in the Pets window." | Familiers épinglés seulement : « N'affiche la carte que pour les familiers épinglés dans la fenêtre Familiers. » |
   | Pets already at Lv 3 (on) | "Also shows the card for pets at the cap, as Lv 3 · 75/75." | Familiers déjà au Lv 3 : « Affiche aussi la carte pour les familiers au maximum, en Lv 3 · 75/75. » |
   | Black-market purchases (on) | "Souls bought at the black market (+5) show the card like a pickup." | Achats au marché noir : « Les âmes achetées au marché noir (+5) affichent la carte comme un ramassage. » |
   | Test card | "Shows the card now with your first pinned pet." Button "Show a test card"; then the good badge "Shown under the toolbar" for 4 s | Carte de test, « Afficher une carte de test », « Affichée sous la barre d'outils » |
   | Group | Collection | Collection |
   | Pets on Ossia | "Read at login, Mon 19:02. The game sends your pets only when you log in; pickups since are added." and "197 of 290" read-only | Familiers sur Ossia : « Lue à la connexion, lun. 19:02. Le jeu n'envoie vos familiers qu'à la connexion ; les ramassages depuis sont ajoutés. » |
   | Before the first read | "Not read yet. Log in with a character on this server while Belisk runs." and "—" | « Pas encore lue. Connectez-vous avec un personnage de ce serveur pendant que Belisk tourne. » |

   Also proposed there: the paw-print icon in the sidebar (the toolbar
   tool's); Soul tracker off dims the rows under it and the test button
   (labels at 50%, controls at 40%) and keeps their values, as Reminders;
   the test card shows the first pinned pet's real progress (the first pet
   in game order when nothing is pinned), header "Test card", nothing
   added, whatever the switches say; "Shown above the meter" when the
   meter is open. In French the page runs about 33 px past the window and
   scrolls with the thumb, as Reminders does.

## Decided

### Agreed (user, 2026-10-10, round 1)

1. **Window layout A**, meter rows: a level badge, a track whose neutral
   fill shows souls toward the next level, the name, "27/75 → Lv 3"; Lv 3
   reads "75/75 Max"; pinned pets under their own label.
2. **Card design A**, the reminder card's twin: header "Pet souls" with the
   paw-print icon and ×, per pet the name, "Lv 2 · 28/75 → Lv 3" and a 4 px
   bar; a good chip "Lv 2 reached" or "Unlocked"; the 2 px time-left line
   draining over 10 s.
3. **Name**: "Pets" (French draft "Familiers").
4. **Kept**: the family summary (owned, at Lv 3, and the levels bar);
   the Show and Order menu (Show all / not owned yet / not at Lv 3; Order:
   game order / closest to the next level; Special pets in All); Special
   pets marked "no stats"; the "as of" source line; the card for
   black-market +5 purchases.
5. **Dropped**: the toolbar's pinned-pets count; typed-in counts.
6. **The designer's proposals, accepted**: the pets tool (Lucide
   paw-print) after To-dos; the game's order by default; pins (the pin
   shows while the pointer is in the row, stays on pinned pets; pinned on
   top of All and of their family's tab; per server); one card for up to
   three pets in arrival order, any gain restarting the 10 s; the pet card
   stacks 6 px beyond a reminder card; keyboard and focus (the window takes
   the keyboard only while the search field has it; Esc clears, a second
   Esc hands it back); motion as the reminder card.

### Agreed earlier (user, 2026-10-10, before round 1)

- A window listing every pet, tabs All, Cogni, Fera, Natura, Varian,
  Special, and a search; pins on top; level and souls toward the next
  level; no pictures.
- The card appears on soul gains, shows progress to the next level and
  disappears 10 s after its last update; "Lv 3 · 75/75" at the cap; no
  insight-crystal figure.
- The rule: 5, 25, 75 souls per level; Lv 3 is the cap; carry-over.
- English and French; French keeps "Lv".
- The card's places: 6 px above the meter while it is open, 6 px under the
  toolbar while it is closed, above the toolbar when it sits low.

### Agreed (user, 2026-10-10, Settings › Pets contents)

A page between To-dos and General. Group "Soul tracker card": the master
switch (on; when off the others are disabled), "Only pinned pets" (off),
"Pets already at Lv 3" (on), "Black-market purchases" (on), "Show a test
card". Group "Collection": a read-only "Read at login on <server>, <date
time>", or how to get it read. Not in Settings: the window's sort and
filters (its menu), the 10 s, the card's place, a sound, forgetting the
collection.

### Settled

- "Research bar" in the user's request meant the **search bar** (French
  "barre de recherche"), which the window has. The family summary's levels
  bar stays as approved in round 1.

## Open (to settle in game or with the catalogue)

1. **The game's order** of pets inside a family (drawn: by pet id).
2. **French names**: the families ("Spécial" drafted; the others kept) and
   each pet's name (the catalogue).
3. **Maxed pets' souls** arrive without a pet id (`11 90`), so the card's
   "Lv 3 · 75/75" for them needs the catalogue's mob → pet map (TASK-34.2).

## Not decided by the mockups

- Icons to add from Lucide (ISC) when built: `paw-print`, `pin`, `search`,
  `list-filter`, `arrow-right`, `chevrons-up`, `sparkles` (all in
  gpui-kit-assets 0.6.6); `info`, `check`, `x`, `play`, `gamepad-2`,
  `chevron-down`, `chevron-right`, `arrow-down` are used as approved.
- Where pins, the collection and the Settings › Pets values are stored
  (TASK-34.1's per-server store; settings.json).

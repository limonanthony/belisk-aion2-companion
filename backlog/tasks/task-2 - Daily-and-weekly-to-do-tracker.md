---
id: TASK-2
title: Daily and weekly to-do tracker
status: Done
assignee: []
created_date: '2026-10-05 14:23'
updated_date: '2026-10-06 15:35'
labels:
  - feature
dependencies: []
type: feature
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user wants to track daily and weekly to-dos (user: "daily/weekly todos (dungeons, daily quests, etc...)").

## Agreed

User (2026-10-05): "let's start working on the todos (daily/weekly) check online what each players need to do daily/weekly (things that reset and will be wasted if not done) please then we'll put them in. We'll add another window for the todos do some mockups for them too please"

- The list covers things that reset daily or weekly and are wasted if not done.
- The to-do list lives in its own window.
- Design mockups come before building.

User decisions (2026-10-06):

- All counts are manual. Packet research found that the meter can't read any count from the game: Odyle Energy, Nightmare, Shugo keys, and dungeon counts were checked against the game's traffic. User: "make the counts manual but empty by default ? So if a user wants to enter the amount of odyl he has, etc... then he can otherwise it's just ignored and it's simple todos like: 5/5 daily quests done, X/14 Daily Dungeon done (they're called daily dungeon but they're weekly for some reason), etc... And the user just checks for simple done/not done and for X amount of things do to (like daily dungeons) we can put an arrow up/arrow down to increase / decrease count"
- Three kinds of rows:
  - Done / not done.
  - Counts X/N with up and down arrows. A click on the count adds 1; the row is done at N.
  - Optional amounts for recharges (Odyle Energy, keys, Nightmare, dungeon recharges). They stay empty and ignored unless the player types a number; then the window estimates refills and warns when the amount is full.
- Done items are struck through and muted. There are no checkboxes; a check button appears under the pointer (user: "I like the alternative with just the v to check").
- No edit mode. A built-in to-do is hidden with a × under the pointer, and "N hidden · Show" restores them per section. Players can add their own to-dos (tick or count, daily or weekly).
- Settings → To-dos:
  - Daily reset hour, default 09:00 (not confirmed).
  - Weekly reset on Wednesday.
  - Membership switch (Odyle Energy cap 840).
  - "Restore default to-dos" brings the built-in to-dos back and keeps own to-dos and progress.
  - "Reset to-dos completely" asks for confirmation, then removes own to-dos and clears counts and amounts.
  - Both apply to all characters. User: "in the settings we should be able to bring back the default ones (either one reset or 2 possible reset, bring back the default todos and keep the existing or reset completely)"
- One list per character, following the character being played. Duty Missions are shared per account and server.
- The window opens from the meter's "…" menu.
- Approved design (user: "All good on my end"): `docs/gpui-mockups/approved/todos.html` and the To-dos section of `docs/gpui-mockups/approved/settings.html`.

## Not in this build

- "+1?" suggestions from boss kills (later).

## Open questions

- What is the real daily reset hour? 09:00 is reported (aion2-meta.com/timers, Berlin time; not confirmed), and a 17:00 candidate was seen in the game's traffic. The weekly reset is on Wednesday (MetaBot, from client data).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The to-do window opens from the "…" menu and lists the built-in daily and weekly to-dos for the character being played.
- [x] #2 Ticks, counts (arrows and click) and amounts work as designed, and progress survives restarting the meter.
- [x] #3 Daily to-dos reset at the set daily hour and weekly ones on Wednesday at that hour; amounts add their refill and show when they are full.
- [x] #4 A built-in to-do can be hidden and brought back; your own to-dos can be added and removed.
- [x] #5 Settings → To-dos changes the reset hour and membership, restores the defaults, and resets everything after a confirmation.
- [x] #6 Clicking the window never takes focus from the game, except while typing (adding a to-do or an amount).
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Work started 2026-10-05.

## Research summary (2026-10-05)

Research candidates only, not agreed scope.

Sources:
- MetaBot weekly reset: https://metabot.gg/en/aion-2/weekly-reset
- MetaBot dungeon guide: https://metabot.gg/en/aion-2/guides/dungeon-guide
- Game8 duty missions: https://game8.co/games/Aion-2/archives/628705
- aion2-meta timers: https://aion2-meta.com/timers/

Daily candidates:
- Duty Missions: 5 per day.
- Capped recharges:
  - Conquest dungeons: +2/day, cap 14.
  - Nightmare: +2/day, cap 14.
  - Transcendence dailies: +2/day, cap 4.
  - Abandoned Balaur Fortress / Fafnium Laboratory: +1/day, cap 7.
  - Invasion key: +1/day, cap 7.
  - Shugo key: +4/day, cap 28 (sources conflict on the membership cap).
  - Odyle Energy: +15 every 3 h, cap 560 (840 with membership).

Weekly candidates (Wednesday reset):
- Sanctuary raids (Ludra, Corroded Decontamination Facility, Chalice of Muspel): 4 attempts each.
- Ascension Trial: 3.
- Subjugation tickets: 3.
- Unknown Fissure: 14.
- Exploration reward counts: 7 per dungeon.
- Conquest weekly counts.
- Transcendence boss kills: 28.
- Duty Commands: up to 20 per week.
- Growth dungeons: 7 h each.

2026-10-06: design approved; packet research found no readable counts; implementation starting.

2026-10-06: tested in game by the user and accepted ("the todos are fine"). Still open: the real daily reset hour (09:00 vs 17:00) and Odyle's refill timing — both settings/estimates the user can correct.
<!-- SECTION:NOTES:END -->

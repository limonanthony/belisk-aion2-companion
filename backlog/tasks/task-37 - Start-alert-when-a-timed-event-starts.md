---
id: TASK-37
title: Start alert when a timed event starts
status: Done
assignee: []
created_date: '2026-10-14 10:31'
updated_date: '2026-10-14 19:09'
labels:
  - feature
dependencies: []
priority: high
type: feature
ordinal: 101000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Event reminders (TASK-1) announce a timed event a few minutes before it starts. This adds a second kind of card, a start alert, shown at the moment the event starts, so the player knows they can join it.

## Intent

User, 2026-10-14: "when in dungeon i'm like 'nice shugo in 5 min I shouldn't restart a dungeon' but while fighting the boss I focus on the fight and forget. ... 2 types of notifications: reminders AND notifications an event is under way ... a reminder is X minutes BEFORE the event and the notification (a new option to add in the settings ...) when you can join the event. The text in the notification for a reminder would be different than for the start of the event."

## Agreed (user)

- Two kinds of card: the reminder X minutes before the event (unchanged) and a start alert when the event starts.
- The start alert is a new option in Settings, with its own card text, different from the reminder text.
- Option A, chosen with "A. is enough": the start alert uses the same 30 s card and chime as reminders.
- Rejected for now: keeping the card up for the whole join window, and holding the card until the fight ends.

## Scope

Agreed (user):
- One new Settings switch "Start alert" under Settings > Reminders > "How you are reminded".
- The start alert is the existing reminder card (same window, placement, 30 s countdown ring, pointer pause, click to dismiss, chime when Sound is on); no new window or sound.
- English and French.

Derived (Main's choices following from the agreed option A and existing reminder behaviour):
- The setting is named "Start alert" (FR "Alerte au début"); card header "Under way" (FR "En cours"), shown as "Under way · <start time>".
- Off by default; a settings file saved before this feature loads it as off.
- It applies to every event switched on for reminders; the master reminders switch off means no reminder and no start alert. The reminder before the event stays unchanged and always on while the master switch is on.
- The alert is due at the event's start instant and may still be shown up to 60 s after it; a later wake drops it. Each start shows at most one alert. Events sharing a start instant make one card. A start already inside its 60 s when the plan is made (app launch, switch turned on) shows at once.
- The switch row sits between "Remind me" and "Sound", and is dimmed and inert while the master switch is off.
- The Sound description becomes "Windows' Chimes sound, once per card." since the chime now covers both kinds.
- "Show a test reminder" is unchanged; there is no test start alert.

Not in scope: a new window, a join-window countdown, holding cards back during fights or loading screens, per-event start-alert switches, a test start alert, changes to the toolbar bell's upcoming list.

Follows TASK-1 (event reminders, Done); that task is not reopened.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 With Settings > Reminders > Start alert on, a card appears when each switched-on event starts, headed "Under way · <start time>", naming the event(s) with the same lines as the reminder.
- [x] #2 The reminder before the event still appears at the chosen lead time, with its own header ("In N min · <start time>").
- [x] #3 The start alert plays the chime when Sound is on and goes away by itself after 30 s; a click dismisses it; it never takes focus from the game.
- [x] #4 With Start alert off (the default) or the master reminders switch off, no start alert appears.
- [x] #5 The Settings switch and the card text are available in English and French.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-14: built and agent-tested, awaiting the user's in-game test. Settings › Reminders › Start alert (off by default, saved as reminders.startAlert); the card is the reminder card headed "Under way · <start>" / "En cours · <start>", shown from the start up to 60 s after it, once per start. cargo test --workspace passed (app 941 + 1 ignored); independent review accepted; captures checked in English and French (reminder-09-start-alert.png, settings-22b-reminders-start-alert.png). Version 1.2.0 with changelog entries in both languages.

2026-10-14: user tested the delivered build in game on 2026-10-14 and, asked whether everything worked (start alert at a :00/:30, soul card after a mid-game start, tool windows reopening in place, clean launch log), answered: "Everything seem[s fine]". Evidence Main observed in the user's belisk.log and data folder: the 19:55 reminder card, then the 20:00 start alert card ("card: start alert [ShugoFestival, RiftPortals] at 20:00: opened a card window"). Accepted by the user; criteria checked on that basis.
<!-- SECTION:NOTES:END -->

---
id: TASK-1
title: Event reminders for recurring timed events
status: Done
assignee: []
created_date: '2026-10-05 14:23'
updated_date: '2026-10-05 20:23'
labels:
  - feature
dependencies: []
type: feature
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user wants a reminder before recurring timed in-game events (user: "the notifications for things that happen X hours (shugo, rifts, etc...)").

## Scope (agreed 2026-10-05)

- Events covered:
  - Shugo Festival (the minigames): every hour at :00.
  - Dimensional Invasion: every hour at :30.
  - Rift portals: every 3 hours from 02:00.
- These are server times taken from metabot.gg/en/aion-2/events, which reads the game's schedule tables. That source does not confirm the server time zone (it assumes Europe = Berlin), so the reminders use the user's local clock; this is to be checked once against the game.
- The reminder is a small separate window placed above the meter, not part of the meter window.
- The reminder plays a sound and disappears after about 30 seconds.
- The sound is Windows' own chimes (chimes.wav in the Windows Media folder), played from the user's Windows install and not bundled with the meter; if that file is missing, Windows' notification sound plays instead (user: "let's take chimes").
- The reminder never takes focus from the game.
- No Windows system notification is used.
- An option in Settings turns the reminders off.
- Settings → Reminders, as in the approved design (user: "I like the mockups you can start implementation"):
  - A master "Event reminders" switch; on by default.
  - A separate Sound on/off switch; on by default.
  - Events offered, each with its own switch:
    - On by default: Shugo Festival, Dimensional Invasion, Rift portals.
    - Off by default: Watcher Kaira, Artifact Siege, Abyss field bosses (Mon/Thu/Sat 21:30), Guardian Lord Nahma (Fri/Sun 21:00), Arena of Tactics opens (11:00/19:00), reward keys refill (16:00).
  - "Remind me": 1, 2, 3 or 5 minutes before the event; default 1 minute.
  - A "Show a test reminder" button.

## Boundaries and dependencies

- Waits on the Settings page, which is being built now and comes first.
- The design was approved by the user on 2026-10-05: docs/gpui-mockups/approved/reminders.html and the Reminders section of docs/gpui-mockups/approved/settings.html.
- Which minigame is open in a given hour is not in the game data, so the Shugo Festival reminder can only announce that the festival starts.

## Open questions

- Resolved 2026-10-05: other timed events, for example Artifact Siege (Mon/Thu/Sat 21:00), are offered in Settings, off by default, as part of the approved design.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Before each Shugo Festival (:00), Dimensional Invasion (:30) and Rift portal (every 3 hours from 02:00, local clock) start, a reminder naming that event appears.
- [x] #2 The reminder appears in its own small window above the meter, not inside the meter window, plays a sound, and disappears by itself after about 30 seconds.
- [x] #3 When a reminder appears while the game has focus, the game keeps keyboard and mouse focus.
- [x] #4 No Windows system notification is produced for any reminder.
- [x] #5 With reminders turned off in Settings, no reminder window or sound appears at event times.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Research (2026-10-05)

Source: metabot.gg/en/aion-2/events (data read from the AION 2 global client, build 0.0.4387.0); all times are server time. EU server time = Berlin time (aion2atlas.com shows the EU server clock as "Europe (Berlin)"; MetaBot assumes the same and says regions are unconfirmed).

Daily:
- Shugo Festival: every hour at :00. Ready time 3 min, rounds 2 min 30 s to 5 min. Which minigames are open in a given hour is only shown in the in-game Shugo Festival menu.
- Dimensional Invasion (Beritra Air Raid, Accursed Sword, Incursion of the Spirits): every hour at :30. Ready time 3 min, round 10 min.
- Rift portals: every 3 hours from 02:00 (02, 05, 08, 11, 14, 17, 20, 23). Open for 1 hour, teleport only in the first 10 minutes (a2db.ru).
- Watcher Kaira (field boss, Lv 65, Chaotic Lower Reshanta): every 3 hours from 01:00 (01, 04, 07, 10, 13, 16, 19, 22).
- Arena of Tactics (10 vs 10, Lv 45): queue windows 11:00-14:00 and 19:00-21:00.
- Reward keys refill (Shugo Festival key, Dimensional Invasion key): 16:00.

Weekly:
- Artifact Siege (Chaotic Lower Reshanta, Lv 45): Monday, Thursday, Saturday 21:00.
- Abyss field bosses: Executor Argo, Executor Kaira, Executor Tamasa (Lv 65, Lower Reshanta); Executioner Dramos, Ravager Marakha, Turncoat Ducal (Lv 80, Middle Reshanta): Monday, Thursday, Saturday 21:30.
- Guardian Lord Nahma (Lv 65) and Enraged Guardian Lord Nahma (Lv 80): Friday and Sunday 21:00.
- Weekly reset of entry allowances: Wednesday; the hour is not in the client data (metabot.gg/en/aion-2/weekly-reset).

## Status (2026-10-05)

- Implementation started 2026-10-05 at the user's request: "start implementing the notifications for the minigames please that is in the backlog, do some research on all timed events (shugo, rifts, etc...)".
- Built and reviewed, agent-tested only: the schedule engine, the Reminders settings, and the reminder sound (Windows' chimes, falling back to Windows' notification sound).
- Design approved by the user, built and agent-tested: the reminder window (docs/gpui-mockups/approved/reminders.html) and the Settings → Reminders page (Reminders section of docs/gpui-mockups/approved/settings.html).

2026-10-05: design approved by the user ("I like the mockups you can start implementation"; approved mockups: docs/gpui-mockups/approved/reminders.html and the Reminders section of docs/gpui-mockups/approved/settings.html), reminder card and Settings → Reminders built and agent-tested; awaiting the user's in-game test.

2026-10-05: tested in game by the user and accepted ("You can commit notifications are working correctly"). Sound: Windows chimes.
<!-- SECTION:NOTES:END -->

---
id: TASK-40
title: Soul tracker cards missing when Belisk starts mid-game
status: Done
assignee: []
created_date: '2026-10-14 15:50'
updated_date: '2026-10-14 19:09'
labels:
  - bug
dependencies: []
priority: high
type: bug
ordinal: 104000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

User report, 2026-10-14: "Also the pets notifications don't show up anymore in the debug build for some reason". Soul tracker cards did not appear for soul gains.

Expected (existing rule, TASK-34.5): a soul tracker card appears on every soul gain.

## Cause found

From the user's belisk.log and a recording of the session:
- Belisk was restarted mid-game at 17:05, so the game sent neither the character's name nor the pet collection again.
- Pet changes were held until the character was known and dropped after 15 s.
- The character was only worked out at 17:12:58 (first skill hit), so the two soul gains (17:08:40, 17:09:46) and 14 counter gains were lost.
- The same rule exists in 1.1.0: this is not a regression.

## Agreed fix

User: "Use the server when there is only one". When every remembered character is on one server, pet changes are bound to that server at once. Several servers, or none, keep the old rule. Tickets are unchanged. The login snapshot still waits for the game's name.

Built and agent-tested in the core (replay of the user's recording: the lost soul gains are now emitted at their recorded times); independent review accepted. Not tested by the user.

## Related

TASK-34.5 (soul tracker card, Done), TASK-35.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 With all remembered characters on one server, starting Belisk while already playing, the first soul picked up shows a soul tracker card.
- [x] #2 With remembered characters on several servers, behaviour is unchanged.
- [x] #3 belisk.log records the server assumption, and a WARN with the number of pet changes when the game later names a character on another server.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-14: user tested the delivered build in game on 2026-10-14 and, asked whether everything worked (start alert at a :00/:30, soul card after a mid-game start, tool windows reopening in place, clean launch log), answered: "Everything seem[s fine]". Evidence Main observed in the user's belisk.log and data folder: "pets bound to server 1302 from the remembered characters" at every launch while the game never named the character, and pets.json written at 21:06 in that session. Accepted by the user; criteria checked on that basis. AC #2 (several servers, unchanged) is agent-tested only.
<!-- SECTION:NOTES:END -->

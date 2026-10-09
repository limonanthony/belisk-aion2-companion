---
id: TASK-22
title: Dimensional Invasion reminder sometimes has no invasion
status: In Progress
assignee: []
created_date: '2026-10-08 09:31'
labels:
  - bug
dependencies: []
priority: medium
type: bug
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The dimensional rifts (Dimensional Invasion, the ones that require a key like the Shugo events) sometimes show a reminder notification, but no rift appears afterwards.

**Reported** 2026-10-08: "The dimensional rifts (the ones that require a key like the shugo events): sometimes a notification pops up but NO rift shows up after. So maybe those rifts are not EVERY hour?"

## Findings

Every public source (published game schedule tables) says the invasion starts every hour at :30, as one of three invasions at a random field spot in Verteron or Altgard. The user's recordings can neither confirm nor rule out skipped hours: the game only tells a player about an invasion they are near. The Shugo Festival, checked the same way, was there every time.

Findings file: the owner's notes file `invasion-schedule.md`, kept outside the repository.

## Possible explanations (unconfirmed)

- The invasion is in the other faction's zone.
- It is elsewhere on a large map.
- The character does not meet a level or unlock requirement.

## Proposed (not agreed)

- Keep the :30 reminder.
- Reword the card and the Settings text to say the invasion starts at a random field spot and may not be in your zone.
- Settle it by the user noting, for a few hours in the open world, whether the in-game announcement or map marker appears.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The reminder text (card and Settings) matches what the game does: the invasion starts at a random field spot and may not be in the player's zone.
- [ ] #2 The user confirms, after noting over a few hours in the open world whether the in-game announcement or map marker appears, that the reminder and the game agree.
<!-- AC:END -->

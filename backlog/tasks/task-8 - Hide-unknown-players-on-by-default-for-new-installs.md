---
id: TASK-8
title: Hide unknown players on by default for new installs
status: In Progress
assignee: []
created_date: '2026-10-06 20:51'
labels:
  - feature
dependencies: []
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Players who only want to see identifiable people on the meter get unnamed "ID:<n>" rows hidden without having to find the setting first.

User (2026-10-06): "Could you add a toggle on by default that hides the ID:{Number} for people who want to see only identifiable people please". Asked whether the new default should apply to everyone or only new installs, the user chose "New installs only".

## Scope

Covers the default value of the existing Settings → Behaviour → "Hide unknown players" switch ("Leaves out players the meter could not name"): it starts on for a new install, that is, a first launch with no settings.json.

Does not cover:
- Existing installs. A settings.json that already exists keeps its saved value; it is not migrated.
- A new toggle. The switch already exists and is reused.
- What the switch hides. Its behavior stays as it is.

## Current behavior (from the code)

- The switch is off by default.
- When on, it hides unnamed "ID:<n>" rows on the Target, Session and PvP boards.
- The user's own row stays visible even while it is unnamed.
- Each settings.json stores the value, so existing installs keep it. The user's own settings.json stores false.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On a first launch with no settings.json, Settings → Behaviour → "Hide unknown players" is on and no "ID:<n>" rows appear on the boards.
- [ ] #2 An existing settings.json keeps its saved value for this switch.
- [ ] #3 The user's own row stays visible while the game has not named them.
<!-- AC:END -->

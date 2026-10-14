---
id: TASK-25.25
title: Every Settings change wakes every consumer
status: In Progress
assignee: []
created_date: '2026-10-08 12:41'
updated_date: '2026-10-12 15:05'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: low
type: enhancement
ordinal: 62000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Every change in the single Settings object wakes every part of the app that listens to it, even if the change is unrelated to them. Dragging an overlay-opacity slider makes the to-do service, the reminder service and the whole Settings window do copy-and-compare work. This is not a correctness bug; it is avoidable work during a drag.

## Why it happens

All settings live in one `Settings` entity; each consumer observes the whole thing. The trace the review followed:

- `settings/effects.rs` (around lines 38-78): a change clones the new settings and compares language, overlay, hide-unknown, decision log, characters and the core update.
- To-dos (`todos/service.rs` around lines 123 and 174-200): `follow_settings` always clones and compares `todos` and `known_characters`, even when only opacity changed.
- Reminders (`reminders/service.rs` around lines 70-84, `all.reminders.clone()` at line 71): the observer always clones `reminders` and `decision_log`.
- The Settings view (`settings/view.rs` around lines 387-395): the observer always notifies the whole window and rebuilds the preview settings, even when To-dos membership changed.
- While Settings is open, `settings/runtime.rs` (around lines 381-394) polls every second, rebuilds the personal-best rows (translations and mob names) and compares them, and re-lists the recordings folder.

Live application of a setting is otherwise one clean path: `Settings::update` then `effects::connect`, then the to-do and reminder observers for their slices. There is no second writer of the language.

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

In the existing observers, compare before cloning (Reminders already compares the config; the To-dos observer should skip `todos.clone()` and `known_characters.clone()` unless they changed). The Settings view should notify only when the overlay preview inputs changed. Do not split the Settings entity until a second stored file exists.

## How to verify

Counting clones or notifications while dragging an unrelated slider (a test with a counter, or a measured count) shows none for the to-do and reminder services. The preview and to-do and reminder behaviour stay as before; the existing effects tests pass.

## Related

A13 (TASK-25.23), A9 (TASK-25.20, the meter's own redundant rebuilds).

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Changing an unrelated setting (for example overlay opacity) causes no work in the to-do and reminder services.
- [ ] #2 The Settings window refreshes its preview only when a preview input changed.
- [ ] #3 The existing settings effects tests pass and to-do and reminder behaviour is unchanged.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Fixed in code. Every long-lived Settings observer compares before notifying; the Settings window now redraws only for a change its open page shows (and for the save error), the prompt and What's new windows only for a new overlay look, and the store observer compares before cloning the settings. Moved to In Progress; the Settings opacity slider smoothness is for the user to check in game.
<!-- SECTION:NOTES:END -->

---
id: TASK-25.28
title: >-
  Small duplicated helpers that already diverge (unnamed player, fight duration,
  colours, weekday)
status: To Do
assignee: []
created_date: '2026-10-08 12:41'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: low
type: enhancement
ordinal: 65000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Several small helpers are written twice and the copies have already drifted, so the same thing looks or behaves differently in different windows: the PvP window shows a different placeholder for an unnamed player and cannot mark the local player as "You"; history and the overlay compute a fight's duration separately; overlay colours are copied between two files; and the weekday is computed with two different conventions.

## Why it happens

- Unnamed-player text: `format::unnamed_player` (`crates/app/src/format.rs` around lines 75-81) returns "You" (through `fl!(common-you)`) for the local player or `ID:{id}` otherwise. `crates/app/src/pvp/data.rs` (around lines 43-44 and 225-229) builds its own `ID:{actor_id}` display name with no "You" branch, and the watch-row construction uses a third `format!("ID:{id}")`; `meter/live.rs` (around lines 364-368) goes through the shared function. The hide-unknown, You and `ID:{n}` rules are locked product rules.
- Fight duration: `history/list.rs` (around lines 270-281, `duration_secs`) folds the target start and last times with the same finite checks as `meter/live.rs` `fight_secs` (around lines 113-122, already public).
- Colours: the tokens TEXT, MUTED, FAINT, INK, SOFT, HOT, GOOD, WARN, BAD, CARD_RGB, CARD_BORDER and GAUGE_EMPTY appear in `meter/view.rs` (around lines 35-60) and `secondary/chrome.rs` (around lines 25-50), with `style.rs` holding a third TRACK. The values match today; nothing enforces it. Meter-only colours (the "you" badge, the personal-best colours) stay in the meter view.
- Weekday: `todos/resets.rs` (around lines 11 and 74-75) uses Sunday-first (`WEDNESDAY = 3`, `(day + 4).rem_euclid(7)`); `reminders/events.rs` (around lines 22-33) uses Monday-first (`(days_since_epoch + 3).rem_euclid(7)`). Both are correct for 1970-01-01 being a Thursday and tests pass; `instant_of_wall_minute` in `clock/wall.rs` is already shared but the weekday is not.

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

- PvP: use `format::unnamed_player(actor_id, is_you)` everywhere.
- History: call the existing `fight_secs`.
- Colours: use the chrome (or style) constants in the meter view.
- Weekday: one `clock::weekday` with a single convention used by both callers, plus a table test for `instant_of_wall_minute` in `clock/wall.rs` so the DST rules are not only tested through callers.

## How to verify

In the PvP window an unnamed local player shows "You" and others show the same placeholder as the meter, in both languages. A history row's duration equals the overlay's duration for the same fight. Changing a colour token in one place changes both windows. The Wednesday weekly reset and the Monday/Thursday/Saturday siege tests still pass.

## Related

TASK-5 and TASK-11 (unnamed and unknown players on the scoreboard) own the product behaviour of the placeholder.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The PvP window uses the same unnamed-player text as the meter and shows "You" for the local player.
- [ ] #2 History and the overlay compute a fight's duration with one function.
- [ ] #3 Overlay colour tokens exist once, and weekday is computed with one convention by the to-do resets and the reminders.
- [ ] #4 Existing to-do reset and reminder schedule tests pass.
<!-- AC:END -->

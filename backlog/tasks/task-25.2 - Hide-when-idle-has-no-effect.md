---
id: TASK-25.2
title: >-
  Hide the overlay between fights (finish the hide-when-idle inherited from
  Aether)
status: In Progress
assignee: []
created_date: '2026-10-08 12:35'
updated_date: '2026-10-12 15:05'
labels:
  - feature
dependencies: []
parent_task_id: TASK-25
priority: medium
type: feature
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Intent

Hide the meter overlay between fights and bring it back when the next fight starts, using the "hide when idle" scaffolding that is already in the code.

## Agreed (user, 2026-10-08)

- Build the feature, because part of it already exists in the code.

## Where this comes from

The user never asked for this feature. Helveticxa added it to Aether in 2.2.0 (commit `95f1ff7`, 2026-09-24, "a meter that steps aside between fights"), before this project forked from Aether at `d9f1d98`. The the removed FORK.md (git history) section on it (around lines 408-434) describes Aether's Tauri version, where hiding went through `aion2_focus`. That module does not exist in the GPUI app. The GPUI rewrite (Phase 2, `388f443`) carried over the core half and never built the window half.

## What already exists (commit 7a48824)

- **Idle detection in the core.** The snapshot loop (`crates/core/src/engine/meter.rs`, around 698-708) computes "hidden" when `hide_when_idle` is on, no peek is running and nothing of yours has happened since the last reset (`DataStorage::activity_at`). It emits `MeterEvent::DpsIdleHidden(bool)` when that value changes. It also emits `DpsIdleHidden(false)` when the meter stops.
- **Peek API.** `DpsMeter::peek_overlay(secs)` keeps the overlay up for a few seconds with no fight. Nothing in the app calls it.
- **Config.** `DpsMeterConfig::hide_when_idle` (`crates/core/src/config.rs`) defaults to on. The Settings window has no switch for it, and `crates/app/src/live_session.rs` passes the default.
- **Activity rule.** "Activity" means a hit you dealt, a hit you took, or a hit your party landed on your current target. Other people's fights nearby do not count (the removed FORK.md (git history), "Activity, not damage, decides").

## What is missing

- **The app ignores the event.** `MeterModel::apply` (`crates/app/src/meter/model.rs`, around 391-394) handles `DpsIdleHidden` like events it does not care about, and nothing hides or shows the overlay window. The test `events_nothing_draws_do_not_ask_for_a_redraw` asserts the event does nothing. It must change with this work.
- **No Settings switch** for players.
- **No peek triggers.** In Aether a peek ran for 8 s after Start, 4 s after Reset, and 15 s from the show shortcut. The GPUI app has the Alt+CapsLock shortcut (`crates/app/src/hotkey.rs`) but no peek.

## Open questions (not decided)

- Should players get a switch in Settings, and should it be on or off by default? With today's default (on), shipping the window half would hide the overlay for every player between fights.
- Which peeks should exist in the GPUI app (shortcut, launch, reset), and for how long?
- How should it work with the overlay's own windows? Reminder cards sit on the meter, and a locked rule says card notifications must appear even during loading screens. The "…" menu, the summary card after a boss kill, and an open History, Report or Settings window may all need the overlay to stay visible.
- Should the overlay come back at the first hit or only once the first snapshot of the fight arrives?

## Suggested direction (Proposed, not agreed)

- Handle the event on the UI thread through the bridge.
- Hide and show the overlay window without taking focus. `crates/app/src/overlay.rs` already has show-without-activating and `SW_HIDE` for other cases.
- Keep the core's detection as the single source of "idle".
- Add the Settings switch (both languages, through i18n) once its default is decided.
- Update the removed FORK.md (git history) and the removed docs/GPUI_MIGRATION.md (git history) so they describe the GPUI behaviour.

## How to verify

- A model test: the event hides and shows the overlay.
- A replay (`--replay` with `--features probes`, not elevated): the overlay hides after a fight ends and comes back on the next fight's hit.
- A manual check by the user in game.

## Related

- TASK-25.1: the overlay stops updating between hits. It is the same snapshot loop, but a different problem.
- TASK-18: the Shugo reminder was not seen. Check that hiding the meter never hides a reminder card.
- TASK-26: fork cleanup. This feature came from Aether.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit. Core capture, parser, events, meter and data_storage changed afterwards, so find each point again in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Between fights the overlay window is hidden without taking focus, and it shows again when your next fight starts, as decided in the open questions
- [ ] #2 Reminder cards and other overlay windows still appear while the meter is hidden
- [ ] #3 No test asserts that the idle-hidden event does nothing
- [ ] #4 the removed FORK.md (git history) and the removed docs/GPUI_MIGRATION.md (git history) describe the GPUI behaviour
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09 decisions (user, answering Main's questions): a switch 'Hide between fights' in Settings → Overlay, off by default; the meter comes back at your first hit; it stays visible while the boss summary card shows and for a few seconds after it is opened from the toolbar; between fights the toolbar's meter icon shows it (10 s); no shortcut. Main's defaults: opening at launch also shows it 10 s; clicking the meter icon while it is hidden this way shows it instead of closing it; reminder cards go under the toolbar meanwhile; it also stays visible while the first-open prompt waits for an answer; mockup waived for one standard switch row. Built and reviewed (agent-tested, not yet tested by the user). In-game check left: the meter hides after a fight, returns at the first hit, stays up during the boss summary, the toolbar icon shows it 10 s, reminder cards appear under the toolbar.

2026-10-09 22:25: the user's meter crashed live ('Did an agent kill my dps meter ? If not it crashed on me') when a Dimensional Invasion reminder card opened with 'Hide between fights' on and the meter open and idle. Cause (this ticket's work): the app checked whether the meter was stepping aside while the list of open windows was still locked during the card window's creation, which panicked inside Windows' drawing and ended the process. Fixed for every window kind; a test reproduces the user's state; panics are now written to aether.log with where they happened. Reviewed; release exe rebuilt; not yet tested by the user.

2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Fixed in code (the core emits the idle-hidden event, default off; the app hides and shows the meter). Waiting for the in-game check of this ticket's boxes.
<!-- SECTION:NOTES:END -->

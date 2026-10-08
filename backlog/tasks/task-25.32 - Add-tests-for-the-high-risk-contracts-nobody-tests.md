---
id: TASK-25.32
title: Add tests for the high-risk contracts nobody tests
status: To Do
assignee: []
created_date: '2026-10-08 12:44'
labels:
  - maintenance
dependencies: []
parent_task_id: TASK-25
priority: medium
type: task
ordinal: 69000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Several contracts that the meter depends on every minute have no test at all, so a change can break them without any test failing. The worst gap is the calculator that builds the frame the overlay shows: the damage-per-second formula, the merge of summons into their owner, the one-second floor on the DPS time base, the "last target" view when nothing was hit, hiding unknown players and truncating to the player cap are all untested. The local-clock helpers, the debounced saving of settings and to-dos, and the 150 ms debounce that pushes settings changes into the core are also untested.

## Why it happens

The tests in this repository were written next to the code that was changed at the time, mostly for the new GPUI windows, the parsers and the persistence. The older engine code inherited from the forks never got tests, and the app's "save later" behaviour is exercised only by tests that wait for the save to be finished. The uncovered items, with where they live:

- `crates/core/src/engine/calculator.rs`: has no `#[cfg(test)]` module. `DpsCalculator::get_dps_snapshot` / `build_combat_snapshot` produce every overlay frame. Nothing checks the DPS formula, the summon merge, the 1 s DPS floor, the empty last target, hide-unknown filtering or the player-count truncation.
- `crates/app/src/clock/wall.rs`: no direct tests. `instant_of_wall_minute` has a `(false, false) => before` branch (neither candidate instant is valid) that no test reaches. The reminder and to-do schedule tests cover it only indirectly.
- Debounced save vs quit: settings and to-dos are saved after a short delay (to-dos 400 ms). `flush` is wired to quit (`crates/app/src/lib.rs` ~168, `todos/service.rs` ~542) but no test proves that a change made inside the delay survives a `flush`, rather than being lost with a dropped timer.
- `crates/app/src/settings/effects.rs`: the 150 ms debounce before a setting reaches the core, and what happens to the pending task when it is dropped, is untested; only the pure `core_update` mapping is tested.
- `keep_valid_fields` (settings store): the leaf-by-leaf merge is tested for reminders and characters, but not for a nested overlay object with one bad field.
- Capture-level parsing of raw network bytes (`parse_captured_packet`, `parse_network_packet` in `crates/core/src/capture`): no tests for a short header, a bad IP header length or an empty payload.
- The capture and dispatch locks (`config`, `state`): a poisoned lock would panic the capture thread; nothing tests that.
- Replay of a recording cut in the middle of a payload (`replay_recording`): any short read is treated as end of file, including a corrupt middle; untested.
- Nickname packets: almost no English or French name-packet fixture; the existing tests cover character keys and single CJK names only.
- The ellipsis menu of the meter closes when the user clicks outside it (a scrim exists in `crates/app/src/meter/view.rs` ~1226-1236); no test.
- Tool buttons of the meter have accessible names; no test.

Contracts that other tickets already own are NOT part of this ticket: the snapshot-send gate (TASK-25.1), `DpsIdleHidden` hiding the overlay (TASK-25.2), history corrupt-file and serialise failure (TASK-25.3), personal-best atomic write and unreadable file (TASK-25.4), history writes off the capture thread (TASK-25.5, TASK-25.6), packet drop under a blocked dispatcher (TASK-25.7), monster damage-over-time not counted as player damage (TASK-25.8), the 50 MB accumulator wipe and the double-applied main actor from two assemblers per flow (TASK-25.9), replay using the packet's own time (TASK-25.16), the region profile disagreement between the nickname and spawn parsers (TASK-25.18), English text in the clipboard and accessibility labels (TASK-25.26), the 120 s dungeon-boss release (TASK-25.10) and the process-listing question (TASK-25.22). Each of those tickets brings its own regression test.

Not testable automatically (stay manual): closing the main window terminates the process (`quit.rs` uses `TerminateProcess`; tracked in TASK-6).

## Evidence

Reported by the review (static reading), not re-verified. Source: the test-quality and "uncovered contracts" sections of the architecture, core, meter and settings reports; line references are to commit 7a48824.

## Suggested direction (Proposed, not agreed)

Add tests for the items above, one contract each. The review's rule for every test: it defends a named contract and its expected value is written independently (a literal, or computed by a different route than the code), never by repeating the code's own formula. Prefer a fake clock over sleeping. For the calculator, feed a small known storage state and compare against hand-worked numbers (for example two players, one summon, a fight of known length). Prefer observing user-visible results (focus, values, what the window shows) over private state.

## How to verify

Each new test fails when the behaviour it names is broken on purpose (change the 1 s floor, drop the summon merge, skip the flush on quit, drop the debounce task) and passes on the current code. The test names state the contract. No production behaviour changes.

## Related

The regression tests for TASK-25.1 to TASK-25.9 belong to those tickets. TASK-25.33 (T2) removes tests that defend nothing or defend bugs; do T2 first so that new tests are not written next to tests that are about to go.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The calculator has tests that check, against independently worked numbers, the DPS value, the merge of a summon into its owner, the 1 s DPS floor, an empty last target, hiding unknown players and the player-count truncation.
- [ ] #2 A change made inside the save delay of settings and of to-dos is still on disk after the quit flush; a test fails if the flush is removed.
- [ ] #3 The 150 ms debounce of setting changes to the core, and the case where its pending task is dropped, are covered by a test using a fake clock.
- [ ] #4 clock/wall.rs, the nested-overlay keep_valid_fields case, short or empty network packets, a recording cut mid-payload and the ellipsis-menu outside click each have a test that fails when the behaviour is broken.
- [ ] #5 No test added here recomputes the code's own formula, and no production behaviour changes.
<!-- AC:END -->

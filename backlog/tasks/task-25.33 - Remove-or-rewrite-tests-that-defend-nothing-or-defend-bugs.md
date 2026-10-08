---
id: TASK-25.33
title: Remove or rewrite tests that defend nothing or defend bugs
status: To Do
assignee: []
created_date: '2026-10-08 12:45'
labels:
  - maintenance
dependencies: []
parent_task_id: TASK-25
priority: medium
type: task
ordinal: 70000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Part of the test suite (about 320 core tests, 456 app tests, 10 xtask tests and 38 packet-explorer tests at the last count) does not protect anything: some tests check a data structure from the standard library, some check that the code equals itself, some pin the current defaults of another crate, one pins a bug as if it were the product rule, and some depend on real time or on the wording of a log message. They cost maintenance, slow the suite, and fail for the wrong reasons (or worse, keep passing while the product is wrong). One of them actively blocks fixing a High bug.

## Why it happens

Tests were written to cover code that was just written or just ported, with the expected value derived from the code under test, and a few were written to describe a behaviour that later turned out to be wrong. Nothing in the project asks "which contract does this test defend, and would it fail if that contract broke?"

## The list (each test, with the reason)

Remove (the test defends nothing), or rewrite where the contract is real but the oracle is wrong:

Defends a bug:
- `events_nothing_draws_do_not_ask_for_a_redraw` (`crates/app/src/meter/model.rs` tests): asserts that `MeterEvent::DpsIdleHidden` does nothing. That is the bug in TASK-25.2 ("Hide when idle" has no effect). Rewrite it together with the fix for TASK-25.2 so that it asserts what was decided there (the overlay hides and shows, or the event no longer exists). Do not delete it before that decision, otherwise the bug loses even its accidental record.

Tests a standard container or plumbing, not a product contract:
- The tests in `crates/app/src/secondary/registry.rs` (~59-118): a HashMap wrapped in extra types; the window-layer tests in `secondary/mod.rs` already cover the product. Remove.
- `bounded_map_tests::combat_targets_are_bounded` (`crates/core/src/storage`): checks the capacity of a generic bounded map, not that ingest keeps combat targets bounded. Remove, or rewrite through ingest.
- `crates/core/src/capture/channel.rs` tests: test the queue's own capacity and timeout, not capture. Remove with the custom channel if TASK-25.13 replaces it by the standard one.
- The `census` test that looks up `"99,99"` right after `observe`: checks the map it just filled. Remove or replace with a real census contract.
- `without_a_data_folder_nothing_is_stored` (`crates/app/src/settings/files.rs` ~129): asserts `Default`. Remove.
- `the_choice_is_stored_by_name` (`crates/app/src/i18n/language.rs`): serde plumbing. Remove.
- xtask `sha256_hex_matches_the_known_digest_of_abc`: checks the standard library's SHA-256, not the dist tool. Remove.
- `sanitize_nickname_keeps_single_cjk_extension_character` (`crates/core/src/capture/parser/nickname.rs` ~458-460): defends Han character ranges, not a name contract for the English and French clients this product targets. Replace with English and French name-packet fixtures (see TASK-25.32); the CJK characters are intentional test input, not fork leftovers.

Expected value computed the way the code computes it (cannot fail when the formula is wrong):
- `the_party_dps_adds_the_players_up` (`crates/core/src/report/fight_report.rs`): expected = the same sum. Rewrite with a literal number from a worked example.
- `summary::the_best_line_follows_the_pace` (`crates/app/src/meter/summary.rs`): repeats the thresholds `best_line` implements. Rewrite with explicit pace values and the expected text written out.
- Encoder/parser pairs that can drift together: `hit_frame` and `Spawn::frame` fixtures (`crates/core/src/capture/parser` and `processor.rs` tests). They do catch truncation and unknown flag bits, so keep them, but add at least one fixture of real recorded bytes per record type so the parser is not only checked against its own encoder (`periodic_record`'s two 19-21 byte literals already look recorded).

Pins another crate's defaults or internal state:
- `live_session::backend_config_matches_the_defaults_of_the_previous_app` (`crates/app/src/live_session.rs`): pins core defaults that this crate does not set. Remove.
- `a_locked_summary_card_has_no_report_button` (`crates/app/src/meter/view.rs`): sets `view.locked = true` directly instead of going through `set_locked` / click-through, so it can pass while the real path is broken. Rewrite through the real lock path.
- `tab_walks_the_sidebar_then_the_page_in_reading_order_and_wraps` (`crates/app/src/settings/view.rs` ~2438-2474): pins an exact sequence of 22 debug ids, so adding a legitimate control fails it. Keep the reading-order contract (tab order follows the visual order and wraps), loosen the oracle (assert relative order and wrap, or use accessible names).
- `overlay_settings_change_the_preview_but_never_move_the_rows` (`settings/view.rs` ~2656): pins layout bounds. The mockup requires it, but it is an implementation pin; confirm whether to keep it as is.

Duplicates:
- French plural checks exist in both `crates/app/src/i18n/tests.rs` and `crates/app/src/todos/copy.rs` (`french_counts_of_zero_and_one_are_singular`). Keep one.

Depends on the machine or on real time:
- `clock::local` `the_clock_is_after_2025` (`crates/app/src/clock/local.rs`): depends on the clock of the machine. Remove, or replace with a fake-clock check.
- Framing tests (`framing_tests` in `crates/core/src/capture/processor.rs`: large packet across 1460-byte segments, garbage resync cap) and the dispatcher tests (web-port heartbeat must not lock, game-port heartbeat must; a 5 s sleep poll) use `thread::sleep`. The contracts are real, keep them; replace the sleeps with an injected clock or a wait on an explicit condition.
- `meter::stopping_a_running_meter_does_not_wait_out_the_snapshot_intervals` (`crates/core/src/engine/meter.rs`): a timing test, not a product rule. Remove, or replace with a deterministic stop signal check.
- To-do service tests call the real `now_ms()` (ticker off, so they are deterministic except `apply_resets` at process start). Give `Todos::moment` an injectable clock when these are touched.

Coupled to wording:
- Capture-check tests (`crates/app/src/capture_settings.rs`) parse the English detail text of the core (`adapter_count`), so a log wording change breaks them. Assert on a structured value instead.

Keep (the reviewers marked these as real contracts; do not remove): the meter model ranking, You marker, row cap, pace and personal best, nickname mask, boss correction, final summary on kill; focus unit tests (20 s switch, quiet release, involvement); the report GPUI tests (sort, inline player and back, keyboard, pointer lift only on enter/leave); settings and to-do persistence tests (garbage, newer schema, set-aside); DST and wall-clock tests; i18n key parity; identity and own-actor tests; record-parser boundary tests; TCP reassembly tests; `catalogue::ids_are_unique_and_groups_hold_rows_of_their_own_section` (structural, but it protects persisted ids); the session tests (combat-time union, PvP excluded).

## Evidence

Reported by the review (static reading), not re-verified. Sources: the test-strategy section of the architecture report and the test-quality sections of the core capture, core domain, meter and settings/to-dos reports. File and line references are to commit 7a48824; test names may have moved.

## Suggested direction (Proposed, not agreed)

Go through the list, decide per test: remove, rewrite with an independent expected value, or keep. Delete rather than comment out. Do not touch tests in the "keep" list. Each rewritten test must fail when the contract it names is broken on purpose.

## How to verify

The suite builds and passes after the change; every removed test is accounted for in the change notes with the reason above; every rewritten test fails when its contract is deliberately broken (mutation by hand: change the pace threshold, break the lock path, reorder two controls only for the loosened tab-order test). The test count in AGENTS.md is updated (see TASK-25.35).

## Related

TASK-25.2 (R2: the test that pins the hide-when-idle bug is rewritten with that fix). TASK-25.32 (T1, adds the missing tests; do this ticket first). TASK-25.13 (parsers return records; may remove the custom channel). TASK-25.35 (T4, test counts in the docs).

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every test named in the list is removed, rewritten or explicitly kept, and the change notes give the reason for each.
- [ ] #2 The test that asserts the hide-when-idle event does nothing no longer pins the bug (rewritten with the fix for TASK-25.2).
- [ ] #3 Rewritten tests use expected values that are independent of the code's own formula and fail when the named contract is broken.
- [ ] #4 No test from the 'keep' list is removed, and the suite passes.
<!-- AC:END -->

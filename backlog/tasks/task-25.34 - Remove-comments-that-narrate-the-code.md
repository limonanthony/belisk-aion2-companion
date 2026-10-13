---
id: TASK-25.34
title: Remove comments that narrate the code
status: To Do
assignee: []
created_date: '2026-10-08 12:45'
updated_date: '2026-10-12 15:05'
labels:
  - maintenance
dependencies: []
parent_task_id: TASK-25
priority: low
type: chore
ordinal: 71000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

A large part of the code is covered in comments that only repeat what the code already says: module headers that list the files next to them, field comments that restate the field name or show a format example, step-by-step numbering inside functions, banner lines, and design essays that retell the history of the forks. The project rule is "no explanatory comments by default; keep only comments that state a non-obvious constraint". The review counted about 1,000 or more doc-comment lines that break it. They add reading time, go stale (several already describe code that moved) and hide the few comments that do matter.

## Why it happens

The code was written and ported in a style that narrates itself, and the comments were never trimmed when the code was moved to the new crates. The comment rule is not enforced by any check.

## Where (from the review)

Count and worst places, by area:

- Core domain: about 80 or more comments in `crates/core/src/storage/data_storage.rs`, 40 or more in `crates/core/src/engine/meter.rs`, a module-level essay in `engine/session.rs` and `report/fight_report.rs`, and numbered walkthroughs in `engine/calculator.rs`. Worst cluster: `calculator.rs` ~70-133, where `// 1. Collect raw data` through `// 9. Extract last-target` and `// ====` banners form a table of contents for the function body. Also delete `// Capture before clearing`, `// Cache non-empty snapshots`, `// Filter unknown players if configured`, `// Truncate to max player count`, and doc comments on `FightReport` that restate field names.
- Core capture: six module-level rationales of 15-25 lines (`recorder.rs`, `rolling_recorder.rs`, `census.rs`, `region.rs`, `preflight.rs`, `windivert_capturer.rs`) that retell design history (the upstream Npcap probe, Aether versus anti-cheat, the opcode census for the global launch); dozens of `///` on private fields that restate names (`payload_offset`, `killer_name`, "Rendered as it appears in the parsers"). Worst: `census.rs` 1-17 (a launch-day essay) and `region.rs` 1-22 (restates the module and the upstream-diff policy).
- Meter (app): about 200 doc-comment lines in `crates/app/src/meter/` alone. Worst concentration in `live.rs`, `summary.rs`, `view.rs` struct fields and `board.rs` (~44-56: comments on `boss_engaged`, `boss_released`, `reset`; `live.rs` ~41-44 comments that only show format examples such as "0-1", "Mobs fought", "A boss was engaged which ends a pin").
- Settings, to-dos, reminders: about 226 module `//!` lines and 854 `///` lines. `crates/app/src/settings/mod.rs` 1-5 and `todos/mod.rs` 1-7 list their submodules; `settings/view.rs` 1-11 repeat the keyboard behaviour the tests encode; `todos/store.rs` and others restate function names. `todos/catalogue.rs` 4-5 states "game names stay English" as a comment instead of the decision being made (that decision is TASK-25.27; do not remove the comment before it is decided).

## What to keep (justified comments)

Do not delete these, they carry facts the code cannot:
- Win32 quirks: `WM_MOUSEACTIVATE` versus `WS_EX_NOACTIVATE` and the reason for not using `WindowControlArea::Drag` and the DPI-stable grab (`overlay.rs` ~83-87, ~331-334, ~394-399); why `TerminateProcess` (`quit.rs` module docs); the single-instance Local namespace and "no process enumeration" (`single_instance.rs`); Settings topmost while active (`secondary/mod.rs` ~209-211); why `text_input` is not the GPUI kit (`text_input.rs` 1-6); shade contrast steps (`shades.rs` 15-19).
- Daylight-saving rules in `reminders/schedule.rs` and `todos/resets.rs` (skipped and repeated hour); the FILETIME epoch in `clock/local.rs`; "capture backend is start-only" in `effects.rs`.
- Corrupt-file handling: set-aside notes in the settings, to-do and mobs stores; history magic and CRC.
- External protocol facts and wire quirks: the `AETHERPC` on-disk layout (`recorder.rs` 13-25), `frame_total_bytes` "plus 4" (`utils.rs` 32-33), `gap_skip` meaning for consumers (`tcp_reassembler.rs` 30-32), WinDivert sniff-only and unload-if-we-loaded (`windivert_capturer.rs` 29-36, 252-253).
- Identity carry and class reject (`bind_own_actor`, `main_actor_changed`), why `dps_stats` is bounded, why `activity_at` ignores nearby fights, why the main-actor callback must not fire on every nickname packet, "do not open the game process", xtask dirty-tree versus `--allow-dirty`.

Commented-out code is handled by TASK-26.1 (dead code), and fork-relative comments by TASK-26.2; do not duplicate their work here.

## Evidence

Reported by the review (static reading), not re-verified. Counts are the reviewers' estimates. Line references are to commit 7a48824.

## Suggested direction (Proposed, not agreed)

Either one pass over the files above, or file by file whenever a file is touched. No behaviour change: this is comments only. Where a comment only shows that a name is unclear, rename instead of keeping the comment.

## How to verify

The workspace builds and the existing tests pass unchanged (comments only). A search of the listed files finds no module header that lists submodules, no numbered walkthrough comments and no banner lines, and every comment in the "keep" list above is still present.

## Related

TASK-25.33 (T2, tests), the dead-code and fork-comment tickets under the fork clean-up work item TASK-26 (TASK-26.1, TASK-26.2), TASK-25.27 (the to-do names decision, for `todos/catalogue.rs`).

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The files listed under Where have no module header that lists submodules, no numbered walkthrough comments, no banner lines and no field comments that restate the name.
- [ ] #2 Every comment listed under 'What to keep' is still present.
- [ ] #3 No code changes other than comments; the workspace builds and the tests pass unchanged.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Still present per the review (numbered // 1. to // 9. comments in calculator.rs); not touched this round. The new overlay and on_top comments are load-bearing Win32 notes.
<!-- SECTION:NOTES:END -->

---
id: TASK-25.37
title: Audit results that are silently ignored (let _ =)
status: To Do
assignee: []
created_date: '2026-10-08 15:38'
updated_date: '2026-10-12 15:05'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: medium
type: enhancement
ordinal: 87000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Many results are thrown away with `let _ = …`. Some of these are failures a player or a maintainer needs to know about: a file that was not written, a folder that could not be created, a packet that was dropped. They fail silently, so bugs look like "it just didn't work".

## Why it happens

- **The count.** The review snapshot has about 128 `let _ =` across the crates, test code included. Many are legitimate, for example ignoring the result of updating a window that has already closed. The fork also brought code that ignores I/O results as a habit.
- **Known harmful examples:**
  - `crates/core/src/personal_best.rs`: `let _ = std::fs::write(…)` and `let _ = create_dir_all(…)`. A failed save of personal bests leaves no trace. TASK-25.4 owns that fix.
  - `crates/core/src/history.rs`: `init_dir` ignores `create_dir_all`.
  - `crates/core/src/capture/capturer.rs` and `windivert_capturer.rs`: `let _ = channel.try_send(packet)` drops packets with no counter. TASK-25.7 owns that fix.
- **Lock unwraps (lower priority).** There are about 192 `.lock().unwrap()` / `.read().unwrap()` / `.write().unwrap()` calls. If any thread panics while holding one of those locks, every later caller panics too, so one bug can take down the capture, snapshot and UI threads one after another. This is usual in Rust and not a defect on its own. Note it, but do not mass-rewrite.

## Evidence

Counts from a grep of commit 7a48824 (all crates, tests included). The personal-best, history and capture examples were verified by Main. Everything else is unreviewed; this ticket is the review.

## Suggested direction (Proposed, not agreed)

- Go through every production `let _ =` and keep the ones whose failure truly does not matter. Where the reason is not obvious from the call, give it a one-line reason.
- Make the others report: log through `AppLogger` (see TASK-25.36), return the error to the caller, or count it (dropped packets). Follow the project's rule: never swallow errors; say what failed, what was attempted, and what the caller can do.
- Leave the lock-unwrap policy as it is unless a concrete poisoning case appears.

## How to verify

- Every remaining production `let _ =` is either self-evidently harmless or carries its reason.
- Failures that matter appear in `aether.log` or reach the caller.
- Regression tests only where a failure is part of a contract (for example a failed save is reported).

## Related

- TASK-25.4: personal bests.
- TASK-25.7: dropped packets.
- TASK-25.3: history.
- TASK-25.36: `eprintln!` lost in the shipped app.

Found by the code review of commit 7a48824 (2026-10-08). Counts refer to that commit; re-count in the current code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every production let _ = is either self-evidently harmless or states why its failure does not matter
- [ ] #2 Failures that matter (file writes, folder creation, dropped packets) are logged, returned to the caller or counted
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Partly fixed. Capture try_send drops are counted inside the channel, and this round the Win32 show, raise and stack results in the overlay path are checked and logged. Remaining: handle.join results, logger create_dir_all and writeln!, recorder flush. Not a full audit.
<!-- SECTION:NOTES:END -->

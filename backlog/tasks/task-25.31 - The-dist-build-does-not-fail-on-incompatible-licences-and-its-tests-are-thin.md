---
id: TASK-25.31
title: The dist build does not fail on incompatible licences and its tests are thin
status: To Do
assignee: []
created_date: '2026-10-08 12:41'
updated_date: '2026-10-12 15:05'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: medium
type: enhancement
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The release build (`dist`) does not stop when a dependency has a licence that is not on the project's compatible list: it prints a warning and builds anyway, so an incompatible licence can ship. Its tests are also thin, so the safety checks it does perform (dirty tree, the checksums of the WinDivert driver files, profile paths) have no automated guard except running the release.

## Why it happens

`crates/xtask/src/notices.rs` (around lines 42-53) prints a WARNING and still returns success when a crate's licence is not on the GPL-3.0-compatible list ("needs review"). The rest of `crates/xtask/src/dist.rs` is careful in production: it refuses a dirty tree, verifies the SHA-256 of `WinDivert.dll` and `WinDivert64.sys`, refuses a locked executable, remaps paths, writes the GPL notices, and builds the source zip from `git archive HEAD` so `--allow-dirty` cannot misstate the source. The tests (`dist.rs` around lines 276-312) cover only line-ending conversion, `sha256("abc")` and the written-offer text.

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

Fail `dist` when any licence needs review unless an explicit flag is passed. Optionally add a unit test that checks the two WinDivert hashes when `crates/app/windivert` is present.

## How to verify

A crate with an unlisted licence makes `dist` exit with an error and without producing the archive; with the explicit flag it proceeds and says what was accepted. The existing dist tests pass, and the new test fails if a WinDivert file is changed.

## Related

No other ticket covers the release build.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A dependency whose licence needs review makes the dist build fail unless an explicit override is given.
- [ ] #2 A test fails if the checked WinDivert files no longer match their recorded checksums.
- [ ] #3 A normal dist build with only compatible licences is unchanged.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Per the services review AC #1 is fixed in code (xtask spdx.rs rejects incompatible and unknown licences; test rejects_incompatible_and_unknown_licences). AC #2 (WinDivert checksum test) and AC #3 were not checked. Not changed by this round.
<!-- SECTION:NOTES:END -->

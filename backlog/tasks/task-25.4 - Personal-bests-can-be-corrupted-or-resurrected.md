---
id: TASK-25.4
title: Personal bests can be corrupted or resurrected
status: To Do
assignee: []
created_date: '2026-10-08 12:35'
updated_date: '2026-10-08 12:35'
labels:
  - bug
dependencies: []
parent_task_id: TASK-25
priority: high
type: bug
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Personal bests can end up corrupted, silently replaced, or brought back after the user cleared them.

## Why it happens

`crates/core/src/personal_best.rs` (around lines 49-67 and 121-131):

- It writes `personal_bests.json` with a plain file write, not an atomic temp-file-and-rename. A crash or power loss in the middle leaves a half-written file.
- It ignores every write error.
- On start, a file that cannot be read is treated exactly like a first run: the bests are rebuilt from the history and the file is overwritten. That can bring back bests the user had cleared, and it silently replaces whatever was there.
- The file has no version field.

History and the mobs file already use an atomic write helper (`write_atomic`), and settings, to-dos and mobs already set aside unreadable files.

## Evidence

Verified by reading the code (commit 7a48824).

## Suggested direction (Proposed, not agreed)

Use the atomic write already used by history and mobs; add a version field; set an unreadable file aside (a `*.bad-<seconds>` name, as other stores do) instead of treating it as a first run; do not ignore write errors.

## How to verify

Unit tests: unreadable file is not the same as first run; the write is atomic. Manual check by the user: clear a best, restart the app, the best stays cleared.

## Related

TASK-25.3 (history wipe: same family). TASK-25.5 (who writes the file).

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A personal-bests file that cannot be read is kept on disk under a set-aside name and is not treated as a first run.
- [ ] #2 A best the user cleared does not come back after a restart.
- [ ] #3 A write interrupted halfway leaves the previous file intact.
- [ ] #4 A missing file (a real first run) still builds the bests from history.
<!-- AC:END -->

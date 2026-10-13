---
id: TASK-25.3
title: Fight history can be wiped when the history file cannot be read
status: In Progress
assignee: []
created_date: '2026-10-08 12:35'
updated_date: '2026-10-12 15:05'
labels:
  - bug
dependencies: []
parent_task_id: TASK-25
priority: high
type: bug
ordinal: 40000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

A player's whole fight history can be lost silently. If the history file cannot be read at start, the next fight that is saved replaces the file with that one fight, and all earlier history is gone. The same can happen, less likely, if saving fails to serialise.

## Why it happens

In `crates/core/src/history.rs`:

- When `combat_history.bin` cannot be loaded (bad magic, newer or older format version, CRC mismatch, decompress failure or JSON failure), the store starts empty and only logs to stderr. The unreadable file is left where it is, and the next fight save rewrites it with only the new fight. The review's slice report says the bad file is "kept"; that is wrong (found by the second check).
- When serialising the records fails, the code falls back to an empty list (`to_vec(...).unwrap_or_default()`) and writes that empty payload over the file. This path is real but unlikely in practice.

Settings, to-dos and the mobs file already rename an unreadable file to a `*.bad-<seconds>` name and keep it; history does not.

## Evidence

Verified by reading the code (commit 7a48824), including the correction above.

## Suggested direction (Proposed, not agreed)

Set the unreadable file aside like the mobs store does on load (`mobs::store::load`), then start empty; never write when serialising fails.

## How to verify

Regression tests that fail before the fix: a corrupt file is set aside and not overwritten after a save; a forced serialise failure leaves the file unchanged. A store-level unit test without a replay is enough.

## Related

TASK-25.4 (personal bests: same family, bests are rebuilt from history). TASK-25.5 (changes who writes history). TASK-15 (fight report, Done) reads this history.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Starting with a history file that cannot be read leaves that file on disk under a set-aside name, and the next saved fight does not remove its contents.
- [ ] #2 A failed serialisation never changes the history file on disk.
- [ ] #3 A readable history file loads exactly as before.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08: started at the user's request ('You can start 1'). Main's choice of direction, delegated by the user: set an unreadable history file aside under a *.bad-<seconds> name and start empty; never write when serialising fails.

2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Fixed in code: an unreadable history file is set aside and the next save does not overwrite it; records that cannot be encoded leave the file as it was. Tests cover both.
<!-- SECTION:NOTES:END -->

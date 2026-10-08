---
id: TASK-14.1
title: Messages of 128 bytes or more are framed one byte short
status: Done
assignee: []
created_date: '2026-10-07 10:02'
updated_date: '2026-10-08 09:31'
labels:
  - bug
dependencies: []
parent_task_id: TASK-14
priority: medium
type: bug
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Messages of 128 bytes or more are read one byte short, so the meter can misread or skip the messages that come after a long one.

## Scope

Covers how the end of each long message is found. Which messages the meter then understands is not changed here.

## Evidence notes

Effect: some messages that follow a long message can be misread or skipped, so damage and other events can be missing or wrong.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A recording with messages of 128 bytes or more is replayed before and after the fix; no message after a long one is misread or skipped after the fix.
- [x] #2 The user confirms the totals look right in a real fight.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed and checked on 7 recordings: long messages now end in the right place; one recording had been losing several hundred messages. Agent-tested only; waiting for the user to check in a real fight.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
User acceptance 2026-10-08: "Most of the tasks you said I've tested and it seems to work correctly (until a new issue appears which is not impossible but for now we can consider them fixed)". Considered fixed on the user's own testing; reopen if a new issue appears.
<!-- SECTION:FINAL_SUMMARY:END -->

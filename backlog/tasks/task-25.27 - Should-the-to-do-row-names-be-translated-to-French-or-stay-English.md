---
id: TASK-25.27
title: Should the to-do row names be translated to French or stay English?
status: To Do
assignee: []
created_date: '2026-10-08 12:41'
updated_date: '2026-10-12 15:05'
labels:
  - research
dependencies: []
parent_task_id: TASK-25
priority: medium
type: task
ordinal: 64000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The to-do window's row names stay in English in the French interface ("Duty Missions", "Odyle Energy", "Shugo Festival keys", dungeon names, "Duty Commands"), although the text around them is translated. A French player sees mixed languages in the window's main content. This is a product question because the code says the English was intended.

## Why it happens

`crates/app/src/todos/catalogue.rs` (around lines 4-5 and 167-207) hard-codes the row labels, and a comment there says the labels are the game's own names and stay English on purpose, only the words around them are translated. `Sub::Game("Unknown Fissure")` is the same, and `todos/list.rs` (around lines 240-243) title-cases class names. The project's general rule (AGENTS.md) is that user-visible text goes through `fl!` with both languages; the review notes this comment is a local exception, not a locked product decision (the locked decisions cover skill and mob names in the meter). `game_data` already has French dungeon names which the to-do list does not use. TASK-7 records, as a Proposed (unconfirmed) item, that game names stay English because there was no French source.

## Evidence

Reported by the review (static reading), not re-verified.

## Decision needed (not decided)

- Option A: translate the rows: a translation key per row id (keeping the id as the stored key), and dungeon names looked up by id through `game_data`.
- Option B: keep the English names, state this as the product rule, and document the exception next to the keys (the review says only true proper nouns the game itself never translates qualify).

## Suggested direction (Proposed, not agreed)

Option A for the rows that have a French name source already (dungeons), and ask the owner for sources for the rest; the earlier translation work in TASK-7 should not be duplicated.

## How to verify

Depends on the decision. If A: the to-do window in French shows the French row names and dungeon names; the persisted row ids are unchanged so existing to-do data still loads; the en/fr key-parity test passes. If B: the rule is recorded and the comment is replaced by it.

## Related

TASK-7 (English and French translations). A16 (TASK-25.26, other English text). TASK-20 and its children (filling to-do counts from the game) use the same row ids.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The owner's decision (translate the rows, or keep English names as the rule) is recorded.
- [ ] #2 Persisted row ids are unchanged: existing to-do data still loads after the change.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Not reviewed by the review of this date; unchanged. Still an open question for the user.
<!-- SECTION:NOTES:END -->

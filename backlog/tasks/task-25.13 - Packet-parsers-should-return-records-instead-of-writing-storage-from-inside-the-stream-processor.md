---
id: TASK-25.13
title: >-
  Packet parsers should return records instead of writing storage from inside
  the stream processor
status: To Do
assignee: []
created_date: '2026-10-08 12:38'
updated_date: '2026-10-12 15:05'
labels:
  - improvement
dependencies:
  - TASK-25.9
parent_task_id: TASK-25
priority: medium
type: enhancement
ordinal: 50000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Adding or changing a game packet means editing the stream processor, a parser file that secretly knows the shared combat storage, and tests that need a full storage object. The packet parsers cannot be reused (for example by packet-explorer) without constructing a whole meter.

## Why it happens

`crates/core/src/capture/processor.rs` (about 1,500 lines) does three jobs: it frames the byte stream, matches opcodes, and hosts most of the tests. The combat parsers (`parser/damage.rs`, `summon.rs`, `buff.rs`, `nickname.rs`) are methods of that processor that write straight into `DataStorage` and read the configuration and region process globals. Only the `*_record.rs` readers (bytes in, a record or nothing out) are already well shaped. The nickname parser already has a small `ParserContext`; the combat parsers do not. Deleting a parser method would remove almost nothing but a pass-through plus logging.

Also reported (low severity): the custom `Channel` type (`capture/channel.rs`) reimplements a bounded queue that the standard library provides, with an `allow(dead_code)` cover over unused methods; `StreamAssembler` (`capture/assembler.rs`) is two fields and four forwarding methods, a third name for "bytes plus processor".

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

- Parsers return records; the processor only frames and dispatches by opcode; the storage applies the records. Reuse `ParserContext` for the remaining parsers.
- Keep the record readers as they are (the review's "strengths to keep").
- After the double parsing of every flow is removed (TASK-25.9), fold `StreamAssembler` into the processor and replace `Channel` with the standard bounded channel.
- Whether this goes ahead depends on the upstream-diff decision (TASK-25.11).

## How to verify

Replaying the project's recordings gives the same totals before and after. Parser tests can build a record from bytes without constructing `DataStorage`. Existing identity and parser tests still pass.

## Related

Depends on TASK-25.9 (stream buffer and double parsing). Enables A3 (TASK-25.14, shared framing). Related A1 (TASK-25.12). The damage-over-time parser fix is TASK-25.8; do that first so the restructuring does not carry the bug along. Subject to the decision in TASK-25.11.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each combat packet parser (damage, summon, buff, nickname) can be tested from bytes to record without constructing the shared combat storage.
- [ ] #2 The stream processor only frames and dispatches; applying a record to the storage happens outside it.
- [ ] #3 Replaying the same recording before and after gives identical totals, names and boss HP.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Partly present per the review: record readers exist, but the combat parsers still write DataStorage from the stream processor. Not touched this round. Its prerequisite TASK-25.9 is fixed in code and the real-fight replay test (TASK-25.39) now exists as a regression oracle.
<!-- SECTION:NOTES:END -->

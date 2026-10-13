---
id: TASK-25.38
title: 'Move the core off layer folders (models/, storage/) and the utils file'
status: To Do
assignee: []
created_date: '2026-10-08 15:38'
updated_date: '2026-10-12 15:05'
labels:
  - maintenance
dependencies: []
parent_task_id: TASK-25
priority: low
type: chore
ordinal: 88000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The core crate is organised in layers, with folders the project's structure rule bans: `crates/core/src/models/` and `crates/core/src/storage/`. It also has a file named `utils` (`crates/core/src/capture/parser/utils.rs`, exported as `pub mod utils`). The app crate already follows the rule; the core does not. Each new piece of core code has to pick between the old layout and the rule.

## The rule (the user's structure conventions)

- Code is grouped by feature or by a named concern.
- No layer folders such as `models/`, `services/`, `repositories/`, `handlers/`.
- No `utils/`, `common/`, `shared/` or `helpers/`. A shared helper moves to a folder or file named after its one concern.

## Why it happens

The core was inherited from Aether/NOIA2 and moved into `crates/core` with its layers intact:
- `models/`: `combat.rs`, `diagnostics.rs`, `packet.rs`, `session.rs`;
- `storage/`: `data_storage.rs`, `loaders.rs`;
- `capture/parser/utils.rs`.

## Evidence

Folder listing of commit 7a48824 (verified by Main). The review's assessment: the core really is a pipeline (capture → parse → combat state → engine → report), so re-folding it into fake features would be wrong. The cost is in the banned names and in types that live away from their owner.

## Suggested direction (Proposed, not agreed)

- **Move types to their owners when you touch them:**
  - combat snapshot types next to the engine that builds them;
  - packet types next to capture;
  - session types next to `engine/session.rs`;
  - loaders next to the data they load (bundled data).
  - Then delete `models/` once it is empty.
- **`storage/`:** the shape of `data_storage.rs` is TASK-25.15. Rename or move the folder as part of that work, not before.
- **`parser/utils.rs`:** rename it to what it holds (byte/varint reading next to `wire_cursor.rs`, for example) and make it `pub(crate)`. Packet-explorer's access goes through TASK-25.14 / TASK-25.12.
- **No big-bang move:** do it with the tickets that already change these files, so the history of each file stays readable.

## How to verify

`crates/core/src/` has no `models/` or `storage/` folder and no `utils` file. Each moved type sits beside the module that owns it. `cargo test --workspace` passes and behaviour is unchanged (same results from the same recording).

## Related

- TASK-25.12: core interface.
- TASK-25.13: parsers.
- TASK-25.14: packet-explorer framing.
- TASK-25.15: DataStorage.
- TASK-25.11: no upstream, so moves are free.

Found by the code review of commit 7a48824 (2026-10-08).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 crates/core/src has no models/ or storage/ folder and no utils file; moved types sit beside the module that owns them
- [ ] #2 Behaviour is unchanged: the same recording gives the same results before and after
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Still present per the review (models/, storage/ and the utils file unchanged); not touched this round.
<!-- SECTION:NOTES:END -->

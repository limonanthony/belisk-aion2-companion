---
id: TASK-26.4
title: Decide what happens to the persisted Aether identifiers
status: In Progress
assignee: []
created_date: '2026-10-08 12:47'
updated_date: '2026-10-09 22:19'
labels:
  - research
dependencies: []
parent_task_id: TASK-26
priority: medium
type: task
ordinal: 77000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Players' installs hold files and folders that carry the forks' names: the data folder `com.helveticxa.aether`, the log file `aether.log`, the recording format `AETHERPC` with its `.aetherpc` extension, and camelCase setting keys from the Tauri era. They are visible to players, to anyone who shares a recording or a log, and they contradict the product's own name (and the planned name Kisk, TASK-23). They cannot be renamed like ordinary code: existing installs depend on them, and changing one without a migration would make the app start with empty settings, to-dos, history and bests.

## Why it happens

The Rust app kept the Tauri app's storage identifiers so that a player moving over from the Tauri build would keep their data. The identifiers were Aether's, so they still say Aether.

## Items (classification: load-bearing — existing installs depend on them)

| Item | Where | Action options |
|---|---|---|
| App data id `com.helveticxa.aether` (under `%APPDATA%`: `settings.json`, `todos.json`, `history/`, `recordings/`, `mobs.json`; under `%LOCALAPPDATA%`: logs) | `crates/app/src/live_session.rs` line 16; `crates/xtask/dist/README.txt` 68-69 | Keep, or migrate together with the Kisk rename (TASK-23) |
| Log file `aether.log` | `crates/app/src/live_session.rs` line 53 | Rename together with the data folder decision, or keep |
| Recording magic `AETHERPC` and extension `.aetherpc` | `crates/core/src/capture/recorder.rs` 17, 37, 404, 440, 561, 589; `crates/app/src/capture_settings.rs` 596-599 (verify); `crates/packet-explorer/src/args.rs` line 9 | Keep. If renamed, the reader must accept both magics and both extensions (a format bump), because players have recordings and the packet-explorer reads them |
| camelCase serde keys in `settings.json` (Tauri era) | `crates/app/src/settings/model.rs`; `crates/app/src/meter/settings.rs` 10; `reminders/settings.rs` 9; `todos/settings.rs` 7; `todos/data.rs`; `i18n/language.rs` 51; `crates/core/src/region.rs` | Keep the keys. Changing them needs a migration reader |
| History file format (magic `DPSH`, a u16 version, CRC32, lz4) | `crates/core/src/history.rs` 219-226, 268-276 | Keep. Any change is a version bump (and see TASK-25.3, TASK-25.21 for the work already planned on this file) |

## Decision needed (user, not decided)

For each item above (or for the whole group): keep the identifier forever, or migrate it at the Kisk rename (TASK-23, whose open question is "whether and when the internal identifiers and data folder are renamed, and how existing user data migrates"). The review's recommendation, Proposed: keep the settings keys and the history format (no player benefit, real migration risk); decide the data folder, log name and recording magic together with the Kisk rename, since a migration is needed once and is cheapest then. This ticket has no recommendation on the user's preference for how visible the old name is.

## Evidence

Reported by the inventory (static reading of commit 7a48824), not re-verified. Items marked "verify" by the scout must be re-checked; line numbers must be re-located in the current code.

## Acceptance

- The decision for each item (keep or migrate, and when) is recorded in this ticket and in TASK-23's open question about internal identifiers.
- Only if migration is chosen: an existing install upgraded to the new build keeps its settings, to-dos, history, personal bests and recordings; old recordings still open; the old folder is not deleted until the new one is confirmed written. That work is then split into its own tickets; this ticket stays the decision record.

## Related

TASK-23 (Kisk rename; same open question), TASK-25.3, TASK-25.4 and TASK-25.21 (history and bests persistence work), TASK-26.3 (F3: the non-persisted identity items), TASK-26 (parent). No native dependency: this is a decision.

Source: fork-leftovers inventory of commit 7a48824 (2026-10-08). Line numbers refer to that commit; re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 For the data folder, log file, recording magic and extension, settings keys and history format, the user's decision (keep or migrate, and when) is recorded here and in TASK-23.
- [ ] #2 If any item is migrated: an existing install keeps its settings, to-dos, history, personal bests and recordings, and old recordings still open.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09 user decision: 'Move to a Belisk folder automatically'. Built and reviewed twice (not yet tested by the user): on the first launch of Belisk, %APPDATA%\com.helveticxa.aether moves to %APPDATA%\Belisk and the logs to %LOCALAPPDATA%\Belisk\logs (belisk.log). If the move fails (e.g. the old meter still running) that run keeps the old folder and retries next launch. If both folders hold files, the Belisk one is used and the old one is left untouched (a warning in the log). Settings keys and the history format are unchanged.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-08 14:50
---
Decided for the recording format (user, 2026-10-08): keep the binary format; new magic and the extension .kiskrec; the reader keeps accepting AETHERPC/.aetherpc permanently so later format changes never break players' recordings. Work split into TASK-26.6. Still open here: the data folder com.helveticxa.aether, aether.log, the camelCase settings keys and the history format.
---
<!-- COMMENTS:END -->

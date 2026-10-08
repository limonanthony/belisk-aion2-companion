---
id: TASK-25.21
title: >-
  One JSON store protocol is copied three times, and history and bests use
  weaker ones
status: To Do
assignee: []
created_date: '2026-10-08 12:40'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-25
priority: medium
type: enhancement
ordinal: 58000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The rule for storing a file safely (versioned, written atomically, an unreadable or newer file is set aside rather than overwritten) exists in three copies, and the two files that matter most to the player (fight history and personal bests) use weaker rules. A change to the rule has to be made in several places and the weaker ones are where data is lost today.

## Why it happens

`settings.json` (`crates/app/src/settings/store.rs`), `todos.json` (`crates/app/src/todos/store.rs`) and `mobs.json` (`crates/core/src/mobs/store.rs`) each implement the same protocol by hand: a missing file starts fresh; an unreadable file or a `version` above the supported schema is renamed to `<file>.bad-<seconds>`; saving adds `"version": 1` and writes through `write_atomic` (temp file, sync, rename). Settings and to-dos then add their own clean-up step (`keep_valid_fields`, `sanitised`). History and personal bests were written separately with weaker handling (see TASK-25.3 and TASK-25.4). The settings and to-do stores also share a save debounce (400 ms, `schedule_save`, `write_in_order`); that has only two callers and should stay as it is.

Persistence inventory from the review (app data root `%APPDATA%/com.helveticxa.aether`, logs under `%LOCALAPPDATA%/com.helveticxa.aether/logs`):

| File | Writer | Mechanism | Versioned | Corrupt handling | Thread |
|---|---|---|---|---|---|
| `settings.json` | `app/settings/store.rs` | `write_atomic` (temp, sync, rename) | yes, `version: 1` | unreadable or newer schema: renamed `settings.json.bad-<secs>`, defaults; valid fields kept | GPUI background after 400 ms; flush on quit (UI) |
| `todos.json` | `app/todos/store.rs` | same `write_atomic` | yes, `version: 1` | same set-aside; a bad entry is dropped, file kept | GPUI background debounce; flush on quit |
| `history/combat_history.bin` | `core/history.rs` | lz4 + CRC + magic `DPSH`, then `write_atomic` | u16 `VERSION = 1` | serialise error writes an empty list over the file; load failure logs and keeps the bad file but memory is empty (next save overwrites it) | dispatcher (player change), snapshot (idle), UI (delete), quit; mutex held across I/O |
| `history/personal_bests.json` | `core/personal_best.rs` | plain `fs::write`, errors ignored | no | unreadable: treated as first run, re-seeded from history and written | same owners as history save |
| `mobs.json` | `core/mobs/store.rs` through `MobCatalog` | `write_atomic` | yes, schema version | renamed `mobs.json.bad-<secs>` | dedicated writer thread with a dirty flag |
| `recordings/*.aetherpc` | `PacketRecorder` / rolling recorder | streaming `BufWriter`, magic `AETHERPC` v1 | u16 in header | truncated trailing record is end of file; bad magic refused on replay; 512 MiB cap, 2 GiB rolling prune | dispatcher per packet when enabled |
| `recordings/auto-*.aetherpc` | same recorder, started from the memory-snapshot thread | same | same | same | start/stop from the memory thread; writes on the dispatcher |
| `recordings/diagnostics-*.txt` | `DpsMeter::save_diagnostics_report` | `fs::write` | no | none | caller (not hot) |
| `logs/aether.log` (+ `.log.1`) | `AppLogger::write_line` | append, roll at 4 MiB | no | write failure swallowed | calling thread |
| `*.json.bad-<secs>` | settings, to-dos, mobs load | `fs::rename` | n/a | the quarantine | load |

Recordings are a binary stream and the log is an append file; they should not be forced into the JSON protocol.

## Evidence

Reported by the review (static reading), not re-verified.

## Suggested direction (Proposed, not agreed)

One helper next to `write_atomic` that reads a JSON object, rejects a newer schema, sets the file aside and encodes with the version. Keep the per-file adapters (`keep_valid_fields`, `sanitised`) and the debounce as they are. Fight history and personal bests adopt it when their own fixes land.

## How to verify

Settings, to-dos and mobs still pass their existing corrupt-file and newer-version tests. The set-aside file name and version field are produced by one function. A new store file would need to write only its own adapter.

## Related

TASK-25.3 (fight history wiped when unreadable) and TASK-25.4 (personal bests) are the correctness fixes that should adopt this helper; this ticket is the clean-up of the three existing copies and is not a precondition for them.

Found by the code review of commit 7a48824 (2026-10-08). Line numbers refer to that commit; core capture, parser, events, meter and data_storage changed afterwards, so re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reading a JSON store (missing, unreadable, newer-schema), setting a bad file aside and writing the versioned file are implemented once and used by settings, to-dos and mobs.
- [ ] #2 Settings, to-dos and mobs pass their existing corrupt-file and newer-version tests unchanged.
- [ ] #3 The save debounce of settings and to-dos is left as it is.
<!-- AC:END -->

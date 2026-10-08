---
id: TASK-26.6
title: Rename the recording format to .kiskrec and keep reading .aetherpc
status: To Do
assignee: []
created_date: '2026-10-08 14:50'
labels:
  - improvement
dependencies: []
parent_task_id: TASK-26
priority: low
type: enhancement
ordinal: 80000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Intent

Give packet recordings the product's own name, and keep every `.aetherpc` file that players already have readable now and after any future format change.

## Agreed (user, 2026-10-08)

- Keep the recording format as it is: a small binary format, not JSONL and not pcap.
- New recordings get a new magic and the extension `.kiskrec`.
- The reader keeps accepting `AETHERPC` / `.aetherpc` files. The user's reason: if we later change how recordings are handled, the `.aetherpc` files of anyone using the meter must not break.

## What exists (commit 7a48824)

- **Format.** `crates/core/src/capture/recorder.rs` defines it. A 12-byte header: magic `AETHERPC` (8 bytes), u16 version (1), u16 reserved. Then one record per captured packet: f64 `captured_at`, source and destination IPv4, ports, u32 TCP sequence, u32 payload length, then the TCP payload. Little-endian. A truncated last record counts as the end of the file. Payloads over 1 MiB are refused when reading.
- **Writers.** Session recordings (started by hand, capped at 512 MiB). Automatic `auto-*` recordings. Rolling `rolling-*` recordings (`capture/rolling_recorder.rs`), pruned to 2 GiB.
- **Readers.**
  - `replay_recording`: app `--replay`, and `crates/core/examples/replay.rs`.
  - packet-explorer: `crates/packet-explorer/src/args.rs` resolves a path, a folder, or part of a name inside the recordings folder.
  - The recordings list in Settings: `capture::recorder::list_recordings`, and `ROLLING_FILE_PREFIX` for rolling files.
- **Probes.** `crates/app/src/capture_settings.rs` (~596-599, verify) shows `.aetherpc` names in Settings captures.
- **Docs.** AGENTS.md (commands), README.md, docs/GPUI_MIGRATION.md and FORK.md name `.aetherpc`.

## Derived

- The record layout does not change in this ticket; only the magic and the extension do.
- The new magic stays 8 bytes so the 12-byte header keeps its shape. The exact bytes are the implementer's choice (for example `KISKREC` padded to 8).
- Reading picks the decoder by magic and version together. `AETHERPC` version 1 stays a supported input permanently. A later format version gets a new version number and its own decoder instead of changing what an existing version means.
- Listing, name lookup and rolling pruning cover both extensions. Old `rolling-*.aetherpc` files must still count toward the 2 GiB rolling budget and be pruned. Otherwise they stay on disk forever next to the new files.

## Suggested direction (Proposed, not agreed)

- Commit one tiny `AETHERPC` v1 file (a few packets, no player names) as a test fixture.
- Add a test that decodes it to fixed, known values, so a change that stops old files from reading fails the build.
- This fits with the recorded replay tests proposed in the review: scrubbed real recordings with expected results, used as the safety net for capture refactoring.

## Out of scope

- Renaming the data folder `com.helveticxa.aether` and `aether.log`. That is still open in TASK-26.4.
- Converting old files: they are read as they are.
- pcap export.

## How to verify

- Record a short session: the file is `.kiskrec` and replays.
- Replay an existing `.aetherpc` with `--replay` (a `--features probes` build runs unelevated), with the core `replay` example and with packet-explorer. The results match a build from before the change.
- The fixture test passes, and fails if the `AETHERPC` branch is removed.

## Related

- TASK-23: Kisk rename.
- TASK-26.4: the decision record for persisted identifiers. This item is decided.
- TASK-25.7: recordings move off the dispatcher thread; same files, so coordinate.
- TASK-17: session recordings, Done.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 New recordings (session, automatic and rolling) are written as .kiskrec with the new magic
- [ ] #2 An existing .aetherpc recording replays in the app, the core replay example and packet-explorer with the same results as before the change
- [ ] #3 Recording lists, packet-explorer name lookup and rolling pruning include both .kiskrec and .aetherpc files
- [ ] #4 A test decoding a fixed AETHERPC v1 file fails if old recordings stop being readable
<!-- AC:END -->

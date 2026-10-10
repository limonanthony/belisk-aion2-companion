---
id: TASK-26.3
title: Replace upstream identity in metadata and non-persisted names
status: In Progress
assignee: []
created_date: '2026-10-08 12:47'
updated_date: '2026-10-09 22:19'
labels:
  - maintenance
dependencies: []
parent_task_id: TASK-26
priority: low
type: chore
ordinal: 76000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

Package metadata, temporary-folder names, test names, probe strings and a few messages still carry the forks' identity: the package author is the previous maintainer, the core's description says it is "behind Aether", the version is Aether's 2.4.0 while the docs say it was reset to 0.1.0, temp folders and test folders are called `aether…`, and some messages and comments describe things that no longer exist (a tray icon, a file that was renamed). None of this is persisted, so it can be renamed freely, but it confuses readers and makes the project look like a copy.

## Why it happens

The fork was renamed in the user-visible places first; the remaining names were left because they worked. Nothing stores them, so nobody noticed.

## Items (classification: safe rename, nothing persisted)

| Item | Where | Action |
|---|---|---|
| `authors = ["Helveticxa"]` | `crates/core/Cargo.toml` line 6; `crates/app/Cargo.toml` line 6 | Replace (the credit stays in the README and the notices) |
| Core description "behind Aether" | `crates/core/Cargo.toml` line 5 | Reword |
| Version 2.4.0 (Aether's) | `crates/core/Cargo.toml` line 3; `crates/app/Cargo.toml` line 3 | the removed FORK.md (git history) 217-221 says the version was reset to 0.1.0: make the two agree. Needs the user's decision on which version to use |
| Bin `aion-2-dps-meter` and lib `aion_2_dps_meter` | `crates/app/Cargo.toml`; `crates/xtask/src/dist.rs` | Rename at the Kisk cutover (TASK-23), not before |
| `temp_dir().join("aether")` | `crates/app/src/live_session.rs` 28; `crates/app/src/replay.rs` 15; `crates/core/examples/replay.rs` 94 | Rename |
| Test temp directories named `aether-*` | app: `decision_log.rs` 83, `reminders/service.rs` 402, `settings/view.rs` 2861; core: `capture/dispatcher.rs` 612, `capture/processor.rs` 537 and 1390, `capture/recorder.rs` 465, 600 and 625, `engine/meter.rs` 1143-1147 | Rename |
| Font probe strings "Aether 0123…" and "Aether Not A Font" | `crates/app/src/font_probe.rs` 21 and 48 | Rename |
| Log prefix `[dps_meter]` (an old module name) | `crates/core/src/engine/meter.rs` 176 and 300-301 | Rename after checking whether anything reads the logs |
| `chrono_stamp()` (there is no chrono crate; it uses `SystemTime`) | `crates/core/src/engine/meter.rs` 46-47 | Rename |
| Stale expect text "bundled en.json" (the file is `skill_names_en.json`) | `crates/app/src/game_data.rs` 153-154 | Fix the text |
| Stale path comment | `crates/app/src/game_data.rs` line 3 | Fix |
| "tray icon" wording (the Tauri app had a tray; this one does not) | `crates/core/src/preflight.rs` 24-28 | Reword |
| `boss_ids.json` says "inherited from NOIA2's boss list" inside the data file | `crates/core/data/boss_ids.json` line 2 | Provenance belongs in the notices, not in the data; move it there and remove from the file (check that the loader accepts the file without the field first) |

Product-name strings (the "AION 2 DPS Meter" name in titles, text, the zip name) wait for the Kisk rename (TASK-23) so they are not renamed twice. Persisted names (`com.helveticxa.aether`, `aether.log`, `AETHERPC`, settings keys) are NOT in this ticket; they are F4 (TASK-26.4).

## Decision needed (not decided)

Which version number the project uses: Aether's 2.4.0 as it is in the manifests, or the 0.1.0 reset that the removed FORK.md (git history) describes. Options: (a) keep 2.4.0 and fix the removed FORK.md (git history), (b) set 0.1.0 in the manifests, (c) another number the user names. The review has no recommendation beyond "make them agree"; the dist zip and the About page show this number to players.

## Evidence

Reported by the inventory (static reading of commit 7a48824), not re-verified. Paths marked "verify" by the scout: none in this table, but line numbers must be re-located in the current code.

## Suggested direction (Proposed, not agreed)

Do the renames in one pass; the rows without a decision need none. Leave the version and the bin/lib names until their decisions.

## How to verify

A search of `crates/` for "aether" (case-insensitive) finds only: persisted identifiers still kept (F4), credits and notices, history in comments marked as such, and the version decision's outcome; the workspace builds and tests pass; a replay writes its temporary files under the new folder name.

## Related

TASK-23 (Kisk rename: the bin/lib and product-name strings), TASK-26.4 (F4: persisted identifiers), TASK-26.5 (F5: docs), TASK-27 (README statement of lineage), TASK-26 (parent).

Source: fork-leftovers inventory of commit 7a48824 (2026-10-08). Line numbers refer to that commit; re-locate each point in the current code before acting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Package authors, the core description, temp and test folder names, probe strings, the log prefix, chrono_stamp, the stale messages and the tray wording no longer name Aether or the previous maintainer.
- [ ] #2 The core and app manifests and the removed FORK.md (git history) agree on one version number, chosen by the user.
- [ ] #3 boss_ids.json carries no provenance text; the provenance is in the notices.
- [ ] #4 Persisted identifiers and product-name strings are unchanged by this ticket.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: package metadata, temp folders, test names, probe strings and log prefixes now say Belisk; credits, licence text and history documents keep the old names on purpose.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-08 15:31
---
Decided (user, 2026-10-08): the version resets to 1.0.0 when the code goes to GitHub. Today crates/app and crates/core say 2.4.0 (Aether's) and packet-explorer/xtask 0.0.0. Proposed: one number for the workspace ([workspace.package] version = "1.0.0", crates use version.workspace = true) so CI checks a single value. Do the reset before building or testing the updater (TASK-28): a build that still says 2.4.0 would see 1.0.0 as older and never offer it. the removed FORK.md (git history)'s claim of a reset to 0.1.0 is Aether's history and goes with TASK-26.5.
---

created: 2026-10-08 15:33
---
Correction (user, 2026-10-08): the version reset is decided, the number is NOT: it will be either 0.1.0 or 1.0.0, to be determined. Read "1.0.0" in the previous comment as "the reset version". CI release workflow: TASK-30.
---

created: 2026-10-08 15:38
---
Fork leftover found by the review, not yet in this list: `DEFAULT_HIDE_KNOWN_PLAYERS` in crates/core/src/config.rs (~11, ~139-141) is the default of the field `hide_unknown_players` and is named after the opposite flag. Rename it to match the field or inline the value. TASK-8 (hide unknown players on by default, In Progress) changes this same default: do it there or after it.
---
<!-- COMMENTS:END -->

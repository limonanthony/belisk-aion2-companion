---
id: TASK-28.1
title: 'Check for updates, update on click, restart into the new version'
status: In Progress
assignee: []
created_date: '2026-10-08 15:23'
updated_date: '2026-10-09 20:59'
labels:
  - feature
dependencies: []
parent_task_id: TASK-28
type: feature
ordinal: 82000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Intent

Step 1 of the update feature. The app finds new releases on GitHub. The player clicks Update to download, verify and stage the release, then clicks Restart to switch to the new version.

## Agreed (user, 2026-10-08)

- When a new release is detected, the app shows an Update button.
- On click, it downloads, unpacks and stages the update. It then shows a Restart button.
- Restart closes the app and starts the new version.
- The app checks at launch, then once an hour while it runs.

## How it works (Proposed, not agreed: the design from the discussion)

**One process for steps 1 and 2.** Idle → Available → Downloading → Verifying → Ready (staged) → Restarting, plus Failed, which falls back to Available for a later retry. Step 2's setting only decides whether Available moves to Downloading on its own.

**Checking:**
- `GET /repos/<owner>/<repo>/releases/latest` with a `User-Agent` header, on a background thread, never on the UI thread.
- Compare `tag_name` with `CARGO_PKG_VERSION`.
- Check at launch, then hourly. Also check after the PC wakes from sleep or hibernation, if the last check was more than an hour ago.
- Stop polling once an update is found or staged.
- When GitHub refuses (403/429), wait until the time it gives (`x-ratelimit-reset` / `retry-after`). Never retry in a loop.
- A "Check now" button in Settings → About, allowed at most once a minute.

**Download and verification:**
- Download the release's lean update archive.
- Verify its SHA-256 and its signature against a public key built into the app. The app runs as Administrator, so a forged release would run as Administrator on every player's PC; the signature is what prevents that, while the hash only catches corrupted downloads.
- On a mismatch, delete the staging folder, install nothing, and report the reason in About.

**Staging.** Unpack into a staging folder next to the exe, on the same volume, so the swap is a rename. If a newer release appears while one is staged, the newer one replaces it.

**Restart order** (it matters):
1. Stop capture, so WinDivert releases `WinDivert64.sys`.
2. Save settings, to-dos and the current fight. The existing quit flushes and `stop_dps_meter` already do this.
3. Release the single-instance lock (`crates/app/src/single_instance.rs`). Otherwise the new exe finds the old one still running and hands over to it (see TASK-12).
4. Rename the running exe to `.old`. Windows allows renaming a running exe, but not overwriting or deleting it.
5. Move the staged files into place.
6. Start the new exe.
7. Exit, through the existing quit path that ends the process.

The new version deletes `.old` once it has started properly.

**Admin prompt.** The user expected a UAC prompt on restart. A process started by an elevated process inherits the elevation, so the new exe will most likely start without one. That is fine either way.

**During a fight.** A restart splits the fight in History, so restarting during one is avoided (the exact behaviour is an open question on the parent).

**UI:**
- An update indicator in the meter footer. Like every card action, it reacts only when the pointer enters or leaves.
- Settings → About shows the current version, the update status, "Check now" and the last failure reason.
- All text goes through `fl!` in English and French.
- Failures never pop up during play.

**HTTP client.** `Cargo.lock` has none today.
- Options: Windows' WinHTTP through the `windows` crate (no new dependency, some unsafe FFI code), or `ureq`.
- The `self_update` crate supports GitHub Releases and swapping the running exe, but it replaces only the binary, and the WinDivert files would still need handling.
- JSON parsing uses `serde_json`, which is already present.

**Capture.** [INFERENCE] The meter's own HTTPS traffic to GitHub uses port 443, which the dispatcher should ignore as web traffic (`is_web_flow`). Verify that before relying on it.

**Downgrades.** An older version that meets a newer `settings.json` sets the file aside and starts from defaults. Today's corrupt-or-newer handling does the same for `todos.json` and `mobs.json`. Rolling back is safe for data, but the player loses settings until the file is copied back. Document this.

## Out of scope

- Step 2: automatic download and throttling.
- The CI release workflow, the lean archive, and creating the signing key (prerequisites, see the parent).
- Silent installation.

## How to verify

Prove these with a test release on a fork or a test repository:
- The update is found at launch.
- Clicking Update stages it.
- A tampered archive is rejected.
- Restart starts the new version with settings, to-dos and history intact.
- `.old` is gone after the next start.

Add unit tests for comparing versions and for the swap order (staging and the rename/replace steps) using a temporary folder. The user checks the real flow in game.

## Related

- TASK-12: single instance.
- TASK-6: closing must end the process.
- TASK-25.29: the single-instance event handle.
- TASK-26.3: version numbering.
- TASK-23: Kisk rename.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 While the app runs, a newer non-prerelease release on GitHub is shown as an available update at launch or within an hour of its publication
- [ ] #2 Clicking Update downloads, verifies and stages the release; a download whose hash or signature does not match is deleted and never installed
- [ ] #3 Clicking Restart closes the app with settings, to-dos and the current fight saved, and the new version starts; the previous exe is removed after the new one has started
- [ ] #4 Without network, or when GitHub refuses requests, the app keeps working, shows no error during play, and checks again later without retrying in a loop
- [ ] #5 All new text exists in English and French
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: built and reviewed; see TASK-28's note.
<!-- SECTION:NOTES:END -->

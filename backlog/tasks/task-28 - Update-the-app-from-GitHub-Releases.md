---
id: TASK-28
title: Update the app from GitHub Releases
status: In Progress
assignee: []
created_date: '2026-10-08 15:23'
updated_date: '2026-10-10 02:28'
labels:
  - feature
dependencies: []
type: feature
ordinal: 81000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Intent

Players learn about new versions inside the app and can update with a click. The app downloads, verifies and stages the new version, and a restart puts it in place. Optionally the download happens on its own, and the player only clicks Restart.

## Agreed (user, 2026-10-08)

- **Source.** Updates come from GitHub Releases. CI builds them on GitHub runners; neither the public repository nor the CI exists yet.
- **Step 1, manual update:**
  1. The app detects a new release and shows an Update button.
  2. The player clicks it, and the app downloads, unpacks and stages the update.
  3. The app shows a Restart button.
  4. The player clicks it; the app closes and starts again in the new version.
- **Step 2, an "auto update" setting.** When it is on, the app downloads, unpacks and stages by itself. The download is throttled so it does not take too much of the network while the player plays. The player then only sees "restart to update". A button or an icon with a hover tooltip are both fine; the user has no preference.
- **Restart.** Installing always waits for the player's click on Restart. Nothing is installed silently.
- **Checking schedule.** At launch, then once an hour while the app runs.

## Children

- TASK-28.1: step 1, check, Update button, download, verify, stage, Restart.
- TASK-28.2: step 2, automatic download and the restart indicator (depends on TASK-28.1).

## Background (from the discussion; facts checked against GitHub's docs)

- **Endpoint.** `GET https://api.github.com/repos/<owner>/<repo>/releases/latest` returns the most recent release that is neither a draft nor a prerelease: https://docs.github.com/en/rest/releases/releases
  - Requests must send a `User-Agent` header.
  - Unauthenticated requests are limited to 60 per hour per IP: https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api
  - With one check per hour, up to 60 PCs behind one IP stay under that limit. With a 5-minute interval, only 5 would.
- **Version.** The app compares the release's `tag_name` with its own version (`CARGO_PKG_VERSION`).
- **Prereleases.** `/latest` never returns them, so betas can be published without reaching normal players.

## Prerequisites (not in this ticket)

- **Version numbering.** It must be decided before the first public release: Cargo still says Aether's 2.4.0 while the removed FORK.md (git history) claims a reset to 0.1.0 (TASK-26.3). CI must check that the git tag equals the Cargo version.
- **Public repository and CI release workflow.** The workflow should attach a lean update archive (exe plus WinDivert files, without `source/`), its SHA-256 and a signature (see step 1). The GPL source obligation is met by the public repository at the release tag and the full dist zip.
- **Repository owner and name.** The app embeds them in its update URL, so they must be known. The Kisk rename (TASK-23) may change the repository name.

## Open questions (not decided)

- Is update checking on by default, with a Settings switch to turn it off?
- Is "auto update" on or off by default?
- Should there be a beta channel? It would read the `/releases` list instead of `/latest`.
- Signing: which scheme (an ed25519 key such as minisign, or Windows Authenticode code signing, which costs money but also removes SmartScreen warnings), and where the private key lives (not in CI).
- Restart during a fight: hide the Restart button until the fight ends, or show "Restart after this fight"?
- The README must say what the app contacts and why. This would be the app's first network call.

## Related

- TASK-23: Kisk rename.
- TASK-26.3: version and metadata.
- TASK-12: single instance.
- TASK-6: closing with × must end the process.
- TASK-27: README.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09: built and reviewed twice for security (not yet tested by the user; no live download possible until the GitHub repository exists). A toolbar icon appears only when an update is available, downloading or ready; Settings › About › Updates shows the status, Check now (once a minute), the automatic download switch (on by default), release notes and errors. Downloads are capped at 1 MB/s and paused during fights. Updates are verified (SHA-256 and an Ed25519 signature) before anything is installed; the restart helper is the installed exe itself, re-verifies and swaps only the exe and the two WinDivert files, with rollback; the running exe is held so it can't be swapped before a restart. Builds without a repository (UPDATE_REPO) or with the development key have updates turned off, and cargo xtask dist refuses the development key. The user still has to generate the real signing key (docs/updates.md).

2026-10-10: v1.0.0's update check crashed at its first HTTPS request (ureq built without its native-tls connector); fixed in v1.0.1 (plus: a crash in the check now shows as a failure and retries hourly). User installed 1.0.1 by hand: About says up to date. Remaining: the real update test, 1.0.1 → 1.0.2 (download, verify, restart into 1.0.2).
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-08 15:31
---
Decided (user, 2026-10-08): once the code is on GitHub, development happens on a `dev` branch and main only receives finished work; a push (merge) to main is the release trigger. The version resets to 1.0.0 (see TASK-26.3). Derived for the CI release workflow: CI runs build and tests on pushes/PRs to dev; on a push to main it releases only when the tag v<Cargo version> does not exist yet. Catch found in the local repository: Aether's tags v0.1.0 to v0.1.15 and v2.0.0 to v2.4.0 exist locally (plus our tauri-final). If they are pushed to the new GitHub repository, our future v2.0.0 to v2.4.0 would collide with them and the 'tag exists' check would skip those releases. Do not push Aether's tags (push branches only, or rename them, e.g. aether-v2.4.0); the user decides, and no tag was changed.
---

created: 2026-10-08 15:33
---
Correction (user, 2026-10-08): the version reset is decided, the number is NOT: it will be either 0.1.0 or 1.0.0, to be determined. Read "1.0.0" in the previous comment as "the reset version". CI release workflow: TASK-30.
---
<!-- COMMENTS:END -->

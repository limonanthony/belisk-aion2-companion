---
id: TASK-30
title: >-
  Build, test and publish releases with GitHub Actions (dev branch, release on
  main)
status: To Do
assignee: []
created_date: '2026-10-08 15:33'
labels:
  - feature
dependencies: []
type: feature
ordinal: 85000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Intent

Once the code is on GitHub, CI on GitHub runners builds and tests every change on `dev`, and publishes a release, with its notes taken from the changelog, when finished work reaches `main`. Players download the builds from GitHub Releases, and the updater (TASK-28) reads them from there.

## Agreed (user, 2026-10-08)

- **Open source.** The project will be put on GitHub as open source, with CI building releases on GitHub runners for people to download.
- **Branches.** Development happens on a `dev` branch, never on `main`. A push (merge) to `main` is the release.
- **Version reset.** The version resets when the code goes to GitHub, either to 0.1.0 or to 1.0.0. Which one is not decided yet.
- **Release notes.** The GitHub release shows that version's changelog section (TASK-29).

## Current state (2026-10-08)

- **No CI at all:** no `.github/` folder, no pinned toolchain (`rust-toolchain.toml`), no `rust-version`.
- **Versions:** `crates/app` and `crates/core` say 2.4.0 (Aether's); `crates/packet-explorer` and `crates/xtask` say 0.0.0.
- **`cargo xtask dist`** already builds the shipped zip: static-CRT release build in `target/x86_64-pc-windows-msvc/release/`, exe checks, licence notices, `source/` from `git archive HEAD` for GPL-3.0. Output is `target/dist/aion-2-dps-meter-<version>-windows-x64.zip`. It refuses a dirty tree or a locked exe.
- **WinDivert:** `WinDivert.dll` and `WinDivert64.sys` are in `crates/app/windivert/` and copied by `build.rs`.
- **Old tags:** the local repository holds Aether's tags `v0.1.0`–`v0.1.15` and `v2.0.0`–`v2.4.0`, plus our `tauri-final`.

## Derived

- **Pushes and PRs to `dev`:** build the workspace and run `cargo test --workspace` on a Windows runner. Failures show on the PR.
- **Pushes to `main`:**
  1. Read the Cargo version.
  2. If the tag `v<version>` already exists, only build and test. A merge without a version bump publishes nothing; otherwise the updater would offer every commit.
  3. If the tag does not exist, release.
- **Before publishing anything, the release job fails when:**
  - tests fail;
  - the top changelog section in `CHANGELOG.md` or `CHANGELOG.fr.md` is not exactly the Cargo version;
  - `## [Unreleased]` still holds entries.
- **Release notes.** That version's section from `CHANGELOG.md`, followed by the section from `CHANGELOG.fr.md` under a "Français" heading.
- **Prereleases.** A version with a prerelease suffix (for example `-beta.1`) is published as a prerelease, so `/releases/latest` and the updater skip it.
- **GPL-3.0 source.** Covered by the public repository at the tag, GitHub's automatic source archives and the dist zip's `source/`.

## Proposed (not agreed)

- **One version number:** `[workspace.package] version = "<reset version>"` in the root `Cargo.toml`, with each crate using `version.workspace = true`, so CI checks one value.
- **Pinned toolchain:** a `rust-toolchain.toml` so CI and local builds use the same Rust.
- **`cargo xtask release-notes <version>`:**
  - It writes the notes file by slicing the two changelogs between their headings. It does not need the app's renderer.
  - It also performs the version/changelog checks above.
  - It has unit tests.
- **The workflow** (`.github/workflows/`):
  - `windows-latest`;
  - a Rust cache action;
  - `cargo test --workspace`, then `cargo xtask dist`;
  - the lean update archive (exe plus WinDivert files, no `source/`) and `SHA256SUMS`, for TASK-28;
  - `gh release create v<version> <dist zip> <update archive> SHA256SUMS --notes-file <notes>` (with `--prerelease` when needed). The `gh` CLI is preinstalled on runners. The job needs `contents: write`.
- **Protect `main`:** changes arrive only by a PR from `dev` with CI green.
- **Document the flow** in AGENTS.md (branches, version bump, the changelog rename at release) and in the README's build section.

## Open questions (not decided)

- **Version:** reset to 0.1.0 or to 1.0.0? Do it before building or testing the updater: a build that still says 2.4.0 would see either version as older and never offer it.
- **Aether's tags:** they must not reach the new repository as they are. If pushed, our future tags in the same range (for example `v2.0.0`–`v2.4.0`) would collide, and the "tag exists" check would skip those releases. Options: push branches only, rename them (for example `aether-v2.4.0`), or delete them locally. Whatever happens to `tauri-final` is also open. No tag has been changed.
- **Signing (shared with TASK-28):** which scheme (an ed25519 key such as minisign, or Authenticode), and where the private key lives. If CI signs, the key sits in GitHub secrets and a compromised workflow could sign. If you sign locally, each release gets a manual step.
- **Prereleases from `dev`:** should `dev` publish prereleases for a beta channel?
- **Repository owner and name:** they may change with the Kisk rename (TASK-23). The updater embeds them.

## Risks to check on the first runs

- [INFERENCE] The first build could take 20–40 minutes (GPUI, static CRT) before the cache helps. Not measured.
- [INFERENCE] The GPUI tests run on GPUI's test platform and should pass on a runner with no screen. Not proven until the first run.
- The `vendor/embed-resource` patch and the WinDivert files are in the repository, so the build needs nothing outside it. Verify on the runner.

## How to verify

- In a test repository or a fork:
  - a PR to `dev` runs the tests;
  - merging to `main` with a new version creates the tag and a release with the zip, the update archive, `SHA256SUMS` and both changelog sections;
  - merging again without a bump creates nothing;
  - a missing French section fails the job before anything is published.
- Unit tests for `xtask release-notes`.

## Related

- TASK-28: the updater reads these releases.
- TASK-29: the changelog files and their parity test.
- TASK-26.3: the version reset.
- TASK-27: README.
- TASK-23: Kisk rename.
- TASK-25.31: dist build licence checks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A push or pull request to dev builds the workspace and runs cargo test --workspace on a Windows runner, and a failure is visible on the pull request
- [ ] #2 A push to main whose Cargo version has no tag yet creates the tag v<version> and a GitHub release with the dist zip and that version's English and French changelog sections as notes
- [ ] #3 A push to main whose version is already tagged publishes nothing
- [ ] #4 The release job publishes nothing when tests fail or when the changelog section for the version is missing in either language
- [ ] #5 A version with a prerelease suffix is published as a prerelease
<!-- AC:END -->

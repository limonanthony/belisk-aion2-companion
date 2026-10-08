---
id: TASK-29
title: >-
  Show the changelog in the app (what's new after an update, full history in
  Settings)
status: To Do
assignee: []
created_date: '2026-10-08 15:30'
updated_date: '2026-10-08 15:33'
labels:
  - feature
dependencies: []
type: feature
ordinal: 84000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Intent

Players see what changed, in their language, inside the app: right after an update, and any time from Settings. One changelog per language in the repository feeds the app, the GitHub release page and the README.

## Agreed (user, 2026-10-08)

- **Source files.** The changelog comes from Markdown files in the repository: `CHANGELOG.md` for English and `CHANGELOG.fr.md` for French. A future language would follow as `CHANGELOG.<code>.md`, with the code of its i18n folder.
- **Display.** The app shows them nicely rendered in GPUI, in the app's own look.
- **Not the whole file:**
  - After an update, only the versions newer than the last one this player ran, up to the current one. This covers skipped versions.
  - Settings → About: the full history, with the newest version expanded and older ones collapsed to their header. More are loaded with "Show older", in a list that draws only what is on screen.

## Current state (2026-10-08)

- `CHANGELOG.md` holds only Aether's own release history (2.4.0 back to 0.1.2, about 640 lines), written for Aether's old updater dialog. None of this project's own changes are in it: the GPUI app, fight report, reminders, to-dos, French.
- Nothing in `crates/` reads it.
- There is no French changelog.

## Derived

- **GPUI has no Markdown renderer.** Standalone `gpui` ships none, and no Markdown parser is in `Cargo.lock`.
  - The app renders a small fixed subset with its own parser: `## [version] - YYYY-MM-DD` headings, a summary paragraph and `- ` bullets. Whether bold, inline code and links are included is open.
  - The files stay valid Markdown, so they read correctly on GitHub.
- **A test enforces the subset.** It parses both files with the app's parser and fails on syntax outside the subset, so a mistake fails `cargo test` instead of rendering badly.
- **The files are embedded at build time** (`include_str!`) and parsed only when the window opens, never at startup or during play.
- **Language.** The app shows the file of the language in force. A missing language falls back to English, like the Fluent strings.
- **Parity test.** Like the i18n parity test, a test fails when the English and French files list different versions or a different number of bullets per version.
- **`last_seen_version`.** It is stored in `settings.json`. A fresh install sets it to the current version and shows no notice. A version whose entry has no bullets does not trigger the notice.
- **Never during a fight.** The "What's new" notice never opens a window or takes focus during a fight. It waits, like the update indicator in TASK-28.

## Proposed (not agreed)

- **Format.**
  - An `## [Unreleased]` section at the top collects entries as changes land. The release step renames it to the version.
  - Entries are written for players, with no internals.
  - Writing the changelog line in both languages becomes part of "done" for any player-visible change (add this to the PM conventions doc and AGENTS.md).
- **Window.** "What's new" opens a secondary window through the existing one-window-per-kind registry, with the same chrome and dragging as History.
- **Size.** It is not a concern now: about 1–2 KB per weekly release. If it ever matters, the build could embed only the last N versions and link to GitHub for the rest.

## Open questions (not decided)

- Move Aether's history out of `CHANGELOG.md` (for example to `docs/history/aether-changelog.md`, credited, not deleted), or keep it below a "Before Kisk" line? Moving it fits TASK-26.5.
- Notes before updating: a "What's new" link to the GitHub release page (simplest), or the notes shown inside the app from the release body that `/releases/latest` already returns?
- The first-start notice: a footer indicator or a card like the reminder card?
- Which inline syntax is supported: bold, inline code, links?

## Out of scope

- The CI release workflow that builds the GitHub release body from the same section (a prerequisite of TASK-28; no ticket yet).
- The updater itself (TASK-28).

## How to verify

- Unit tests:
  - the parser on the subset;
  - the parity test;
  - choosing which versions to show from `last_seen_version` (none on a fresh install; all skipped versions after a jump).
- A GPUI test: About opens the changelog with the newest version expanded.
- `--features probes --capture` screenshots of both views in English and French.
- The user reads the window.

## Related

- TASK-28: updates; its "update available" state links to the notes.
- TASK-26.5: fork-era docs, including Aether's history.
- TASK-23: Kisk rename.
- TASK-27: README.
- TASK-7: translations.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On the first start of a newer version, a non-blocking notice offers "What's new", which lists every version newer than the last one this player ran, in the player's language
- [ ] #2 A fresh install shows no what's-new notice
- [ ] #3 Settings > About opens the full changelog with the newest version expanded, older versions collapsed, and older entries loaded on demand
- [ ] #4 cargo test fails when the English and French changelogs list different versions or bullet counts, or use syntax outside the supported subset
- [ ] #5 The notice never opens a window or takes focus during a fight
<!-- AC:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-08 15:31
---
Decided (user, 2026-10-08): the version resets to 1.0.0 on GitHub, so the first changelog entry is ## [1.0.0]. Open: how a settings.json from a pre-1.0 build (no last_seen_version) is treated: as a fresh install (no notice) or shown the 1.0.0 notes once.
---

created: 2026-10-08 15:33
---
Correction (user, 2026-10-08): the version reset is decided, the number is NOT: it will be either 0.1.0 or 1.0.0, to be determined. Read "1.0.0" in the previous comment as "the reset version". CI release workflow: TASK-30.
---
<!-- COMMENTS:END -->

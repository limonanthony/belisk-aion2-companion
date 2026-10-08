---
id: TASK-27
title: Say up front that this project is built from Aether and NOIA2
status: To Do
assignee: []
created_date: '2026-10-08 12:47'
updated_date: '2026-10-08 14:48'
labels:
  - documentation
dependencies: []
priority: medium
type: docs
ordinal: 79000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Intent

The README should say clearly, near the top, that this project was built from the projects it forked. Today that fact is only in a Credits section at the bottom.

## Agreed (user, 2026-10-08)

- README.md must state that the project was built from the project it forked, as a shout-out to them.
- There is no upstream. The project is fully split from Aether and NOIA2. The fork was only a baseline to start from (see TASK-25.11).
- Mention them in the licence material too, if the licence requires it.
- Context, not in scope here: the project will become open source on GitHub, with CI/CD building releases automatically.

## Facts to use

- Lineage: NOIA2 (https://github.com/ZDYoung0519/NOIA2, by zdyoung) was forked by Aether (https://github.com/Helveticxa/Aether-Aion2-DPS-meter-Global, by Helveticxa). This project forked Aether at Aether 2.4.0 (commit `d9f1d98`, 2026-09-24) and has been developed separately since.
- What came from them: the Rust capture pipeline, the packet parsers and the game-data catalogues. The existing Credits section (README.md ~65-75) already says this.
- Both upstream projects are GPL-3.0. `LICENSE` in this repository is the GPL-3.0 text, unchanged from Aether's.

## What GPL-3.0 asks (Derived from the licence text; not legal advice)

- **Same licence.** The project must stay GPL-3.0, because it is a modified version of GPL-3.0 code. `LICENSE` stays as it is; the GPL text itself is never edited to add credits.
- **Keep their notices.** Copyright and licence notices that came from Aether and NOIA2 must be kept intact.
- **Say it was modified (section 5a).** A modified version must carry a prominent notice that it was modified, with a relevant date. A README statement such as "based on Aether 2.4.0 (2026-09-24), modified since" covers this, together with the git history.
- **Ship the source with binaries (section 6).** Distributed builds must come with the corresponding source. `cargo xtask dist` already puts `source/` in the zip, or a written offer with `--no-source`. Once the repository is public, a release built from a tagged commit of that repository also satisfies it.
- **Credits beyond that are not required by the licence.** Credits for other data in the notices (Kuroukihime's GPL names, fonts, icons, WinDivert) stay where they are.

## Derived

- Put the statement in the first screen of the README. The Credits section stays.
- Word it so it does not need rewriting at the Kisk rename (TASK-23). For example, say "this project", or use the name once in the title area.
- Keep it factual: what was forked from what, when, and what code and data came from there.

## Proposed (not agreed)

- Keep the player README in the dist zip (`crates/xtask/dist/README.txt`) and the notices header (`crates/xtask/dist/notices-header.txt`) consistent with the new statement.
- The About page in Settings (`crates/app/src/settings/view.rs` ~1805; keys in `crates/app/i18n/en/aion_2_dps_meter.ftl` 430-434 and the French file 444-448) credits "TK-open-public, taengu, p62003". These names appear in neither the README nor the notices, and nobody knows where they come from. Find out, then credit them consistently or ask the user. Do not add or remove credits on a guess.

## Scope

In scope: README.md, and the two dist files plus the credit check if the proposals are accepted.

Out of scope:
- The Kisk rename (TASK-23).
- Fork cleanup in code and docs (TASK-26).
- Changing the licence.
- The GitHub publication and CI/CD.

## How to verify

- The user reads the first screen of README.md. It names NOIA2 (zdyoung) and Aether (Helveticxa) with links, gives the lineage and the fork date, and says what came from them and that the project is GPL-3.0.
- The Credits section says the same things.
- The user accepts the wording.

## Related

- TASK-23: the name in force.
- TASK-25.11: there is no upstream.
- TASK-26: fork cleanup, especially TASK-26.3 and TASK-26.5. That work must not remove credits or licence notices.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 README.md states within its first screen that the project was built from Aether (Helveticxa), forked from NOIA2 (zdyoung), with links, the fork point (Aether 2.4.0, 2026-09-24) and that it has been modified since
- [ ] #2 The statement names what came from them (capture pipeline, packet parsers, game-data catalogues) and that the project is GPL-3.0
- [ ] #3 The Credits section is still present and consistent with the statement; LICENSE is unchanged
- [ ] #4 The statement would still be correct after the Kisk rename without rewording
<!-- AC:END -->

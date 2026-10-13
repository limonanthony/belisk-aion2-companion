---
id: TASK-25.39
title: Replay real recordings in cargo test before refactoring capture
status: In Progress
assignee: []
created_date: '2026-10-08 15:38'
updated_date: '2026-10-12 15:05'
labels:
  - maintenance
dependencies: []
parent_task_id: TASK-25
priority: medium
type: task
ordinal: 89000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Problem

The capture and parsing code is about to be restructured: TASK-25.9 (stream buffer), TASK-25.13 (parsers return records), TASK-25.14 (shared framing), TASK-25.15 (DataStorage). Nothing proves that the meter still reads real game traffic the same way afterwards.

## Why it happens

- **No recordings in the repository.** Recordings live in the players' and the developer's `%APPDATA%` recordings folder.
- **Tests use hand-written bytes.** The parser and processor tests build packet bytes by hand, with encoders written by the same people as the parsers, so a wrong assumption is built into both and the test still passes. The review named encoder/parser pairs such as `hit_frame` and `Spawn::frame`.
- **Replay already reproduces a session exactly.** `--replay` and `crates/core/examples/replay.rs` re-run a recording through the real pipeline. It just isn't part of `cargo test`.

## Suggested direction (Proposed, not agreed; the review's recommendation)

1. **Pick a few short real recordings** that cover what the meter must get right: a dungeon boss kill with party, a death, heals, damage over time (including a monster's), a summon, a zone change, a character switch.
2. **Trim and scrub them.** Cut each to the needed minutes. Replace character names and any other personal text with placeholders without changing byte lengths, so framing stays valid. packet-explorer already masks text runs for display; a scrub command could reuse that knowledge.
3. **Commit them as test fixtures** (small; check the size budget), each with an expected-results file written from a trusted run: damage per player, DPS, heals, deaths, the boss kill and its time. The expected values come from the current, trusted behaviour or from the game itself, never recomputed by the code under test.
4. **A test in `cargo test`** replays each fixture through the real pipeline and compares the results. Any parser or processor change that alters a result fails with a readable difference.

## Notes

- Fixtures must not contain other players' names. They will be public once the repository is on GitHub (TASK-30).
- Keep the fixtures in the current recording format. TASK-26.6 keeps `.aetherpc` readable and needs its own small `AETHERPC` v1 fixture, which can be one of these.
- When a known bug changes a number on purpose (for example TASK-25.8, DOT ticks), the expected file is updated in the same change, with the reason in the commit.

## How to verify

`cargo test --workspace` runs the replay tests. Deliberately breaking a parser (for example dropping critical hits) makes them fail with the differing numbers.

## Related

- TASK-25.9, TASK-25.13, TASK-25.14, TASK-25.15: refactors this protects; do this first.
- TASK-25.8: DOT ticks.
- TASK-25.32: missing tests.
- TASK-26.6: recording format.
- TASK-30: public repository.

Found by the code review of commit 7a48824 (2026-10-08).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cargo test --workspace replays committed, scrubbed recordings through the real pipeline and compares damage, DPS, heals, deaths and boss kills with expected files
- [ ] #2 The fixtures contain no other players' names
- [ ] #3 Breaking a parser on purpose (for example dropping critical hits) makes the replay tests fail with the differing numbers
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12 (full code review of this date, see TASK-36; fixes tested by agents only, not by the user): Fixed in code. packet-explorer fixture builds crates/core/tests/fixtures/fight-anonymised.belisk (273 KB from a real 70 s session; player names replaced by placeholders, IPs by documentation addresses; the tool refuses to write if an original name remains), and crates/core/tests/replay_fixture.rs replays it through the real pipeline: total damage 2,068,011, 14 players whose rows add up to the session, own player identified; the same fight replayed at two speeds gives the same figures. The expected total is what the core makes of the frames (a regression oracle, not an independent decoder). AC #1 and #2 are met in code; AC #3 was shown for the clock (a wall-clock revert made the speed test fail), not for dropping critical hits. Moved to In Progress; not tested by the user.
<!-- SECTION:NOTES:END -->

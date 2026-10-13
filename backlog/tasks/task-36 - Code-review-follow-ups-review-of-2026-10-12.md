---
id: TASK-36
title: Code review follow-ups (review of 2026-10-12)
status: In Progress
assignee: []
created_date: '2026-10-12 15:03'
labels:
  - maintenance
dependencies: []
priority: high
type: task
ordinal: 100000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An independent full code review of the tree on 2026-10-12 (dev at 345562a, Belisk 1.1.0, plus the uncommitted overlay z-order, card-close and diagnostics work) read the whole code in four slices: core capture and engine, core state and data with packet-explorer, app windows and UI, and app services and tooling. It found 4 high, 13 medium and 12 low findings, no critical ones; the follow-up reviews of the fixes added 1 medium and 3 low residuals (all listed below). The user asked for every finding to be fixed ("Fix them all"). Three agents fixed them, two follow-up reviews checked the fixes, and the four residuals were fixed in a final round.

Reports: review-core, review-ui, review-services (the findings); review-core-fixes and review-app-fixes (the follow-up reviews). The older refactor tickets of TASK-25 that were not findings of this review (25.12, 25.14, 25.15, 25.17, 25.18, 25.23, 25.24, 25.38 and similar) are not part of this work item; each of them carries a dated note with its state.

Every finding below is **fixed in code and tested by agents only**. Nothing is accepted until the user has tried it in game.

## Core capture and engine

- [x] **High** Npcap queued every TCP payload, web flows included, into the 8,192-packet game queue, so a big download could make the game's packets drop. Fix: a game-flow gate (`game_flow_gate.rs`) in front of the queue for both Npcap and WinDivert: web flows never enter it, and other flows enter only from their heartbeat on.
- [x] **High** The session and automatic recordings accepted port 80/443 traffic. Fix: the recorder refuses web flows, as the rolling recording already did.
- [x] **Medium** Every hit, damage-over-time and buff packet built a debug log line (with a hex dump) even with debug logging off. Fix: the debug logger takes a closure and builds the text only when debug is on.
- [x] **Medium** Every candidate flow was parsed twice, so one login produced two `CharacterSeen` events. Fix: one assembler per flow; a test checks one identity packet is reported once.
- [x] **Medium** A fight reset left the previous fight's ping on the overlay. Fix: reset clears the ping (the unread ping history is gone).
- [x] **Medium** Hits, heals, deaths and the idle timeout used the PC clock at processing time, so a replayed fight lasted as long as the replay ran. Fix: they use the packet's capture time; a fixture test replays one fight at 50x and 200x and gets the same duration, damage and rows.
- [x] **Medium** The stream buffer was copied at every parse step and silently dropped data at 50 MB without resetting framing. Fix: the assembler owns its buffer and parses in place; on overflow it warns, discards and keeps the chunk that crossed the limit.
- [x] **Medium** The dispatcher wrote an INFO line for every owned spawn and cloned the whole name map on every boss-HP packet. Fix: the chatter is debug-only and the HP parser looks names up without cloning.
- [x] **Low** Tests that could not fail the contract they named (party DPS, bounded combat targets, nickname characters). Fix: replaced by tests with independent oracles.
- [x] **Low** Docs said a 200 ms snapshot where the core default is 100 ms (the app sets 200 ms). Fix: docs and the replay example corrected.
- [x] **Low** `cargo test` replayed no real recording. Fix: an anonymised fixture of a real fight (`crates/core/tests/fixtures/fight-anonymised.belisk`, built by `packet-explorer fixture`, no player names or real IPs) and a replay test; its expected total is what the core makes of the frames, so it guards against parser regressions rather than proving the decoder.

## App windows and UI

- [x] **High** Raising covered overlays one by one scrambled Belisk's own stacking (it followed hash-map order). Fix: a fixed stack is re-asserted in one pass: toolbar, meter, tools, reminder card, soul tracker card on top; Settings is never part of it.
- [x] **High** New overlay windows (toolbar, tools, History, Pets, reminder card, a new soul tracker card) were shown without a raise. Fix: windows are created hidden, then revealed and restacked.
- [x] **Medium** A visible but covered soul tracker or reminder card was not raised on a new gain. Fix: every update brings the card up.
- [x] **Medium** A snapshot whose current target had no mob code (an add the game had not described) dropped the whole meter frame, freezing the meter. Fix: the frame is always drawn; such a target only cannot take the focus.
- [x] **Medium** Settings (and the prompt and What's new windows) redrew everything on every settings change. Fix: they redraw only for a change the open page shows.
- [x] **Low** A test compared a constant to itself instead of testing the reveal. Fix: replaced by real Win32 tests.
- [x] **Low** GPUI log lines were formatted before the rate gate. Fix: a line is built only when written.
- [x] **Low** Results of the Win32 show and raise calls were ignored, and the log could claim a raise that failed. Fix: results are checked, failures are logged as warnings and retried on the next tick.

## App services and tooling

- [x] **Medium** GPUI's errors were logged without where they came from. Fix: empty-target lines carry file and line.
- [x] **Medium** "RefCell already borrowed" at every launch. Fix: Win32 shows and raises are deferred until the GPUI update returns (cause proven from GPUI's code; see TASK-35).
- [x] **Medium** A stalled update download never timed out and held its connection during fights. Fix: a body timeout per request, the connection is dropped while a fight runs and reopened with a `Range` request. Limit: cancel and "automatic download off" take effect after the read in progress, at most the body timeout.
- [x] **Medium** A remembered character can be assumed for a new alt of the same class. Fix: the core has no reliable signal to tell them apart before the game names the character (checked in the logs and recordings), so the rule is kept and every assumption, and every wrong one with how many ticket and pet changes were bound to it, is now logged.
- [x] **Low** The folder move ran before the single-instance check. Fix: the instance is claimed first.
- [x] **Low** Two benign DXGI lines were logged as ERROR at every launch. Fix: those two lines are left out.
- [x] **Low** The log gate lost the last repeat count. Fix: it is written on flush and at quit.
- [x] **Low** Two debounced-save mechanisms (pets and to-dos). Fix: one shared background writer (`background_save.rs`) now saves pets, to-dos and settings.
- [x] **Low** A failed pets save was not retried until the next change. Fix: the shared writer retries every 10 s.
- [x] **Low** The prompt and What's new windows redrew on every settings change. Fix: see the Settings item above.
- [x] Also removed: `stall_resync_delay_ms`, which nothing read after the processor change.

## Found by the follow-up reviews, fixed in the final round

- [x] **Medium (R1)** The once-a-second keep-on-top check was off while "hide everything" is set, so a soul card shown through it could stay under the game. Fix: visible overlays are still restacked.
- [x] **Low (R2)** The once-a-second restack still ran inside a GPUI update. Fix: it is deferred like the other Win32 calls.
- [x] **Low (R3)** The Settings observer cloned all settings on every store tick. Fix: it compares before cloning.
- [x] **Low** The assumed-character INFO line was logged even when nothing was assumed. Fix: logged only when the assumption is newly made.

## Boundaries

- The overlays keep being always on top, and a foreign topmost window (a graphics or chat overlay) never counts as covering them; the stricter reading was not taken.
- No change to the .aetherpc format or to any project identifier.
- Live Npcap capture, a real desktop with the game's borderless window over the stack, and a real launch log were not exercised by agents.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 During a boss fight with adds the game has not described yet, the meter keeps updating (names, boss HP, damage) instead of freezing.
- [ ] #2 The meter and the other windows do not freeze while Steam or a browser downloads a large file.
- [ ] #3 Dragging the Settings opacity slider is smooth, and the preview follows it.
- [ ] #4 Updates still download (check, update on click, restart into the new version), including when a fight starts during the download.
- [ ] #5 Pets, to-dos and settings changed in one session are still there after a restart.
<!-- AC:END -->

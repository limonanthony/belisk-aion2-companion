# Moving from Tauri to GPUI

This repository is a fork of [Aether](https://github.com/Helveticxa/Aether-Aion2-DPS-meter-Global)
(remote `upstream`), forked at `d9f1d98` (v2.4.0). Aether is itself a fork of
[NOIA2](https://github.com/ZDYoung0519/NOIA2). Both are GPL-3.0-only, so this
fork is too: any build shared with others ships with its source.

The goal of this fork is to replace the Tauri + WebView2 frontend with a native
[GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) UI. This document records the assessment and the
plan agreed before starting.

## Summary

- The Rust backend (capture, parsing, DPS engine, storage) carries over with
  little change.
- The frontend is a full rewrite. No React, JS, or CSS code carries over.
- Tauri's platform services (installer, updater, tray, hotkeys, and so on)
  each need a replacement crate or custom code.
- The expected gain is mostly memory, plus some CPU during fights. A visible
  increase in game FPS is not expected.
- The port is narrower than Aether: the streamer features, the updater, the
  installer, Korean, the light theme and the developer screens are dropped.
  See [Scope](#scope).

## Scope

Agreed with the user on 2026-10-03, before any implementation. Anything not
listed under Dropped is ported.

### Kept

- **Meter lifecycle:** Home with start/stop, live status, detected character and
  server, and the option to close the main window after Start.
- **Meter overlay:** full and capsule layouts, fight summary, personal-best
  pace, lock (click-through), the row and column options, scale and opacity,
  mask nicknames, Copy report.
- **PvP overlay, player detail window, fight history, personal bests** (the
  list and reset in Settings included).
- **Between fights:** idle reset and hide-when-idle, with the peek timings.
- **Focus auto-hide** (`aion2_focus`): the overlays follow the game's focus.
- **Keeping our overlays above a borderless game.** The foreground fullscreen
  check in `game_display.rs` and `on_top::raise_all_over`, reduced to the meter
  and PvP overlays. It moves out of `on_top`, which is otherwise dropped. It
  only raises our own windows and never touches the game's.
- **Shortcuts:** `Alt+E` show/hide (or peek), `Alt+Q` reset, `Alt+CapsLock`
  lock, configurable in Settings.
- **Startup gate**, authoritative in Rust, including the Npcap install button.
- **Capture backends:** Npcap first, WinDivert as the fallback. Npcap stays
  optional.
- **Connection status**, the automatic recording toggle, and manual packet
  recording and replay.
- **Tray** (Show window, Quit), single instance, window position memory.
- **Settings** for everything above; **About** with the version, licences,
  storage summary and clear-cache action.
- **Translations: English and French.** App and overlay strings go through one
  Rust i18n setup from the first screen. Game data (about 7,100 skill names and
  8,000 boss and mob names) stays English: there is no French source for it.
- **Dark theme.**

### Dropped

| Feature | Code it removes | Notes |
|---|---|---|
| Always on top: pinning the player's browser windows over the game | `pages/always-on-top.tsx`, `lib/on-top.ts`, `plugins/on_top/` (`mod.rs`, `win32.rs`, `browsers.rs`), `Ctrl+Alt+T/G/H` | `aion2_focus` loses its `on_top::is_managed()` check |
| Live chat pop-ups (YouTube, Twitch) | `plugins/on_top/chat/`, `overlay/chat/` | |
| Fullscreen-mode explanation | `game_display` status page, the once-per-game notification, the button to Windows graphics settings | The raise behaviour above stays |
| Auto-updater | `tauri-plugin-updater`, the update dialog, release notes, the release workflow's manifest | Nothing to replace |
| NSIS installer | `installer-hooks.nsh`, `bundle.resources` | Replaced by a portable zip, see below |
| Korean | `ko.json` locales, Korean skill table | French added instead |
| Light theme, background video and artwork | `bg-dune.mp4`, `background-dune.webp`, the light palette | Dark theme only |
| Developer screens: opcode census, TCP reassembly settings, diagnostics report, live log window | `opcode-census-setting.tsx`, `tcp-reassembly-settings.tsx`, `diagnostics-copy-setting.tsx`, `overlay/log/` | The backend code stays. The log window becomes an "Open log folder" button |
| System notifications | `tauri-plugin-notification`, `show_system_notification` | "Report copied" shows inside the overlay; the notice after closing the main window on Start goes |

### Distribution

A portable zip: the exe with the Administrator manifest embedded, its data, and
`WinDivert.dll` plus `WinDivert64.sys` beside it. One build script produces it,
so there is no second list of runtime files to drift from the first. Npcap
cannot be bundled (its free licence forbids redistribution), so each user
installs it themselves, or runs on the WinDivert fallback.

## Design

The UI gets a new design, not a copy of Aether's. It was agreed with the user
in three mockup rounds on 2026-10-03, and the validated result is the
reference for the implementation:

- `docs/gpui-mockups/approved/`: one HTML page per screen, with every state
  (meter, PvP, detail, history, startup gate, Home, Settings). Open
  `index.html` in a browser; there is no build step.
- `docs/gpui-mockups/approved/tokens.md`: colours, radii, type, spacing,
  motion, interaction patterns, and how each maps to GPUI and GPUI Kit.

Only validated choices go into `approved/`. The other folders under
`docs/gpui-mockups/` are exploration history. The main choices:

- **Meter:** class-coloured bars as soft rectangles (5 px radius) that brighten
  towards the leading edge, with no edge marker; rounded-square rank badges, cream
  for you; a warm near-black card. Under the pointer, the controls take the
  boss gauge's line, so the window never resizes.
- **PvP:** the HP watch list pinned on top, Kill stats and Damage in tabs below.
- **Detail:** a skill table with Hits, Crit and Total; a row opens to the other
  figures.
- **History:** two lines per record; deleting asks again in the button itself
  (Aether deletes without asking).
- **Main window:** a sidebar with Meter and the settings sections; Segoe UI
  Variable for text and Bahnschrift for titles and figures; a live meter
  preview beside the overlay settings. When Npcap is missing, the startup gate
  waits for "Open Aether" so its recommendation gets read.

Not yet checked on Windows: the mockups were reviewed in a Linux browser with
substitute fonts, so selecting Bahnschrift SemiCondensed and tabular figures in
GPUI has to be verified in phase 2.

## What the code looks like

Measured at `d9f1d98`.

| Layer | Size | Tauri dependency | Cost to migrate |
|---|---|---|---|
| `src-tauri/src/dps_meter/`: capture, TCP reassembly, parsing, engine, storage | ~10k lines of Rust | Low. Only `engine/meter.rs`, `storage/data_storage.rs` and `api/commands.rs` touch Tauri, through `AppHandle`, `emit` and `State` | Easy. Replace Tauri events with a channel or GPUI entity updates |
| `src-tauri/src/plugins/`: overlays, always on top, chat, focus tracking, game display, tray, shortcuts | ~7k lines of Rust | Heavy use of Tauri's window API, but much of it is already direct Win32 | Moderate |
| `src/`: React 19 + Radix/shadcn + Tailwind main app, plus six overlay windows in plain JS/CSS (meter, PvP, detail, history, log, chat) | ~16k lines | Everything | Full rewrite |
| Bridge between the two | 93 `#[tauri::command]` handlers, 46 `invoke` calls, ~11 event names | | Goes away. The UI calls Rust directly |

## Tauri services that need replacing

| Today | Replacement |
|---|---|
| Tray icon (`tray-icon` feature) | `tray-icon` crate |
| `tauri-plugin-global-shortcut` | `global-hotkey` crate |
| `tauri-plugin-updater` | Dropped, see [Scope](#scope) |
| NSIS installer (`installer-hooks.nsh`, `bundle.resources`) | Portable zip that ships the WinDivert driver and DLL beside the exe, which keeps the Administrator requirement in its manifest |
| `tauri-plugin-single-instance`, `tauri-plugin-window-state`, `tauri-plugin-opener` | Small crates, or a few dozen lines each. `tauri-plugin-notification` is dropped |

The WinDivert files have gone missing from released installers before, because
the local copy script and the installer's resource list drifted apart (see
`CLAUDE.md`). The zip build script has to be checked for this.

## Window features the overlays need

- Transparent, borderless, always on top, hidden from the taskbar: GPUI has a
  transparent window background and `PopUp`/`Floating` window kinds.
- Click-through and non-focusable windows: the current code calls
  `set_ignore_cursor_events` and `set_focusable` 7 times each. GPUI has no API
  for either, so this needs the raw window handle plus `SetWindowLongPtr`
  (`WS_EX_TRANSPARENT`, `WS_EX_LAYERED`, `WS_EX_NOACTIVATE`). The existing
  `plugins/on_top/win32.rs` already does this kind of work. Not yet verified:
  that GPUI exposes the window handle on Windows.
- Dragging by a custom title area (`startDragging`, `data-tauri-drag-region`):
  GPUI can start a window move from a click area.

## Things the web UI does that GPUI handles poorly

- Background video (`bg-dune.mp4` in `window-frame.tsx`): GPUI has no video
  playback. Dropped together with the background artwork.
- Frosted-glass panel (`backdrop-filter: blur(18px)` in `overlay/detail/style.css`):
  GPUI can only blur the whole window background, not one element.
- Multi-color inline SVG icons: GPUI draws SVGs in a single color. Convert
  multi-color icons to images.
- Translations (i18next, English and Korean): the port uses a Rust i18n crate
  with English and French; Korean is dropped. The overlays currently carry their
  own small i18n module; the rewrite merges them into one.

## Component library: GPUI Kit

[GPUI Kit](https://github.com/longbridge/gpui-kit) is Longbridge's new name for
`gpui-component`. It is Apache-2.0, which is compatible with GPL-3.0.

Where it saves time:

- It is modeled on shadcn (`gpui-component` is the styled layer, `gpui-base` the
  unstyled one) and ships the Lucide icon set. The current UI uses shadcn/Radix
  and `lucide-react`, so most components translate one-for-one.
- It has 75+ components, including buttons, switches, dialogs, dropdowns,
  tooltips, inputs, virtual-scrolling tables and lists, and charts.
- `gpui-kit` pins a GPUI version that is known to work with it, so the app
  depends on one crate instead of tracking GPUI's own changes.
- It has headless UI tests that render real components and simulate clicks and
  keystrokes without opening a window. These cover interaction checks before
  manual testing.

Where it does not help much:

- The overlays (meter, PvP, detail, history, log) have their own look: damage
  bars, animated numbers, class and skill icons. That is custom drawing in plain
  GPUI, using the kit's theme colors so the windows stay consistent.
- Window-level work (click-through, non-focusable windows, tray, hotkeys,
  updater, installer) is outside its scope.

Decision: use `gpui-kit` styled components for the main app and settings, and
plain GPUI for the overlays.

Caveat: GPUI Kit is pre-1.0 (0.6.x to 0.7 at the time of writing). Upgrades can
break things, so upgrade deliberately.

## Performance

Aether has not been measured yet. These are expectations, not results.

Where Tauri costs resources today:

- Memory, the biggest difference. WebView2 runs a browser process, a GPU process,
  helper processes and a renderer per window. Aether can have up to six overlays
  plus the main window. GPUI runs as one native process. A several-fold drop in
  RAM is expected but unmeasured. For scale, AION 2 itself was observed at about
  6.9 GB.
- CPU during fights. By default the meter sends a full snapshot every 100 ms
  (`DEFAULT_DPS_SNAPSHOT_INTERVAL_MS`). It goes out as a global
  `app.emit("dps-snapshot", …)`, so every open window receives it. Each time the
  data is serialized to JSON, sent to each window, parsed again, and the page is
  laid out and redrawn, with animated counters on top. In GPUI the UI reads the
  data directly and only the window that changed redraws.
- GPU. The detail overlay's backdrop blur is redrawn by the browser engine over
  the game. GPUI draws only when something changes.

What does not change:

- Packet capture and parsing are already Rust.
- Drawing an always-on-top transparent window over the game costs about the same
  whatever it is built with, because Windows has to blend it with the game's
  image either way. This is the part most likely to affect game FPS.
- GPUI is not automatically lighter. A running animation redraws at 60–120 fps.
  Code must redraw only when data changes.

Before and after the meter prototype, measure memory and CPU of Aether and its
WebView2 processes from the process list during the same fight, without touching
the game's process.

## Rules carried over from Aether

These still apply. See `CLAUDE.md` and `FORK.md` for the details.

- Never touch the game's process: no `OpenProcess` on it, no `sysinfo` refresh
  of all processes, no injection, input or hooks.
- Packet capture needs Npcap and Administrator rights.
- The startup gate is enforced in Rust (`preflight::passed()`), not in the UI.
  Every path that shows the main window goes through it.
- The app never changes other programs' windows. The one z-order change left
  is raising our own overlays above a borderless game, never the game's.
- No window calls on the meter's threads. Post window work to the UI thread.

Aether's `CLAUDE.md` asks for small diffs against upstream. That stays true for
the core crate, so parser and protocol fixes from Aether and NOIA2 keep merging
cleanly. UI fixes from upstream have to be redone by hand.

## Plan

1. **Done** (branch `phase-1-core-crate`). The backend lives in
   `crates/aether-core` (package `aether-core`, no Tauri dependency, no Cargo
   workspace yet). `src-tauri` depends on it by path and keeps only the command
   bridge (`src/commands.rs`) and `tauri_event_sink.rs`. The core reports
   through `events::MeterEvent` and `trait EventSink`; the logger is
   `aether_core::logger`; `DpsMeter::new` takes the sink, the logger and the app
   data dir. `cargo test` runs 50 tests in the core and `cargo test --lib` 58 in
   `src-tauri` (108 combined). `cargo run --example replay -- <file.aetherpc>`
   replays a recording with no driver and no elevation. Worth doing even if the
   migration stops.
2. GPUI app skeleton plus the meter overlay: click-through, not taking focus,
   dragging, lock, animated numbers. Drive it from a recorded fight
   (`.aetherpc`, replayed by `capture/recorder.rs::replay_recording`, which needs
   no capture driver). This phase shows whether GPUI handles the overlay
   requirements on Windows.
3. The other overlays: PvP, detail, history.
4. Main window, startup gate and settings, with GPUI Kit.
5. Tray, global hotkeys, single instance, window position memory,
   translations (English and French).
6. Portable zip build (WinDivert files, Administrator manifest) and release
   script.
7. Fix what in-game testing finds until the app matches Aether's kept scope.

The GPUI app is built as a separate binary that reaches parity and then replaces
the Tauri app. Running both UIs in one process is not planned: Tauri and GPUI
each want to own the main-thread event loop.

### Branches and history

Agreed on 2026-10-03: the history stays, rather than a clean `main` with only
the needed code copied in.

- `main` keeps the shared history with `upstream`. Parser and protocol fixes
  from Aether and NOIA2 keep arriving through `git merge upstream/main`. Git
  follows renames, so this keeps mostly working after the backend moves into
  its own crate.
- Each phase is a short branch (`phase-1-core-crate`, ...), reviewed and merged
  into `main` after the user agrees.
- The GPUI app is written as new crates. The Tauri app keeps working on `main`
  until the GPUI app covers the kept scope. Then `src/`, the Tauri app and the
  dropped plugins are deleted in one cutover commit, and the commit before it is
  tagged `tauri-final`.
- Line endings: the repository sets `core.autocrlf=true` in `.git/config`, so
  Windows git and WSL git agree. Commits must not turn LF files into CRLF.

## Effort

Rough estimate, to be revised after phase 2:

- Agent working time: about 2–4 days of sessions across the whole project. GPUI
  builds on Windows take minutes each, and each phase gets a separate review.
- Calendar time: about 1–2 weeks if in-game testing happens after each phase,
  longer if test rounds are far apart.

What can move it:

- Phase 2. If click-through, non-focusable overlays do not work cleanly in GPUI
  on Windows, that shows up within the first session.
- How often in-game testing happens. That sets the calendar more than coding
  speed.
- GPUI being pre-1.0. Expect a few workarounds, most likely around transparent
  windows and text rendering.

## Who tests what

- Agent: builds, `cargo test`, GPUI Kit headless UI tests, and replaying a
  recorded fight through the meter.
- User: everything in-game. Overlay click-through, focus never leaving the game,
  staying on top of a borderless-fullscreen game, live capture, looks, and
  running the portable zip on a clean machine.

The agent never launches the game.

## First step

Done. A two-minute automatic recording of one fight on the global servers is at
`%APPDATA%\Belisk\recordings\auto-1791052949892.aetherpc` (kept on the owner's machine),
outside the repository because it contains other players' data. The meter
named the user's character in that session. Phases 1 and 2 start from that
recording.

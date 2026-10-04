# Moving from Tauri to GPUI

This repository is a fork of [Aether](https://github.com/Helveticxa/Aether-Aion2-DPS-meter-Global)
(remote `upstream`), forked at `d9f1d98` (v2.4.0). Aether is itself a fork of
[NOIA2](https://github.com/ZDYoung0519/NOIA2). Both are GPL-3.0-only, so this
fork is too: any build shared with others ships with its source.

> **Status (2026-10-04):** the Tauri app has been removed (tag `tauri-final`);
> see [Cutover](#cutover). Text below about Tauri services and the old frontend
> is the assessment that led here.

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

The executable is `aion-2-dps-meter.exe`, not `Aether.exe` (user request,
2026-10-03). The Tauri app already builds under that name through
`mainBinaryName` in `tauri.conf.json`; the GPUI app's binary uses it too. The
app identifier stays `com.helveticxa.aether`, so settings, history and
recordings stay where they are.

The name shown to users is "AION 2 DPS Meter", or "DPS Meter" where an overlay
has little room; no window says "Aether" (user request, 2026-10-04). About
still credits the projects this is forked from. Internal names (the
`aether-core` crate, the identifier, `aether.log`) stay as they are.

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

## Open-world meter and the mob list

Agreed with the user on 2026-10-04, after the first live test in the open
world. Aether (and so the first GPUI meter) shows only the attackers of the last
mob you hit, so rows came and went with every target switch. The backend already
keeps per-target totals; the change is in what the meter shows.

- **Automatic view.** A Session board (everyone's damage across all targets
  since the last reset or idle reset) in the open world, and the single-target
  board while a boss is engaged (see *Sticky boss focus*). A label in the
  header names the active view; clicking it pins the other one until the next
  reset or the next boss.
- **Boss detection, for everyone.** The bundled boss list, the existing
  "possible boss" HP rule (over 10 million HP), and the user's corrections. A
  mob pinned as a boss or as not a boss is saved and wins over the other two.
  Order: exclusions, then the user's additions, then the bundled list, then the
  HP rule.
- **Mob collection, off by default.** A "Collect mob data" switch (Settings →
  Capture; a `--collect-mobs` launch flag until Settings exists) records mobs
  into `mobs.json` in the app data folder: code, highest HP seen, first
  and last seen, times fought, boss flag, and the name with where it came from
  (bundled list, the user, or a review). The network sends codes, never names,
  so missing names are proposed by an agent from public community data and
  confirmed by the user. Confirmed names can later go into the bundled list.
  The file holds no player names and is safe to share; it never goes into the
  repository. User corrections are saved even with collection off.
- **Toolbar.** The controls move off the gauge line; the new layout makes room
  for the view label. Chosen from designer options.
- **Your row before the game names you.** The game sends your character's id
  and name together, in one packet, at login and zone changes. A meter started
  later cannot tell which row is yours until then, and shows `ID:<n>`. The
  meter says so and suggests changing zone once.

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
   `crates/aether-core` (package `aether-core`, no Tauri dependency; a Cargo
   workspace with `crates/app` since 2a). `src-tauri` depends on it by path and keeps only the command
   bridge (`src/commands.rs`) and `tauri_event_sink.rs`. The core reports
   through `events::MeterEvent` and `trait EventSink`; the logger is
   `aether_core::logger`; `DpsMeter::new` takes the sink, the logger and the app
   data dir. `cargo test` runs 50 tests in the core and `cargo test --lib` 58 in
   `src-tauri` (108 combined). `cargo run --example replay -- <file.aetherpc>`
   replays a recording with no driver and no elevation. Worth doing even if the
   migration stops.
2. **2a and 2b done** (branch `phase-2-meter-overlay`, results in
   [Phase 2a results](#phase-2a-results) and [Phase 2b results](#phase-2b-results)).
   GPUI app skeleton plus the meter overlay: click-through, not taking focus,
   dragging, lock, animated numbers. Drive it from a recorded fight
   (`.aetherpc`, replayed by `capture/recorder.rs::replay_recording`, which needs
   no capture driver). This phase shows whether GPUI handles the overlay
   requirements on Windows.
3. The other overlays: PvP, detail, history. **3a done** (branch
   `phase-3-overlays`: windows, the typing focus exception and the data, with
   placeholder views; see [Phase 3a results](#phase-3a-results-windows-focus-and-data-for-pvp-detail-and-history));
   3b draws them from the approved mockups.
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
  tagged `tauri-final`. **Done early on 2026-10-04**, see [Cutover](#cutover).
- Line endings: the repository sets `core.autocrlf=true` in `.git/config`, so
  Windows git and WSL git agree. Commits must not turn LF files into CRLF.

## Phase 2a results

Branch `phase-2-meter-overlay`, built on phase 1. This phase delivers the
window, the data bridge, the replay driver and a placeholder view; the approved
meter design (2b) only has to draw.

### Versions

- **GPUI:** `gpui-pre` and `gpui-pre-platform`, both `=0.3.7` (a snapshot of
  Zed at `1a28cff`, repackaged on crates.io as the `gpui-pre-*` family). These are
  the exact pins that GPUI Kit 0.7.0 carries: `gpui-component 0.7.0` requires
  `gpui-pre = "=0.3.7"`. The `gpui` crate on crates.io (0.2.2, October 2025) is an
  older line the Kit no longer uses.
- **GPUI Kit:** 0.7.0 (`gpui-kit`, `gpui-component`, `gpui-base`, all 0.7.0). Not
  added yet. In phase 4, replace the two direct GPUI dependencies in
  `crates/app/Cargo.toml` with `gpui-kit` and the tree stays on one GPUI.
- Both are pre-1.0 and exact-pinned, so upgrade them together.

### What is in the repository

- A Cargo workspace at the root: `crates/aether-core` and `crates/app`.
  `src-tauri` is excluded: it keeps its own `Cargo.lock` and `src-tauri/target`.
  The root `/target` is ignored, which Tailwind needs (it scans every
  non-ignored file).
- `aether-core` gains `ReplayPacer` and `DpsMeter::replay_packet_recording_paced`.
  Records carry `captured_at`; a paced replay sleeps until each packet's capture
  time (relative to the first, divided by the speed), so the meter sees a fight
  at the pace it happened. The existing unpaced replay and its Tauri command are
  unchanged. Out-of-order timestamps neither wait nor delay later packets.
- `crates/app`, package `aion-2-dps-meter`:
  `aion-2-dps-meter --replay <file.aetherpc> [--speed <f>] [--exit-after-replay]`.
  `bridge.rs` is the `EventSink`: it only pushes into an unbounded channel, so the
  meter's threads never block and never touch a window. One foreground task
  sleeps on the channel, applies whatever has piled up as one batch to the
  `MeterModel` entity, and calls `cx.notify()` only when something a view draws
  changed (memory figures, history and personal-best events do not redraw). App
  code has no redraw poll and no animation loop. It has two one-shot timers: 1.5 s
  of quiet before `--exit-after-replay` quits, and the placeholder's 5 s lock
  (`LOCK_DURATION`). GPUI's own `VSyncProvider` thread is platform code, see
  Cost. Quitting cancels a running replay before stopping the meter.
  `meter/view.rs` is the placeholder; `overlay.rs` is the window.
- The exe imports `WinDivert.dll` (through `aether-core`), even for a replay.
  Run `pnpm app:windivert:debug` once so the DLL sits beside the exe.
- `--overlay-probe` and `--font-probe` run the checks below. They exist only in a
  build with the off-by-default Cargo feature `probes`
  (`cargo run -p aion-2-dps-meter --features probes -- --overlay-probe`); the
  default binary contains none of that code. The probe sends no mouse input to
  anything.

### Overlay window requirements

Measured on Windows 11, two 2560x1440 monitors at 100%, with the game running
and in the foreground. Foreground, hit-testing and styles were read back with
`GetForegroundWindow`, `WindowFromPoint`, `WM_NCHITTEST` and
`GetWindowLongPtrW`; the only window other than ours that was looked at is the
foreground one, by handle.

| | Requirement | Result | How, and what was observed |
|---|---|---|---|
| 5a | Transparent, borderless, always on top, out of taskbar and Alt+Tab | Works | `WindowKind::PopUp` creates `WS_EX_TOOLWINDOW \| WS_EX_TOPMOST` and no `WS_EX_APPWINDOW`; `WindowBackgroundAppearance::Transparent` is real per-pixel alpha (the game shows through the card and its rounded corners). Read back: ex-style `0x08200188`, no owner, visible. Screenshots: `unlocked.png`, `locked.png` |
| 5b | Never takes focus | Works for creation and style changes; a real click is not tested | `focus: false` shows it with `SW_SHOWNOACTIVATE`; `overlay.rs` then adds `WS_EX_NOACTIVATE`, so clicking it does not activate it. `GetForegroundWindow` was the same before and after opening, after every lock toggle, and GPUI reports the window inactive. GPUI's `WM_MOUSEACTIVATE` handler answers `MA_ACTIVATE`; per the Win32 documentation `WS_EX_NOACTIVATE` is what stops it, and that was not exercised with a real mouse press (it would need synthetic input over the game). **Manual check:** click and drag the overlay in game and confirm the game keeps focus |
| 5c | Lock is click-through, toggled at runtime | Works | `WS_EX_LAYERED \| WS_EX_TRANSPARENT` (plus `SetLayeredWindowAttributes(255)`) set and cleared with `SetWindowLongPtrW` on the UI thread. Read back `0x082801a8` locked, `0x08280188` unlocked. `WindowFromPoint` over the card skips the window when locked and returns it when unlocked. The window keeps drawing after becoming layered. The header button reaches the same path and unlocks itself after 5 s |
| 5d | Drag by the header; hover while unfocused | Works; drag itself not exercised with a real mouse | A div marked `window_control_area(WindowControlArea::Drag)` answers `WM_NCHITTEST` with `HTCAPTION` (read back: 2 over the title area, 1 over the button and rows), so Windows runs the native move loop. A mouse move posted to the unfocused window set the button's hover state and cleared it again. The drag area must be its own element: a button inside it would start a drag instead of a click. **Manual check:** drag the header in game. |
| 5e | HWND from GPUI | Works | `Window` implements `raw_window_handle::HasWindowHandle`; `RawWindowHandle::Win32` gives the `HWND` (`overlay::hwnd`). No private API needed |
| 5f | Per-monitor DPI | Works at 100%; cross-DPI not testable here | GPUI's embedded manifest declares PerMonitorV2 and the window reports it. At 96 dpi the scale is 1.0 and the client is 310 px for 310 logical. Both monitors run at 96 dpi, so a move to a different DPI could not be exercised; the probe does it automatically when it finds one |
| 5g | Fonts | See below | Bahnschrift SemiCondensed cannot be selected; the rest works |
| 5h | Idle and replay cost | See below | |

Other things seen:

- The window rect is 8 px wider on each side and 4 px taller on each side than
  the client area (326x260 for a 310x252 card), and `WM_NCHITTEST` there answers
  client. Unlocked, the overlay swallows clicks in that thin transparent ring.
  Phase 2b removes that frame, see [Phase 2b results](#phase-2b-results).
- GPUI's Windows backend embeds a manifest with `requestedExecutionLevel
  asInvoker` (feature `windows-manifest`, on by default in `gpui-pre-platform`).
  It collided with the Administrator manifest; resolved in
  [Live capture and the manifest](#live-capture-and-the-manifest).
- `Application::run` needs no admin rights and no activation. The overlay never
  calls `cx.activate`.
- The 5 s lock of 2a was a stand-in; 2b replaced it with `Alt+CapsLock`.

### Fonts (5g)

`aion-2-dps-meter --font-probe` shapes the same string in each candidate and
draws a specimen sheet (`specimen.png`). GPUI's `Font` is family, weight, style,
features and fallbacks; there is no width. `gpui-pre-windows` passes
`DWRITE_FONT_STRETCH_NORMAL` for every lookup and matches the name against
DirectWrite font-set family names only.

| Family requested | Result |
|---|---|
| `Bahnschrift`, weight 400 or 600 | Works. The weight axis is honoured (600 draws bold) at normal width |
| `Bahnschrift SemiCondensed`, `Bahnschrift SemiBold SemiCondensed`, `Bahnschrift Condensed` | **Does not work.** The name is not found and GPUI silently falls back to Segoe UI. Loading `C:/Windows/Fonts/bahnschrift.ttf` through `add_fonts` changes nothing |
| `Segoe UI Variable` | Does not resolve (falls back to Segoe UI). The installed families are `Segoe UI Variable Text`, `Display` and `Small` |
| `Segoe UI Variable Text` (400), `Segoe UI Variable Display` (600) | Work |
| `tnum` | Works. Segoe UI Variable Text: "1111111111" is 75.8 px and "0000000000" 107.8 px without it, both 107.8 px with it. Bahnschrift: 66.5 vs 110.4 px without, both 106.5 px with it |

So the approved type holds except for the width: Bahnschrift SemiCondensed is
not reachable with stock GPUI 0.3.7. The ways forward, for a decision: use
Bahnschrift at normal width and accept wider figures; bundle a
semi-condensed font with a licence that allows it; or patch
`gpui-pre-windows` so a family name can carry the width (a vendored patch of
the Kit's pinned GPUI, to maintain on every bump). `Segoe UI Variable Text` must
be named in full in the design.

### Cost (5h)

Release build, `Get-Process -Id <our pid>` once a second, 8 logical cores,
overlay visible. Nothing else was sampled.

| Case | CPU (share of one core) | Private bytes | Working set |
|---|---|---|---|
| Idle: window open, meter not started, no events (46 s) | 1.2 % average, 4.6 % peak | 64.5 MB | 50.7 MB |
| 1.0x replay of the 2-minute recording, 6,731 packets (120 s) | 2.5 % average, 10.8 % peak | 76.3 MB max | 65.2 MB max |
| After the replay, meter running, no events (62 s) | 2.1 % average | 77.1 MB max | 65.2 MB max |

The idle figure is not zero. `gpui-pre-windows` starts a `VSyncProvider` thread
that wakes on every display refresh and calls `RedrawWindow(RDW_INVALIDATE)` on
every window, forever. Nothing in this app requests a frame; that thread is the
floor. [INFERENCE: GPUI draws only when its frame is dirty, so these wake-ups
cost a message round trip but not a render. The CPU sampling resolution is one
15.6 ms tick, so the percentages carry that granularity.] The meter's own
threads add about 1 % of a core. The replay itself is paced: it took 120.5 s.
For scale, the Tauri app was not measured in this phase.

## Phase 2b results

Branch `phase-2-meter-overlay`, on top of 2a. The placeholder view is gone; the
overlay draws the approved meter (`docs/gpui-mockups/approved/meter.html`,
`tokens.md`) in plain GPUI from real `aether-core` data.

### What was built

- `crates/app/src/meter/`: `view.rs` (the card, capsule and summary),
  `model.rs` (events folded into prepared rows; personal bests asked of the
  core once per character and boss), `live.rs` and `summary.rs` (what each
  card shows, worked out once per snapshot), `fight.rs` (when the fight summary
  goes up and comes down, a unit-tested state machine), `format.rs`,
  `motion.rs` (the 240 ms ease-out tween), `settings.rs` (`OverlaySettings`,
  the `aion2.overlay` defaults of `use-settings.ts`; not persisted, no UI) and
  `game_data.rs`.
- Rules ported from `overlay/meter/main.js`: snapshots of a target without a
  mob code are skipped; at most 10 rows and your row is never the one dropped;
  bars are damage over top damage with a 6 % floor; contribution or share;
  combat power grouped, DPS compact; masking covers players and the target;
  personal-best pace after 10 s of a boss fight; the summary is final when the
  boss dies, provisional after 15 s without a snapshot, never for fights under
  15 s, taken down when the fight resumes, the next boss starts, or after a
  minute; Copy writes the battle report (live) or the summary text.
- Controls under the pointer take the gauge line: reset (only with a meter),
  capsule, lock (only when `Alt+CapsLock` is registered) and close (quits:
  the overlay is the only window). Copy sits in the footer as in the mockup,
  and says "Copied" for 1.5 s. History, PvP and Settings come with their
  windows.
- Lock: `hotkey.rs` registers `Alt+CapsLock` with `global-hotkey` 0.7 (the
  version Tauri already uses). Its hidden window gets `WM_HOTKEY` through
  GPUI's message loop; the handler only queues and the toggle runs on the UI
  thread. If registration fails (another Aether holds it) the lock button is
  hidden, since nothing could unlock the overlay.
- The window fits the card: after layout the view measures the card and
  `overlay::fit_client_size` resizes the client area with
  `SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE`. GPUI's `Window::resize` omits
  the last two, so it could activate the overlay over the game. The window is
  exactly the card in every state (card, capsule, summary, unavailable): no
  transparent margin is left to take clicks while unlocked.
- No drop shadow, removed after the in-game test (2026-10-04). The card first
  carried the approved `0 10 28` shadow in 20/12/30 px of transparent room;
  over a bright scene the window edge clipped it into a faint hard-edged dark
  rectangle, and that room took clicks while unlocked (`WM_NCHITTEST`
  returning `HTTRANSPARENT` only passes clicks to windows of the same thread,
  never to the game). The user chose to drop the shadow; `tokens.md`,
  `meter.html` and `overlay.css` now say none for overlay cards.
- No window frame. GPUI creates its pop-up with an overlapped style, so
  Windows gave it a frame (the 2a ring above), and on Windows 11 DWM drew a
  1 px light border with rounded corners around the whole window. The in-game
  test showed that border; `PrintWindow` captures do not.
  `overlay::open` now opens the window hidden, sets `GWL_STYLE` to a plain
  `WS_POPUP`, sets `DWMWA_NCRENDERING_POLICY = DWMNCRP_DISABLED`,
  `DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND` and
  `DWMWA_BORDER_COLOR = DWMWA_COLOR_NONE` (failures ignored, for Windows 10),
  then places and shows it without activation. GPUI calls no
  `DwmExtendFrameIntoClientArea`. Read back off screen: style `0x94000000`
  (`WS_POPUP | WS_VISIBLE | WS_CLIPSIBLINGS`), window 350x165 = client
  350x165, non-client rendering off (0), corner preference 1 (do not round);
  the border colour cannot be read back (`DwmGetWindowAttribute` answers
  "parameter is incorrect" for it).
- Motion: bars, row order and the boss gauge tween over 240 ms; tool-row swap,
  gauge, chips and the capsule opening fade (100, 150, 180 ms) through GPUI's
  one-shot `Animation::new`. Both request frames only until they finish; a
  settled overlay draws nothing. Windows' "Animation effects" setting turns
  all of it off (`cx.set_reduce_motion`; GPUI does not read the setting
  itself).
- Game data compiled into the exe (`include_str!` / `include_bytes!`) from
  where the Tauri app keeps it, until the cutover moves it:
  `src/games/aion2/data/servers.json` and `dungeons.json`,
  `src/i18n/locales/aion2skills/en.json`, `public/aion2/class/*.png`. The exe
  needs none of these paths at run time. Icons are Lucide SVGs in
  `crates/app/assets/icons/` with its licence, compiled in the same way.

### Font

Barlow Semi Condensed, the four static weights 400/500/600/700 from
`google/fonts` (`ofl/barlowsemicondensed`), embedded with `include_bytes!` and
registered through `add_fonts`. Licence: `crates/app/assets/fonts/OFL.txt`.
DirectWrite files these faces under the family **`Barlow`** with a
semi-condensed width, so GPUI must ask for `Barlow`; `Barlow Semi Condensed`
does not resolve. A system-wide Barlow install is untested. Measured with
`--font-probe` at 20 px: "1111111111" vs "0000000000" is 61.80 vs 101.40 px at
500 and 62.60 vs 101.80 px at 600 without `tnum`, and equal with it (99.00 and
101.40 px). The four weights resolve to distinct faces (254.6, 259.8, 264.0,
269.1 px for the sample line). There is no 650, so your name is 600.

### Checking it without showing anything

`--capture <dir>` (feature `probes` only) asks GPUI for a hidden window (GPUI
moves bounds outside every monitor back onto the primary one), moves it past
the left edge of the virtual screen, then shows it without activation. Every
capture first checks with `MonitorFromRect` that no monitor holds any of it,
and hides it and stops otherwise. `PrintWindow(PW_RENDERFULLCONTENT)` returns
the composed card over black.
`--fixtures` draws every state from synthetic snapshots (invented names) and
then reports frames built and CPU used while idle; `--replay <file>` captures
the paced replay every 15 s as it is, under the pointer, locked and as a
capsule, with names masked. Captures go outside the repository. No global
shortcut is registered in this mode. The 2a overlay probe still shows its
window on a monitor and was not run.

Results (2026-10-04): every state matches `meter.html` in layout and size,
within 1–4 px of card height at the same width. Deviations:

- The typeface (Barlow, not Bahnschrift) and your name at 600, not 650.
- Only working controls in the tool row (see above).
- Waiting hint: "Alt+CapsLock locks and unlocks the meter" instead of
  "Alt+E hides the meter · Alt+Q resets it": those shortcuts are phase 5.
- The capsule opens by resizing the window at once and fading the card in
  over 180 ms, rather than growing.
- The percent column is 400 weight, as `meter.html` draws it (tokens.md now
  says the same).
- A long name ellipses while combat power stays visible, rather than the whole
  line ellipsing.
- The summary's best line has two tones (new or first, behind), not the Tauri
  overlay's third "within 5 %" tone.
- No tooltips on the icon buttons: they would need room outside the window,
  which ends at the card.
- No drop shadow (see above).
- System figures (ping, CPU, memory) redraw on their own only while waiting;
  under the pointer on a live card they update with the next redraw.

The recording replays as one player on many short-lived targets with no boss,
so live replay captures show waiting, live, locked and capsule; the summary
states and the pace come from fixtures. A replay has no data directory, so no
personal best exists to pace against.

Idle cost: in the fixture run, 10 s of the waiting card and 10 s of a settled
live card built **0 frames** each (a render counter in the view); the process,
with no meter running, used 1.9 % and 1.4 % of one core in a debug build. The
release figure was not re-measured. The only frame requests in app code are
the running tweens and GPUI's own one-shot fades, which stop when they finish.

Manual checks left for the game: dragging, `Alt+CapsLock`, the pointer
showing and hiding the controls and opening the capsule, Copy into chat, the
card edge on a bright scene, and a monitor at another DPI.

## Live capture and the manifest

Added after the first in-game test of the 2b build, which showed "Waiting for the
first hit" forever: launched without `--replay`, the app created no meter.

- **Live mode** is launching without `--replay` (`live_session.rs`). The meter
  gets the Tauri app's folders: data in `%APPDATA%/com.helveticxa.aether`
  (Tauri 2 `app_data_dir()` = `dirs::data_dir()/<identifier>`), log in
  `%LOCALAPPDATA%/com.helveticxa.aether/logs/aether.log` (`app_log_dir()`), both
  read from `tauri-2.10.3/src/path/desktop.rs`. History, personal bests and
  recordings are therefore shared with the Tauri app. The config is
  `DEFAULTS.aion2.backend` of `use-settings.ts`, which differs from the core's
  defaults in two values: a 200 ms snapshot interval and a 50 ms unknown-packet
  resync delay (a unit test pins the full set). A user's own backend settings live
  in the Tauri webview's local storage, which the GPUI app cannot read, so they
  are not carried over until the GPUI Settings exist. The meter starts with
  `start_dps_meter` (Npcap first, WinDivert fallback) and is stopped on quit.
  The development checks (`--capture`, `--overlay-probe`, `--font-probe`) never
  start live capture.
- **When the meter cannot start** (not elevated, no backend), the overlay shows
  the card "The meter is not running" with the core's error as the reason, status
  "Stopped", and never "Waiting for the first hit". The capsule layout stays a
  full card in that state. `--capture --fixtures` includes it as
  `fixture-unavailable`. The reason is the core's own text, for example "No
  capture backend available; Npcap: ...; WinDivert: ...".
- **The local character** (`MainActorDetected`) now updates the model and redraws:
  the waiting card's header shows the name (masked when masking is on), and a
  held snapshot's "you" marker follows it.
- **Manifest.** `crates/app/build.rs` embeds `windows/admin.manifest.xml`:
  `requireAdministrator`, PerMonitorV2 (`dpiAware true/pm`), Windows 10/11
  compatibility, SegmentHeap, and the Common-Controls 6 dependency, i.e. what
  `gpui.manifest.xml` carried, with the execution level changed. A build with the
  `probes` feature embeds the same file with `asInvoker`, so the development
  checks run from an ordinary terminal. Read back from the executables with
  `FindResource`/`LoadResource` (data-file load, nothing run): release has
  `requireAdministrator`, the probes build `asInvoker`, each with exactly one
  manifest (id 1).
  - GPUI's `gpui-pre-platform` forces its `windows-manifest` feature on, so a
    second `RT_MANIFEST` made the link fail (LNK1123). Features cannot turn it
    off. The root `Cargo.toml` patches `embed-resource` with `vendor/embed-resource`,
    a copy of 3.0.11 whose `compile()` returns early when the calling package is
    `gpui-pre`. Drop the patch when GPUI stops embedding, or when the Kit's pinned
    snapshot changes how it does it.
  - The manifest is linked into every binary of the package, including a bin's
    unit-test harness, which then cannot start unelevated (os error 740). The
    program therefore lives in `src/lib.rs` (`run()`), `src/main.rs` is three
    lines with `test = false`, and the tests run in the library. `cargo test
    --workspace` runs unelevated.
- **Not verified:** the release exe was not run (it asks for elevation and the
  user plays on this machine), so live capture on a real game session is a
  manual check.

## Open-world meter: data and model

The data side of the decisions under *Open-world meter and the mob list*. The
view that draws it is *Layout D* below.

### Session aggregate

`CombatSnapshot.session` (`models/session.rs`, built by `engine/session.rs`
from the per-target overview the snapshot already computes, so it adds no map
clones). Per player: damage, combat time, DPS, share, and combat power; for the
session: total damage, mobs fought and the session clock. Rows are sorted by
damage, honour `hide_unknown_players` and `max_player_count` (5–20; the local
player, found by id or by name, takes the last row when ranked below the cut),
and the shares count everyone, hidden players included. `boss_only` is applied
at ingest, so a Boss-only session holds bosses only. A reset or idle reset
clears it with the rest.

- **Which damage counts.** Damage to mobs. Targets that are named players
  (PvP, a mob hitting a player) are left out; the PvP boards keep them.
- **Targets** are counted per spawn: a pack of thirty is thirty.
- **Combat time.** On each target a player's window runs from their first hit
  to the last hit anyone landed on it, at least 1 s: the span the per-target DPS
  already divides by, so a one-target session reads like that target's board.
  A player's combat time is the length of the *union* of their windows, so two
  mobs fought at once count once and the walk between packs counts not at all.
  Session DPS is `damage / combat time`. The session clock (header time) is the
  union of every target's window.
- Replays compress time (timestamps are taken at replay time), so replayed
  session DPS figures are large; the figures are right for live play.

### Boss rules and the mob catalogue

`mobs/catalog.rs`. `MobCatalog::is_boss(code)` is the only boss decision: the
snapshot's `TargetInfo.is_boss`, the Boss-only filter and the summon parser all
ask it. Order: the user's exclusion, the user's addition, the bundled list, then
the HP rule (HP over 10 million, when `show_possible_boss` is on; the GPUI app
turns it on in live and replay mode). The HP rule's set now survives a reset: it
is a fact about the mob. `set_boss_correction(code, Some(Boss | NotBoss) |
None)` is the API for a correction; the header's "…" menu calls it (see
*Layout D*). Names: the bundled name
first, then a name from `mobs.json` for codes the bundle lacks.

### `mobs.json`

In the app data folder (`%APPDATA%/com.helveticxa.aether/mobs.json`, shared with
the Tauri app, which reads corrections and names but never collects).

```json
{ "version": 1,
  "mobs": { "2100793": { "name": "…", "nameSource": "bundled|user|reviewed",
                          "maxHpSeen": 1234567, "firstSeen": 1790000000,
                          "lastSeen": 1790000900, "timesFought": 3,
                          "bossCorrection": "boss|notBoss" } } }
```

Every field but `timesFought` is optional. Times are epoch seconds. There is
**no zone field**: the core never learns which zone or dungeon a mob was met in,
so there is nothing to record. No player data is stored.

- **What is saved.** User corrections always. Everything else (unknown mobs,
  HP, counts, names) only when `DpsMeterConfig.collect_mobs` is on: the GPUI
  app's `--collect-mobs` flag, live mode only. The default is off.
- **When it is written.** A change sets a flag; a writer thread (`mob-store-writer`)
  writes when the file is dirty and the last write is at least 30 s old, so the
  first change goes out at once and a burst costs one write. A fight ending
  (reset, idle reset, identification reset, stop) is such a change, and
  quitting calls `save_now()`, which writes on the quitting thread. Writes are
  a temp file, `sync_all`, then rename. The packet and snapshot threads only
  take in-memory locks and set the flag; a failed write is logged and retried.
- **Bad files.** A file that does not parse, or has a newer `version`, is
  renamed `mobs.json.bad-<epoch seconds>` and logged, and the meter starts with
  no mob data. It never crashes.

### View mode and the app model

`crates/app/src/meter/board.rs` is the pure state machine (`BoardChoice`):
Auto (Session in the open world, Target while a boss is engaged) or a pin.
Clicking the inactive side pins it; clicking the active side while pinned
returns to Auto. A pin ends on reset, on an emptied meter, and when a boss is
engaged. A pinned Target board with no boss engaged follows the current
target. `MeterModel` exposes `active_board()`, `is_board_pinned()`,
`select_board(side)` (the caller notifies), `session()` and `live()` (rows),
`header_name()`, `elapsed()`, `footer()` (`Session { damage, targets }`,
`TargetPace`, or `TargetPartyDamage`) and `identified()`.

### Sticky boss focus

The game's "current target" is whatever was hit last (by you, or by anyone
when you are not identified), so in a boss fight with adds it jumped between
the boss and trash and the automatic board flapped between Session and Target.
`meter/focus.rs` (`BossFocus`) fixes that. The first snapshot whose current
target is a live boss **that is your fight** engages it, and the Target board,
its gauge, time, pace and the header name then follow *that boss*, from the
snapshot's per-target overview (`by_target_player_stats`, `target_infos`),
whatever is hit meanwhile.

"Your fight" (`Involvement`): once the game has named the local player, a
boss qualifies only if **you hit it** (your row under the boss in
`by_target_player_stats[boss]`) or **it hit you**. The core records every hit
as `by_target_player_stats[target][attacker]`, mobs as attackers included, so
"it hit you" is the boss's id under your own actor id
(`by_target_player_stats[you][boss]`); that is how a tank or healer who only
takes hits still engages it. Being merely the fallback "last target" is not
enough: until you hit something, the core's current target is *anyone's* last
target, which once put the board on a world boss in a PvP zone that the player
never touched. Before identification the meter cannot tell, so any boss target
engages as before; if one was engaged then and, once identified, you have
neither hit it nor been hit by it, it is released ("not your fight"). A
second boss takes over only if it also qualifies. A stranger's boss that dies
gets no summary. The decision is logged with `--debug-log` (reason "you hit
it" or "it hit you"). It is let go when:

- the boss **dies** (the final summary follows, then the automatic board is
  Session again; a dead boss is never engaged again);
- **another boss** is engaged;
- it goes `QUIET_RELEASE` (2 minutes, one constant in `focus.rs`) without a
  single hit on it, which also puts up the provisional "Fight ended" summary
  (it used to appear after 15 s of quiet, while the boss was still around);
  a boss let go this way is engaged again when someone hits it;
- it is gone from the snapshot, or the user ruled it "not a boss";
- the meter is **reset** (button, hotkey, idle reset or character change);
- the local player's **actor id changes**: ids are per-zone handles, so that
  would show a zone change. The code path exists, but the identification
  packet is not re-sent on a zone change in the recorded sessions, so in
  practice a boss that is left behind is let go by the 2-minute quiet rule.
  The core parses no despawn or entity-removed packet, so nothing more
  precise is known. (A re-sent identification with the same id is not a zone
  change.)

A user pin overrides the board but does not release the boss: unpinning goes
back to it. Hits are noticed by the boss's fight length growing; the quiet
limit is armed on the model's timer, since snapshots stop when nothing is hit.

### Whose summary, whose row

- The summary is built for the target the fight tracker named (the boss), not
  the current target, and from the full per-target overview.
- "You" on any board is found by **name** first. An actor id is trusted only on
  a row nobody has named, because ids change between zones and a stale one can
  belong to someone else by now. If the game has not named the local player,
  or they did no damage to the boss, the summary is about the party ("Party
  DPS", player count, no rank, share, crit, skills or personal best) and the
  copied text starts "Party ...". Nobody else is shown as you. (The capsule's
  lead is `Lead::You` or `Lead::Party`; the top row is no longer passed off as
  you.)
- The Target board no longer reads the core's `last_target_all_players_...`
  list, which the core cuts to its player limit before the app can keep your
  row in it; it ranks the full per-target overview and keeps your row in the
  last place if you rank below the cap.
- Known gap: the identification packet (`0x33 0x36`) was seen in one of the
  five recorded sessions, the one where the meter was started before login.
  In the other four it never reaches the parser, in the replays and in the
  live log (no "main actor" line in sessions of up to 24 minutes with several
  connection changes), so a meter started with the game running stays
  unidentified, and "change zone once" does not fix it. Everything above
  degrades to party figures in that case.

### Diagnostic log

`--debug-log` (app launch flag; a Settings switch comes later) turns on the
app's own decision log. It does not touch the core logger's debug level, which
also writes a line per hit and buff on the packet thread and stays off. It
writes, to `aether.log` (live) or
`%TEMP%/aether/gpui-app.log` (replay), lines starting `[meter]`, only when the
decision changes: the board and why (automatic or pinned), boss focus engaged
and released with the reason, whether and how your row is on the Target and
Session boards (not identified, matched by name or by id, below the row cap,
absent), identification and zone changes, summaries and who they are about,
and boss corrections. Lines may hold player names, as the log already does;
the log stays on the machine. With the flag off the cost is one flag check.

### Layout D in the view

`crates/app/src/meter/view.rs` draws approved layout D
(`docs/gpui-mockups/approved/meter.html`, `tokens.md`), replacing the 2b card:

- **Header.** The current target's name on the left (`muted` on the Session
  board, `text` on the Target board, cut with an ellipsis only when it runs out
  of room), the board's clock on the right. Under the pointer the clock
  cross-fades (100 ms) to the tools: reset · capsule · lock · copy · … · close,
  evenly spaced 2 px apart (the extra 3 px before close is gone, after the
  in-game test).
  Copy turns into a check for 1.5 s. Waiting there is nothing to reset or copy.
  The window never resizes on hover.
- **Boards.** Session: rows of name + combat power, quiet DPS with a faint
  "/s", damage in bold, share %. Target: as in 2b, with the gauge line. A board
  change cross-fades in 150 ms and clears the row tweens; the card's height
  follows the board (the gauge adds a line).
- **Footer, always shown.** Session: damage and the target glyph with the
  count. Target: the PB pace once a boss fight is 10 s old and a best exists,
  else the party's damage. Then ping and the fixed-width (100 px) [Session |
  Target] switch; its highlight slides in 120 ms, the pin dot marks an
  override. Locked, the switch shows and has no handlers (the window is
  click-through anyway).
- **Switch tooltip.** Drawn inside the card above the footer after 500 ms of
  rest, fading in over 150 ms; on a one-row board it sits beside the switch.
  No tooltip for the active side while the choice is automatic.
- **Not identified yet.** A quiet line above the footer, "Start before login,
  or relog via character select", until the game names you. The game sends
  the name at login (a zone change does not resend it); returning to
  character select is the lead from another meter, unverified here.
- **Capsule.** Session: your rank, your damage, "Session" and the clock.
  Target: your DPS, the clock and the pace. Until the meter knows which row
  is you (or when you are not on the board) it shows the party's figure
  without a rank badge ("party damage" / "party DPS"), as the summary does.
  Waiting: "DPS Meter · Standing by".
- **Boss correction ("…").** A click opens a menu under the button: "Treat
  <mob> as a boss", "Not a boss", "Automatic", the current one ticked
  (`MeterModel::correction_target`, `set_boss_choice`). Choosing saves through
  `MobCatalog::set_boss_correction`, patches `is_boss` on the snapshot the
  model holds and re-runs the board choice, so marking a mob a boss turns an
  automatic Session board into the Target board at once; the view pin stays
  independent. Without a target with a mob code (or without a meter) the items
  are faint and inert under a note. The menu is drawn in the overlay window;
  when it hangs below the card the window grows to hold it (the same
  `SWP_NOACTIVATE | SWP_NOZORDER` resize) and shrinks back when it closes.
  While it is open an occluding scrim covers the window under the menu: a
  press anywhere else closes the menu and reaches nothing below it (so a
  press on the switch or a row only dismisses), and a second press on "…"
  closes it the same way. An open menu also closes 400 ms after the pointer
  leaves the window; the check runs on every frame, so a pointer that came
  and went between two frames still closes it. The leave is the
  `WM_MOUSELEAVE` Windows sends after GPUI's own
  `TrackMouseEvent(TME_LEAVE)` on our HWND; no hooks.

Verification, off screen only (`--capture <dir> --fixtures`, the
`MonitorFromRect` guard, `PrintWindow`): one fixture per state of the approved
page (`d-01-session` … `d-21-provisional`), the menu on a full card, on a
one-row card (window 118 px for a 95 px card), with no target, and after
"Treat as a boss" (the board switched to Target), plus the scaled and
unavailable cards. Each was compared with the page's own render of the same
state at 1.25 zoom; heights agree within 1–4 px. The paced replay of the
recording was captured with names masked. The idle check built 0 frames on the
waiting, Session and Target cards. The menu's dismissal was checked off
screen by posting mouse messages to our own window only (`menu-check` lines
of the fixture run): a press on a row and on the switch closes the menu and
leaves the board as it was, while the same press on the switch with the menu
closed pins Target; a second press on "…" closes it; with no press it stays
open for 300 ms; one posted `WM_MOUSEMOVE` is handled (GPUI's pointer moves
to it) and GPUI's hover flag is false 100 ms later although no
`WM_MOUSELEAVE` was posted (only that message clears it), and the menu
closes about 420 ms after the move.

Remaining deviations from the page:

- The waiting hint is "Alt+CapsLock locks and unlocks the meter": Alt+E and
  Alt+Q do not exist yet (phase 5).
- Reset only appears with a running meter, so fixtures (no meter) show the
  tools without it.
- The tooltip for the inactive Session side ("Show the session board / Stays
  until reset or the next boss.") is not on the page, which shows only the
  Target side and the pinned side.
- The menu has no drop shadow: it can reach the window's bottom edge, where a
  shadow would be clipped (the reason the card has none).
- The emptied gauge segment does not fade to the track over 200 ms; the drain
  itself is tweened.

### Class shades

`meter/shades.rs`: when several players share a class, each gets a shade of
the class colour (rule and step values in `tokens.md`, class shades). The
model keeps one `ShadeBook` for both boards, keyed by player name (by id while
unnamed), so a friend has the same shade on Session and Target and keeps it
when ranks swap; an emptied meter or a reset clears it. Shades are assigned
when the boards are rebuilt from a snapshot, in board order for players first
seen in the same snapshot, and the row carries the final colour, so drawing
does no colour work. The capsule badge uses your shade; summary skill bars
keep the plain class colour. A sixth player of a class with four shades (or a
fifth ranger or cleric) takes the class colour again, by the cycling rule.

Checked by unit tests (distinct within a class, stable across a rank swap and
a zone's new id, unnamed players by id, reset, cycling, 4.5:1 for white
figures on every added shade over a dark scene) and off screen
(`shades-session`, `shades-target`, `shades-capsule` in the fixture run).

### Not done

No Settings switch for collection and no name-review UI. The Copy report still
describes the target board.

## Phase 3a results: windows, focus and data for PvP, detail and history

Branch `phase-3-overlays`. Everything the three windows need except their
look: the placeholder views (`view.rs` in `src/history/`, `src/detail/`,
`src/pvp/`) are plain lists of the data, so the flow can be seen and tested;
phase 3b replaces their drawing from `docs/gpui-mockups/approved/{pvp,detail,history}.html`
and keeps the models.

### Windows (`src/secondary/`)

- **One window per kind**, in a registry (`registry.rs`, `WindowKind::{History,
  Detail, Pvp}`). Opening a kind that is open **raises it without activating
  it** (`SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE)`); it does not toggle, since
  the meter's "…" menu is the only way in and a window can sit behind others.
  The detail window is also pointed at the new player or board. A window is
  forgotten when its view is released, however it ends (its ×, or the app
  quitting, which closes them all).
- **Same window as the meter's:** `overlay::popup_options` (pop-up, so topmost
  and tool window, transparent, never shown by GPUI) and `overlay::make_overlay`
  (frame removed, `WS_POPUP` plus the DWM attributes, `WS_EX_NOACTIVATE`). The
  window is as large as its card: `overlay::fit_to_children` runs after layout
  and resizes with `SWP_NOACTIVATE | SWP_NOZORDER`. Header drag is
  `WindowControlArea::Drag`, with the × outside it, as on the meter.
- **Placement, first open only (no position memory yet, phase 5):** beside the
  meter, on the monitor's work area (`placement.rs`, tested): detail to its
  right, history below it (both as in the Tauri app), PvP to its left; each
  flips to the opposite side when it does not fit, then is pulled inside the
  work area. The Tauri app also followed the meter when it moved; this does not.
- **Lock (Alt+CapsLock) applies to the meter only.** Checked in the Tauri code
  (`tauri-final`, `plugins/aion2_overlay.rs`): the lock flag was applied to the
  DPS overlay window, and the PvP window was created with
  `set_ignore_cursor_events(false)`.
- **Entry points.** The "…" menu has "Fight history" and "PvP meter" above a
  separator and the boss corrections. A press on a meter row opens the detail
  window: on the Target board, that player on the board's target (the engaged
  boss, else the one being hit); on the Session board, that player's session
  totals. Rows are not drag areas (the header's name area is), so a click and a
  drag do not conflict. A locked meter is click-through, so rows are not
  reachable then. History players open the detail in history mode; PvP rows open
  it too (a damage-taken row: that player on you; a damage-dealt row: you on that
  player), as in Aether.
- **PvP mode** (`DpsMeter::set_pvp_mode`, a core addition that touches no other
  setting) is on from the moment the PvP window opens until it closes.

### Typing: the one focus exception

The PvP add-to-watch field is the only place an overlay takes keyboard focus
(`secondary/typing.rs`, used by `pvp/view.rs`).

1. A press on "+" (a click on our window, which is what lets this process call
   `SetForegroundWindow`) removes `WS_EX_NOACTIVATE` and calls
   `SetForegroundWindow` **on our own window only**, then focuses the field.
   GPUI's `Window::activate_window` is not used: it injects an Alt key press with
   `SendInput`, which is input injection and has no place next to the game.
2. Enter (adds the name), Escape (cancels), the window losing activation, the
   field losing focus, or the pointer being away from the window for 400 ms all
   end the edit: `WS_EX_NOACTIVATE` goes back on and, if the window still holds
   the foreground, it is hidden and shown again without activation.
3. **Nothing else is given focus.** Hiding the foreground window makes Windows
   activate the next window in the z-order (the documented behaviour of
   `ShowWindow(SW_HIDE)`: "hides the window and activates another window"), and
   it skips `WS_EX_NOACTIVATE` windows when it chooses ("the system does not
   bring this window to the foreground when the user minimizes or closes the
   foreground window"), so our meter is not picked. That is the game, or whatever
   the player was using. `SetForegroundWindow` is never called on another
   process's window. The window calls run on the next turn rather than inside a
   GPUI event handler, because `SetWindowPos` and `ShowWindow` send messages back
   into GPUI synchronously.
4. **The request in flight** (added in 3b review). The foreground request runs
   on the turn after the press, so the edit can stop before it is answered.
   `typing::Edit` (Idle, Starting, Editing; unit-tested) tracks it: the request
   is made only while still Starting, and if the edit stopped while it was
   answered (or the view is gone), focus is given back at once. Closing the
   window (×) and quitting mid-edit also give it back (`on_release`,
   `on_app_quit`), with the window handle kept from creation so a failed
   lookup cannot skip it.

**Not verified:** that Windows really hands focus back to the game. An agent
run must not take the user's foreground, so this stays a manual check (type a
name, press Enter, keep playing without clicking the game).

**Text input: a minimal one**, `secondary/text_input.rs`, adapted from GPUI's
own `examples/input.rs` (Apache-2.0) without selections or mouse dragging:
typing, IME composition, caret keys, Ctrl+V, Enter, Escape, a 32-character
limit. GPUI Kit's `Input` was not added now: the Kit is the planned library for
the main window (phase 4), and pulling it in for one 26 px field would put its
dependency tree and theme into the overlay build early and then have to be
restyled to the overlay tokens. When the Kit arrives, the field can be swapped
without touching the model (`PvpModel::add_name`).

### Data, per window

- **History** (`src/history/`): `list.rs` is the pure part, tested: records newest
  first (from `DpsMeter::get_history`), each row with target, local time
  (`when.rs`, Windows time zone rules), main player, named-player count, damage and
  duration; filters by target and by main player with their record counts, most
  frequent first ("Unknown" for no main player); a filter nobody has any more falls
  back to all; "N records" or "n / N records"; a record's players by damage. The
  model deletes one record or all (`DpsMeter::delete_history_record`,
  `delete_all_history`) and reloads; a delete-all sets the "Deleted N records.
  Personal bests are kept." line for four seconds. The window refreshes on
  `MeterEvent::HistoryUpdated`: the pump in `bridge.rs` calls
  `secondary::history_updated`, which reaches the open history model. The two-step
  confirmation is the view's, in 3b.
- **Detail** (`src/detail/`): `data.rs` builds `DetailData` from a `DetailSource`:
  - *Target board*: the player on one target, as Aether. Skills sorted by damage
    with hits, share, min, max, average, the seven hit-type rates (critical,
    back, front, double, perfect, parry, multi-hit), multi-hit damage and its
    "2 hits, 3 hits…" breakdown, and the five spec slots; summary (damage, DPS and
    fight time from the player's own window on the target, hits); the player's
    buffs and the target's (active first, then by coverage, caster name and
    class); identity (name, class, short server, combat power). Live: rebuilt from
    each snapshot, redrawing only when the result differs.
  - *Session board*: the player's skills **summed per skill code across every
    mob** (below), with DPS and combat time from the Session board's own row for
    that player (so the window and the board agree). No target buffs.
  - *History*: the same figures from the saved record, frozen.
  Skill and buff names come from `skill_names_en.json`, icons from
  `assets/game/skills/` (471 files, 9.4 MB, now embedded: `build.rs` lists the
  folder, `game_data::skill_icon` applies Aether's rule: a six-digit id has its
  own icon, any other id uses its first four digits). `format.rs` and
  `game_data.rs` moved from `meter/` to the crate root, since the new windows are
  their second user.
- **PvP** (`src/pvp/`): `data.rs`, tested. The watch list: add and remove by
  name (trimmed, no blanks or repeats), shown in the order added, a name the game
  has not shown as "Not in range"; **your last attacker is pinned on top, is not
  removable, and shows once even when also watched** (Aether's rule). Suggestions
  from the players seen this session (prefix matches first, at most six).
  Kill stats sort by damage, kills, assists or deaths (3b keeps damage and kills
  only, as approved), ties falling back to kills,
  assists, fewer deaths, name. Damage dealt and damage taken come from the
  snapshot's lists. The watch list and stats are read from the meter every 500
  ms and redraw only on a change; Clear empties the tally at once. The watch
  names are not saved (phase 4).

### Session detail: the aggregation and its cost

`engine::session::session_skill_totals(by_target_player_skill_stats, actor, is_mob)`
in the core (additive, tested): one pass over that player's own tables, adding
hits, damage, min and max and every special count per skill code, skipping
targets that are players (PvP damage is not the session's, as on the board). The
snapshot already carries every target's skill tables, so this adds **nothing to
the capture or snapshot path**: the app calls it on each snapshot only while a
detail window is open on a Session source, and the work is at most one hash
lookup per target the core keeps (512) plus that player's skills. The detail
model compares the result with the last one and redraws only when it changed.

### Checked

`cargo test --workspace`: core 93 + 1 ignored (new: session skill totals, history
deletion), app 155 (new: window registry single instance, placement, history list
filters and deletion, PvP watch list, kill stats, suggestions, detail data for all
three sources, skill icons, local time).

Off screen (`--features probes`, `--capture <dir> --fixtures`, invented data;
every window is outside all monitors and `MonitorFromRect` is checked before each
capture; nothing asks Windows for the foreground):

- `history-01-list` … `history-04-empty`, `detail-01-history` … `detail-04-session`,
  `pvp-01-lists` … `pvp-04-cleared`.
- Window style of each: topmost, tool window, non-activating, frameless.
- Single instance: History opened twice leaves two windows in all (meter + history);
  Detail opened again is retargeted, still two.
- Typing, with posted messages to our own window: "Dov" typed, Enter adds the
  name; Escape cancels; the pointer being away ends an edit within 750 ms; the
  window style is non-activating, activatable while editing, and non-activating
  again after.
- Entry points, with posted mouse messages: the menu's two links and a Session row
  open their windows.

`--capture <dir> --replay <file>` also opens the detail window on the recording's
local player on both boards and a history window, printing the figures with names
masked (`replay-detail-target`, `replay-detail-session`, `replay-history`).

### Not done in 3a

The approved drawing (3b); tabs, popovers, the two-step delete confirmation and
the "last attacker" tooltip. Settings, saved watch names and window positions
(phases 4 and 5). Healing and damage taken. Following the meter when it moves.

## Phase 3b results: the approved PvP, detail and history windows

The three placeholder views are replaced by the approved designs
(`docs/gpui-mockups/approved/{pvp,detail,history}.html`, values in `tokens.md`),
in plain GPUI and in the language the meter ships: Barlow Semi Condensed with
tabular figures, no card shadow, the window exactly as large as the card,
frameless, the window's actions at the top right. Content scale and card
opacity are the meter's settings (`OverlaySettings::scale`,
`background_alpha`), as the mockups' controls apply them.

### Shared pieces

- `src/style.rs`: `Scale`, the class-coloured bar (`class_fill`, `track`,
  `bar`) and the health fill. The meter's view uses the same functions now
  instead of its own copies.
- `src/secondary/chrome.rs`: the overlay.css classes as functions — card,
  header, tool and text buttons (quiet, danger, disabled), chips, section rows,
  tabs, table head, rank badge, inputs and selects, popover and menu items,
  empty state, note, footer, a GPUI tooltip in the popover colours, and
  `scroll_list`.
- **Scroll thumb.** GPUI draws none, so `scroll_list` paints the approved 3 px
  thumb from the scroll handle in a `canvas` during paint: scrolling redraws
  it without another render. The views know their row heights, so whether a
  list overflows (and reserves the thumb's 8 px) is decided before layout.
- **Popovers** (history filter dropdown, PvP suggestions) are the window
  root's own children, drawn over the card. `chrome::Anchor` records where the
  select or field is in a `canvas` prepaint and asks for one more frame only
  when it moved; the popover hangs from it. The root fits the window to all of
  its children, so the window grows while a popover reaches below the card
  and shrinks back when it closes, as the meter's menu does (captured: 169 →
  197 → 169 px). No popover shadow: it would be cut at the window edge; the
  meter's menu has none either.
- The text field draws only text and caret; the PvP view draws the approved
  box around it.

### Per window

- **Record times** follow the page (`history/when.rs`, tested at midnight
  and seven days): "Today 21:42", "Yesterday 22:07", the weekday for two to
  six days back ("Mon 19:12"), then "3 Oct 20:42"; the history-mode detail
  line uses the same form. A list left open over midnight keeps the old words
  until it reloads (a fight saved, a delete).
- **History**: every state of the page — empty, records, filter dropdown,
  record opened (its players open the detail in history mode), deleting one
  and all with the confirmation in the button (reverts after 4 s, the record
  concerned gets the pending rim), after deleting all (the note fades in over
  150 ms). The dropdown closes on a press elsewhere (a scrim, as in the meter),
  on a second press on its select, or 400 ms after the pointer leaves. An
  opened record adds its players' height to the list instead of being
  scrolled into a fixed box.
- **Detail**: nothing selected, live, a skill opened to its twelve figures
  (multi-hit breakdown as a tooltip on "Multi dmg"), buffs open, history
  mode, no skill data. The target panel says "Boss buffs" for a boss and
  "Target buffs" otherwise (`DetailData::target_is_boss`). Lists scroll past
  eight skills and six buffs.
- **Detail, Session source** (no mockup): the header line reads "Session ·
  every target since the last reset" where a target would be, the chip says
  Live, the figures and skills add every mob since the reset, and only the
  player buff panel shows — no single target's buffs belong to a session.
- **PvP**: just opened (with the passive-capture note, which the info button
  also toggles), kill stats by damage or kills, damage dealt and taken (a row
  opens the detail; the eye adds the player to the watch list), adding to the
  watch list with suggestions, the long list scrolling. Only Damage and Kills
  sort, as approved; `StatsSort` lost Assists and Deaths. Enter takes the
  suggestion matching the text, else the first suggestion, else the text.
  Dealt bars now scale to the biggest row, as approved (they were full).

### Colours

A player's class colour is their meter shade wherever the meter has drawn
them since the reset (`ShadeBook::known`, read through
`MeterModel::player_colour` / `actor_colour`; nothing is assigned by
looking): live and Session detail skill bars, buff coverage by caster, PvP
damage rows. Anyone the meter has not drawn gets the plain class colour. A
saved fight shades its own players afresh, most damage first
(`history::record_colours`), so a record's player list and its history-mode
detail agree. Watch rows use the health colour, kill stats the neutral bar.

### Deviations from the mockups

- Barlow Semi Condensed instead of Bahnschrift (narrower, as in the meter).
- No popover shadow (see above).
- History's empty line drops "(Alt+Q)": the GPUI app has no reset shortcut
  until phase 5.
- Detail without skill data shows "--" for the fight time: the mockup's
  target time is not known for a player who never hit it.
- PvP's "Last attacker" has no tooltip (none is drawn in the mockup either).

### Checked

`cargo test --workspace`: core 93 + 1 ignored, app 163 (new: `ShadeBook::known`,
dealt bars scaled to the biggest row; after review: the typing edit state, record
times at midnight and seven days, the mob name left unmasked).

Off screen (`--features probes`, `--capture <dir> --fixtures`), each compared
side by side with the approved page's dark-scene tile at the same width:
`history-01-empty` … `history-10-dropdown-grows-window`, `detail-01-empty` …
`detail-08-session-buffs-open`, `pvp-01-just-opened` … `pvp-08-cleared`. The
typing, single-instance and entry-point checks of 3a still pass. `--replay`
writes `replay-detail-target` and `replay-detail-session` for the recording's
local player, names masked.

### Not done in 3b

Settings, saved watch names and window positions (phases 4 and 5). The
frames-per-idle-second probe exists for the meter only; the new windows redraw
on model changes, anchor moves and tweens, but that was read from the code,
not measured.

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

## Cutover

Decided by the user on 2026-10-04, ahead of the original plan: the Tauri app is
gone, since it was never used. Branch `cutover-remove-tauri`, built on `main`
at `90bac82` (phases 1 and 2), and that commit is tagged `tauri-final`.

- **Removed:** `src-tauri/`, `src/`, `public/`, `index.html`, all Node tooling
  (Vite, TypeScript, ESLint, Prettier, shadcn, `package.json`, the pnpm files),
  `app-icon.png`, the Tauri release workflow and the two scripts that served it
  (`copy-windivert-runtime.ps1`, `release-version.mjs`). The workspace no longer
  excludes `src-tauri`.
- **Moved with `git mv`:** compiled-in game data (`servers.json`,
  `dungeons.json`, `skill_names_en.json`) to `crates/app/assets/game/data/`
  (the UI is its only reader; the core's own data stays in
  `crates/aether-core/data/`); class icons, the skill icons (471 files, 9.4 MB,
  needed by the phase 3 detail window) and `profile_power_icon_pc.png` to
  `crates/app/assets/game/`; WinDivert to `crates/app/windivert/`; `icon.ico` to
  `crates/app/windows/`.
- **Mechanism:** `crates/app/build.rs` copies the two WinDivert files beside the
  exe in `target/<profile>/` (and again if they go missing), and the `.rc`
  files embed the icon next to the manifest. No pnpm or PowerShell step.
  Phase 6 packages the same files into the portable zip.
- **Not carried over:** `bossIcon.png`, `logo.png`, the background video and
  image, the Korean strings and the Tauri overlay locale files. They are in
  `tauri-final` if a later phase wants them; buff icons come from the skill set.
- **Upstream merges:** changes to deleted paths are resolved by keeping them
  deleted; core changes follow the phase 1 renames.
- **Plan now:** phases 1 and 2 done, cutover done. Phases 3 (PvP, detail,
  history overlays), 4, 5, 6 and 7 remain. The Tauri-side text elsewhere in
  this document describes the old app and is kept as history.

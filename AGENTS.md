# AION 2 DPS Meter — working notes

A real-time DPS meter for AION 2, aimed at the global servers. A native GPUI
overlay on a Rust capture/parse/DPS core. Fork of
[Aether](https://github.com/Helveticxa/Aether-Aion2-DPS-meter-Global), itself a fork of
[NOIA2](https://github.com/ZDYoung0519/NOIA2) (GPL-3.0). The Tauri + React app
was removed on 2026-10-04; tag `tauri-final` is its last commit.

[FORK.md](./FORK.md) is the record of what diverges from upstream and why (its
Tauri-only sections are history). [docs/GPUI_MIGRATION.md](./docs/GPUI_MIGRATION.md)
holds the scope, plan and per-phase results. Approved designs: `docs/gpui-mockups/approved/`.

## Commands

PowerShell 5.1 on the development machine has no `&&` — chain with `;`. Cargo
runs from the Windows toolchain (`cmd.exe /c "cargo ..."` from WSL).

```
cargo test --workspace            # the quality gate: meter-core (301 + 1 ignored) + the app (454, including the Settings window's, the reminder card's, the to-do window's and the fight report window's keyboard, focus, hover, timing and layout tests in GPUI's test platform, and the English/French key parity test) + xtask (10) + packet-explorer (36), no elevation needed
cargo build -p aion-2-dps-meter   # add --release for the shipped build
cargo run -p aion-2-dps-meter -- --replay <file.aetherpc> [--speed 1.0] [--exit-after-replay]   # replay starts no driver, but the default exe still asks for Administrator; --features probes builds run unelevated
cargo run -p aion-2-dps-meter     # live capture; the exe asks for Administrator, so run it from an ELEVATED terminal. Settings → Capture has "Collect mob data" (records the mobs fought into mobs.json, off by default) and "Decision log" (writes the meter's decisions (board, boss focus, who "you" are) to aether.log as [meter] lines, off by default); both are stored in settings.json, no launch flags
cd crates/core; cargo run --example replay -- <file.aetherpc>
# --features probes adds --overlay-probe, --font-probe and --capture <dir> (--fixtures | --replay <file>) [--language english|french]:
# --capture draws the overlay off every monitor and saves PrintWindow images, safe while someone plays
cargo run -p packet-explorer -- <opcodes|flows|timeline|dump|diff|search|events|periodic|tickets|parity|unnamed-skills> <recording>... [options]   # developer-only packet explorer for .aetherpc recordings (a path, a folder, or part of a name in the recordings folder); `help` lists the options. `unnamed-skills` lists the skill ids the app would show as "Skill <id>" (hits, damage, class) so their names can be added to the bundled data. `tickets [--id <n>]... [--changes-only]` decodes the ticket messages `0B 61` (full snapshot) and `0C 61` (one change, with reason gain/refill/spend) per character (bound to the own-identity `33 36`), with the content name per ticket id and a per-character first/last/changes summary; give recordings in time order. Text runs are masked unless --show-text; never shipped (not reachable from the app, so not in the dist zip)
node scripts/build-npc-names.mjs <kuroukihime mobs.json> [noia2 npc_names.json]   # regenerates npc_names_en.json; plain node, no packages
cargo xtask dist [--allow-dirty] [--no-source [--source-contact "<text>"]]   # builds the portable zip: target/dist/aion-2-dps-meter-<version>-windows-x64.zip (see docs/GPUI_MIGRATION.md, Distribution). --no-source omits source/ for a GPL-3.0 §6(b) written offer in README.txt (zip goes to target/dist/no-source/). Does not run the exe; refuses on a dirty tree (source = `git archive HEAD` / the commit the offer names) or a locked exe
```

`build.rs` copies `crates/app/windivert/*` beside the exe in `target/<profile>/`
(and embeds the manifest and icon), so a fresh `cargo run` finds `WinDivert.dll`.

## Conventions

- **Comments and logs in English**, everywhere.
- Keep core diffs against upstream **small and anchored**; `git fetch upstream && git merge upstream/main` should stay cheap.
- Files use **CRLF** with `core.autocrlf=true`. Scripts that rewrite files must preserve CRLF, or the diff becomes a whole-file rewrite.
- Forward slashes in paths inside Rust strings.
- **User-facing text goes through i18n**: `fl!("area-key", arg = value)` from `crate::i18n`, with the message in both `crates/app/i18n/en/aion_2_dps_meter.ftl` and `fr/…`. No English words in `format!` or string literals that reach a window, a clipboard report or an accessibility label; game names (bosses, dungeons, skills) are data and stay English. The test `i18n/tests.rs` fails when the two files differ in keys or arguments.
- Moving the project directory requires `cargo clean` — Cargo bakes absolute paths into `target`.

## Project management

- Provider: backlog
- Project root: /mnt/c/Users/Anthony/Documents/Projects/Aion2-DPS-Meter
- Backlog directory: backlog
- Project name: AION 2 DPS Meter
- Documentation home: provider:docs

## Layout

|Path|What lives there|
|---|---|
|`crates/core/`|The backend, no UI dependency: `capture/` (capture, TCP reassembly, dispatch, opcode parsing, recorder/replay, census), `engine/` (DPS calculation, meter lifecycle), `region.rs`, `preflight.rs`, `events.rs` (`MeterEvent` + `EventSink`, how the core talks to its host), `report/` (the boss fight report: `events.rs` the hits taken, heals and deaths recorded while playing, `ledger.rs` what is saved with a boss record, `fight_report.rs` the `FightReport` the report window reads), `history.rs`, `logger.rs`, `data/` (bundled JSON read by the core), `examples/replay.rs`|
|`crates/app/`|The GPUI app (package `aion-2-dps-meter`): `src/overlay.rs` (window, Win32 styles, fit-to-card), `src/bridge.rs` (meter events to the UI thread), `src/meter/` (the approved meter view, its model, boss focus, fight summary), `src/secondary/` (what the history, detail, report, PvP, Settings and reminder windows share: one-window-per-kind registry, placement, the typing focus exception, the text field), `src/history/`, `src/detail/`, `src/pvp/` (each a `model.rs`/`data` part and a `view.rs`), `src/report/` (the boss fight report window: `data.rs` what it shows from the core's `FightReport`, `model.rs`, `view.rs` with the player details inside it, `tests.rs`), `src/format.rs`, `src/game_data.rs`, `src/hotkey.rs` (Alt+CapsLock), `src/fonts.rs`, `src/assets.rs`, `src/sample.rs` (the invented boss fight of the Settings preview and the captures), `src/replay.rs`, `src/live_session.rs`, `src/lib.rs` with a three-line `src/main.rs`; behind `probes`: `probe.rs`, `font_probe.rs`, `capture.rs`, `capture_secondary.rs`, `capture_report.rs`, `capture_settings.rs`, `capture_reminders.rs`, `fixtures.rs`|
|`crates/app/src/i18n/`|The one translation setup: `loader.rs` (Fluent loader over the embedded `crates/app/i18n/<lang>/aion_2_dps_meter.ftl`, English fallback), `language.rs` (`Language`, the `LanguageChoice` stored as `general.language`, Windows' UI language), `mod.rs` (`fl!`, `apply`, the `Locale` entity windows observe), `words.rs` (durations, weekdays, months, lists), `tests.rs` (parse, key and argument parity)|
|`crates/app/src/settings/`|`settings.json` (versioned JSON, atomic write, bad file set aside), the `Settings` entity every window observes, `effects.rs` (a change reaches the language, the overlay model, the decision log and the core config at once; the remembered characters go to the core too), `characters.rs` (the characters the game named as the player, kept in `settings.json` so the core can name your row before the game does), `runtime.rs` and `files.rs` (capture status, recordings, personal bests, storage, folders), `view.rs` (the Settings window, opened from the meter's "…" menu, with its keyboard and focus tests) and `widgets.rs` (its own switch, slider, segmented control, select, buttons and focus ring, with AccessKit roles; no GPUI Kit)|
|`crates/app/src/reminders/`|Event reminders: `events.rs` (the timed-event catalogue and rules, from MetaBot), `schedule.rs` (next start on the local clock, DST rules), `planner.rs` (show / wait / idle for a given moment), `service.rs` (the `Reminders` entity: GPUI timer, `ShowReminder` request, `test_reminder`), `settings.rs` (the switches), `sound.rs` (Windows' Chimes sound), `view.rs` (the reminder card: its own non-activating window on the meter)|
|`crates/app/src/todos/`|Daily and weekly to-dos, per character: `catalogue.rs` (built-in rows with stable ids, limits and refills), `data.rs` (what is stored and every change to it; Duty Missions shared by the characters of one server), `resets.rs` (daily at the reset hour, weekly on Wednesday, on the local clock), `amount.rs` (typed amounts plus the refills since, capped), `list.rs` (what a window shows for a character), `copy.rs` (the countdown, plural and note sentences, through i18n), `store.rs` (`todos.json` beside `settings.json`: versioned, atomic write, a bad entry costs only that entry), `service.rs` (the `Todos` entity: character following, minute tick, debounced save), `settings.rs` (reset hour, membership), `view.rs` + `view/` (the to-do window, opened from the meter's "…" menu: rows, add form, amount field, typing focus exception, hover rule; its tests). Settings → To-dos is `settings/view/todos_page.rs`; `--capture --fixtures` shoots it as `todos-*.png` and `settings-25…29-todos*.png`|
|`crates/app/src/clock/`|The PC's local clock shared by reminders and to-dos: `local.rs` (Windows time zone, now), `wall.rs` (wall-clock arithmetic across daylight-saving changeovers), `testing.rs` (test zones)|
|`crates/app/assets/`|`fonts/` (Barlow Semi Condensed, OFL), `icons/` (Lucide SVGs), `game/` (game data used by the UI: `data/` servers, dungeons, skill names; `class/`, `skills/` (embedded by `build.rs`) and `profile_power_icon_pc.png` icons)|
|`crates/app/windivert/`|`WinDivert.dll` and `WinDivert64.sys`, copied beside the exe by `build.rs`|
|`crates/app/windows/`|Administrator and unelevated manifests, `icon.ico`, the `.rc` files|
|`vendor/embed-resource/`|Patched copy so GPUI's own manifest does not collide with ours (see docs/GPUI_MIGRATION.md, "Live capture and the manifest")|
|`scripts/`|`build-npc-names.mjs`, `resize-aion2-skill-icons.py`|
|`crates/xtask/`|`cargo xtask dist` (alias in `.cargo/config.toml`): static-CRT release build into `target/x86_64-pc-windows-msvc/release/`, exe checks, licence notices, the zip; `dist/` holds the player README and the notices header|
|`docs/`|Migration plan, design mockups, upstream's protocol notes (`AION2_PACKET_PROTOCOL_ANALYSIS.zh-CN.md`, Chinese)|

## Things that will bite you

- **Elevation.** The exe's manifest demands Administrator (capture needs it); a
  `probes` build is `asInvoker`. Tests live in the library and run unelevated.
  Never run the release exe from an agent session: the user may be playing.
- **Never touch a game's process.** No `OpenProcess` on another program, no
  `sysinfo` refresh of all processes, no injection, input or hooks. WinDivert
  is opened only when Npcap cannot capture, and unloaded when the last handle
  closes. See FORK.md, *Staying clear of anti-cheat*.
- **WinDivert files must ship.** A release needs `WinDivert.dll` and
  `WinDivert64.sys` beside the exe. The DLL is delay-loaded (`build.rs`:
  `/DELAYLOAD:WinDivert.dll`), so the meter starts and captures with Npcap when
  they are absent; the guard in `windivert_capturer.rs` loads the DLL before
  the first WinDivert call, and without it WinDivert is just unavailable. They
  drifted out of a release once while working for whoever built it.
- **Region profiles are load-bearing.** A server id is used *structurally* to
  locate fields inside player-info packets; the profile is always `Auto`.
- **No blocking in the meter's threads.** The core calls `EventSink::emit` on
  its own threads; the sink must not block. UI work goes through `bridge.rs` to
  the UI thread, never a window call from the meter thread.
- **There is no cloud.** Upstream's Supabase/leaderboard code was removed in
  2.2.0; a merge that brings it back is resolved by dropping it.
- **English only.** Bundled data carries no Chinese: `npc_names_en.json` is
  built by `scripts/build-npc-names.mjs` from Kuroukihime's GPL data, and a test
  fails if a Han character appears in it. Regenerate rather than edit by hand.
- **Upstream merges.** Changes to paths deleted at the cutover (`src/`,
  `src-tauri/`, `public/`, frontend tooling) are resolved by keeping them
  deleted. Core changes follow the phase 1 renames (`src-tauri/src/dps_meter/**`
  became `crates/aether-core/src/**`, then renamed to `crates/core/src/**`).
- **Never run the release or elevated app, or launch the game, in agent work.**
  Visual checks go through `--features probes --capture`, which renders off every monitor.

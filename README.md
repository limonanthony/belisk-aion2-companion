# AION 2 DPS Meter

A real-time DPS meter for AION 2, built for the global servers: a small native
overlay that shows who is dealing damage, live, and summarises boss fights.

> [!IMPORTANT]
> **Pre-release.** Nothing here has been confirmed against a long global
> session yet. Expect rough edges.

## What it is

The meter reads combat data by **passively sniffing network packets**. It does
not read or write game memory, inject code, modify packets, or automate any part
of the game. It is a monitor.

- A floating, click-through overlay with class-coloured bars; Session board for
  the whole area and a Target board that follows a boss until it dies.
- A boss kill summary: your DPS and place, top skills, personal best.
- Finds the game, your character and the server on its own.
- English names for about 8,000 monsters and bosses. Runs fully offline.
- Alt+CapsLock shows or hides the overlay.

## What it never does

Anti-cheat watches what other programs do to the game. This meter is built so
there is nothing to find:

- It never injects code into a game, loads anything into it, or hooks its graphics.
  Switch the game to borderless to see the overlay.
- It never opens a game's process, reads or writes its memory, sends input, or
  automates anything.
- Packets are copied, never touched: Npcap, or WinDivert in sniff and
  receive-only mode. The WinDivert driver is loaded only when Npcap cannot
  capture, and unloaded when the meter is done with it.

None of this is a promise about terms of service. Whether a publisher allows a
third-party tool is its call.

## Requirements

- Windows 10 or 11.
- **Administrator**: packet capture requires it, and the exe asks for it.
- [Npcap](https://npcap.com/#download) is recommended. Without it the meter falls
  back to the bundled WinDivert driver (`WinDivert.dll` and `WinDivert64.sys`
  must stay beside the exe).
- Start the meter before you log in to the game, so it can find your character.

## Run it

Build from source with the MSVC Rust toolchain:

```
cargo run --release -p aion-2-dps-meter     # from an elevated terminal
cargo run -p aion-2-dps-meter -- --replay <file.aetherpc>   # replay a recording
cargo test --workspace
```

`--collect-mobs` records the mobs fought into `mobs.json`; `--debug-log` writes
the meter's decisions to `aether.log`. Both are off by default.
[AGENTS.md](./AGENTS.md) has the layout and the gotchas;
[docs/GPUI_MIGRATION.md](./docs/GPUI_MIGRATION.md) the plan and status;
[FORK.md](./FORK.md) what differs from upstream and why.

## Credits

- Forked from **[Aether](https://github.com/Helveticxa/Aether-Aion2-DPS-meter-Global)**
  by Helveticxa, itself a fork of **[NOIA2](https://github.com/ZDYoung0519/NOIA2)** by
  [zdyoung](https://github.com/ZDYoung0519): the Rust capture pipeline, packet
  parsers and game-data catalogues come from there.
- English monster and boss names, the Fighter's skill names and the server codes
  come from **[Kuroukihime/AIon2-Dps-Meter](https://github.com/Kuroukihime/AIon2-Dps-Meter)**
  (GPL-3.0), merged by `scripts/build-npc-names.mjs`.
- UI: [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui); font
  Barlow Semi Condensed (OFL); icons from Lucide.

## Disclaimer

This tool reads network traffic on your own machine. It does not inject code,
modify memory, or alter game traffic, and it uploads nothing. It is still
third-party software: use it at your own discretion and in accordance with the
game's terms of service.

## License

[GPL-3.0-only](./LICENSE). Any distributed build must ship its source under the
same terms.

# Fork notes

This file records how the code inherited from NOIA2 by zdyoung (through Aether by
Helveticxa) was changed, and why. Both are only the baseline; nothing is merged
from them.

Licensed **GPL-3.0-only**, like both. Any distributed build must ship its
source under the same terms.

> **History notice.** The Tauri + React app was removed on 2026-10-04; tag
> `tauri-final` is its last commit and git keeps everything it did. Its sections
> here are cut down to short notes; see [Cutover](#cutover-tauri-removed) at the end.
> The capture, parsing, engine and anti-cheat sections still describe the core.


## Why this base

Four AION2 meters were evaluated before forking:

| Project | Verdict |
|---|---|
| [TK-open-public/Aion2-Dps-Meter](https://github.com/TK-open-public/Aion2-Dps-Meter) | MIT but **discontinued 19 June** and deliberately incomplete -- `.gitignore` withholds `resources/json/` (skills, mobs, buffs), the `/addon/` module, and the frontend's `constants/`. Does not build or run as published. Useful only as a licence-safe protocol reference. |
| [p62003/aletheia_AION2_DPS_Meter](https://github.com/p62003/aletheia_AION2_DPS_Meter) | **Not open source** -- docs and screenshots only, zero code, "All Rights Reserved", derivative works forbidden. UX benchmark only. |
| [Kuroukihime/AIon2-Dps-Meter](https://github.com/Kuroukihime/AIon2-Dps-Meter) | C# / .NET 10 / WPF, GPL-3.0, complete game data. Good **cross-reference** -- carries opcodes this base lacks (`PLAYER_STATS 0x49,0x36`, `ENTITY_DEATH`). |
| **NOIA2** (this base) | Rust + Tauri 2 + React 19, GPL-3.0, complete game data, actively developed. |

The deciding factor for a global-server target: NOIA2 **does not hardcode a server
address**. It scans every network interface for the heartbeat magic
`[0x0E, 0x00, 0x36]` and infers the device and port from whichever interface
answers (`crates/core/src/capture/capturer.rs`, `inspect_device_for_magic`).
The Kotlin project, by contrast, pins the Korean server (`206.127.156.0/24:13328`)
in a properties file. Auto-discovery should carry to the global servers unchanged.

Opcodes are also identical across the Korean and Taiwanese clients -- verified by
comparing all three open implementations -- which is good evidence they will hold
on global too. Still to be confirmed against a real global capture.

## Changes to the inherited code

### WinDivert files, and the startup gate

The original installer listed only `WinDivert.dll` as a resource, so every
installed copy had no driver file and reported `WinDivert64.sys was not found`
forever. The code worked around it by downloading the `.sys` from a cloud
bucket with no integrity check. That download is gone: the driver is bundled
beside the DLL (`crates/app/windivert/`, copied beside the exe by `build.rs` and
shipped in the zip).

For the record, the vendored binaries are **byte-identical** to the official
`WinDivert-2.2.2-A.zip` from `basil00/WinDivert` (sha256
`8da085…ddc2` for the driver). The Chinese company on the driver's Authenticode
signature is WinDivert's own signer, not something the inherited code
introduced.

The native build delay-loads `WinDivert.dll`, so the exe starts and captures with
Npcap when the two files are absent; they still ship in the zip for the fallback.

The full hashes, which `cargo xtask dist` checks before packaging:

|File|sha256|
|---|---|
|`WinDivert.dll`|`c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2`|
|`WinDivert64.sys`|`8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2`|

The licence text (LGPL-3.0 or GPL-2.0, your choice) is `crates/app/windivert/LICENSE`,
from the same release.

`crates/core/src/preflight.rs` weighs the capture runtime instead of passing or
failing it:

- **Required vs optional is computed, not fixed.** Capture needs one backend, so
  Npcap is only required while WinDivert cannot stand in for it. An unavailable
  WinDivert beside a working Npcap is shown in grey and never blocks — the old
  screen dressed it in amber next to the word "starting", which is what made a
  perfectly healthy machine look broken.
- **The Npcap probe enumerates adapters** rather than loading `wpcap.dll`, which
  keeps succeeding after the driver service stops.

### Region profiles (Taiwan / Korea / global)

The inherited code hardcoded Taiwan's server catalogue inside the player-name parser:

```rust
fn is_available_server_id(server_id: u32) -> bool {
    (1001..=1021).contains(&server_id) || (2001..=2021).contains(&server_id)
}
```

That value is not cosmetic -- it is used *structurally*, to decide where the
server field sits inside a player-info packet. On any service whose ids fall
outside Taiwan's range, `find_server_id` returns `None` and **no player is ever
named**: a silent, total failure with no error to go on.

`crates/core/src/region.rs` replaces it with per-region profiles:

| Profile | Server-id rule |
|---|---|
| `Auto` (default) | `1001..=1999`, `2001..=2999` |
| `Taiwan` | `1001..=1021`, `2001..=2021` (the bundled catalogue) |
| `Korea`, `Global` | same permissive rule as `Auto` |

The permissive rule is structural, not a shrug: server ids are
`raceId * 1000 + index` (race 1 Elyos, race 2 Asmodian), confirmed by the bundled
Taiwan list. It keeps the disambiguation the parser depends on -- an arbitrary
`u16` still fails -- without assuming how many servers a region runs.

**Detection is deliberately conservative.** It only claims a region on evidence.
Korea is identifiable by its server block (`206.127.156.0/24`, from the MIT
TK-open-public meter); Taiwan's and global's blocks are unknown, so detection
returns "no fingerprint matched" rather than guessing from an id range they may
well share. Settings → Aion 2 → Connection shows the observed server IPs and ids
so an uncatalogued service can be characterised from a real session -- which is
exactly how the global fingerprint gets filled in after Early Access.

**Since 2.2.0 there is no region to pick.** `Auto` parses on every service, so
the other profiles could only ever make things worse; `DpsMeterConfig::normalized`
brings a stored region back to `Auto`, and the picker is gone from Settings.
The profiles stay in `region.rs` because detection still names Korea.

Server names now fall back to `Server <id>` instead of a hardcoded
`"未知服务器"`, so non-Taiwan players stay distinguishable.

## Protocol tooling

Two additions exist for one purpose: the global servers launch once, and being
in game for every parser attempt is the slow part.

### Packet recording and replay

`crates/core/src/capture/recorder.rs` writes raw packets to
`%APPDATA%/<app>/recordings/*.aetherpc`, tapped in the dispatcher **before**
reassembly, so a recording holds exactly what capture produced.

Replay pushes those packets back into the same channel the capturer feeds.
Reassembly, dispatch, parsing and aggregation then run identically to a live
session, because it is the same code path with the same input. Capture once,
iterate offline as many times as it takes.

The format is deliberately plain -- a 12-byte header, then one length-prefixed
record per packet, little-endian. A recording cut short by the app exiting reads
up to the cut rather than failing, which is covered by a test.

Recording is off by default and capped at 512 MB. Replay applies backpressure
when the queue runs deep: `Channel::try_send` drops the packet it is handed when
full, and a silently lossy replay would be worse than a slow one.

### Opcode census

`crates/core/src/capture/census.rs` counts every dispatched packet by
opcode, recording payload size ranges and whether a parser claims it.

This answers the question that decides everything else on day one: are the
global opcodes the ones we already parse? Korea and Taiwan agree on all of them,
which is good evidence -- but a mismatch would present as an empty meter with no
error, so it is worth measuring rather than assuming.

Off by default; it sits on the per-packet path, so disabled costs one relaxed
atomic load.

### Automatic recording (2.2.0)

On a server no fingerprint matches, the meter records the first two minutes of
game traffic by itself, once per meter session (`auto_record_tick` in
`engine/meter.rs`, on the memory-snapshot thread). Files are named `auto-*` and
only the newest five are kept (`recorder::prune_recordings`); a recording
started by hand is never pruned and never interrupted. Today that is every
Taiwan and global session, because neither has a fingerprint yet. Once the
global server block is catalogued, it stops by itself for global.

It exists because launch day is exactly when nobody thinks to press Record in
time. Off switch: Settings → Aion 2 → Connection → Advanced.

### Rolling recording of whole sessions

Settings → Capture → **Record every session** (off by default) keeps the game's
traffic of the whole live session so a game event can be investigated after the
fact (`capture/rolling_recorder.rs`). It is a second `PacketRecorder` beside the
manual / automatic one, so all of them can run at once; files are
`rolling-<unix ms>.aetherpc`, in the same recordings folder and format, and
replay like any other. A new file starts every 30 minutes, and after each start
(and when the meter starts) the oldest `rolling-*` files are deleted until
their total is at most 2 GB. `session-*` and
`auto-*` files are never touched. It records only while the meter captures live
(not during a replay) and closes the file when the switch is turned off or the
meter stops.

Only game packets are written. A packet is kept when its (local port, remote
ip:port) pair is a connection `query_game_connections()` lists for `aion2.exe`
(refreshed once a second on its own thread, never per packet), when it goes to
a server port such a connection has used (so a new connection is not lost
before the next refresh), or when its flow has carried the `0E 00 36`
heartbeat (the fallback when the lookup finds nothing, as with a replayed
source). Packets before the heartbeat on a flow the lookup cannot see are not
kept; the dispatcher ignores them as well.

A rolling file also ends when the next packet would pass the per-file size cap
(512 MB, the recorder's own), and a new file starts with that packet, so the
recording never stops until the 30-minute rotation. The 2 GB prune applies as
after any start. Manual and automatic recordings still stop at the cap.

Flows on port 80 or 443 (either side) are never game traffic: the rolling
recorder's heartbeat fallback ignores them, and so do the dispatcher, the Npcap
magic probe and WinDivert's candidate set. A CDN download on 443 carries the
`0E 00 36` bytes by chance (seen at 34, 19 and 4 packets in three 443 flows of
one evening) and was taken for the game, filling a 512 MB file in 20 minutes.

### On Early Access day

1. Start the meter and play. The first two minutes are recorded automatically.
2. Settings → Aion 2 → Connection → Advanced: switch on **Opcode census**.
   Start a manual recording as well if a longer session is wanted.
3. Read the census. Familiar opcodes (`04,38` damage, `05,38` DoT, `2A/2B,38`
   buffs, `33,36` player info, `41,36` summon) at familiar sizes means the
   parsers should hold. Amber `?` rows are the work list.
4. Connection shows the server IPs and ids. Those fill in the global region
   fingerprint.
5. From then on, replay the recording instead of playing.

## The meter between fights (2.2.0)

The inherited meter never ended a fight. Totals piled up target after target until
someone pressed Reset, and the whole pile was cloned and sent to the overlay on
every snapshot. The overlay also sat on screen whether or not anything was
happening.

**Activity, not damage, decides.** `DataStorageInner::activity_at` records the
last hit that belongs to your fight (`is_own_fight`): one you dealt, one you
took, or one your party landed on your current target -- a healer can go minutes
without a hit of their own on a boss. Before the game has said who you are,
every hit counts. Other people fighting nearby move nothing.

The snapshot loop then does two things each tick:

- **Idle reset.** `idle_reset_secs` (default 300, 0 = never) after the last
  activity, the fight is saved to History and the meter starts clean
  (`end_idle_fight`). A meter holding only strangers' fights is cleared
  without a record, measured from its start time, so it cannot grow unseen.
- **Visibility.** With `hide_when_idle` on, the loop says
  `MeterEvent::DpsIdleHidden(true)` while there is no activity since the last
  reset and no *peek* is running, and `DpsIdleHidden(false)` at the first
  activity or when a peek starts. The core peeks 4 s after Reset; the host asks
  for others with `DpsMeter::peek_overlay(secs)`. `hide_when_idle` is off by
  default since 2026-10-09 (Aether had it on).

**In the GPUI app** (TASK-25.2), Settings › Overlay › Window › "Hide between
fights" (`windows.hideMeterBetweenFights`, off by default; a `settings.json`
without the key reads as off) sets `hide_when_idle` on the running core at
once. The event goes through `bridge.rs` to `MeterModel` on the UI thread; the
hub (`toolbar/mod.rs`) observes the model and hides the meter's window
(`SW_HIDE`) or shows it without activating it (`SW_SHOWNOACTIVATE`, then
raised in the topmost band without activation), so the game keeps the focus.

- The window stays up while a fight summary (kill or wipe) is showing,
  and while the first-open prompt, which sits on it, waits for an answer.
- Opening the meter, from the toolbar or at launch, peeks 10 s. Clicking the
  toolbar's meter icon while the meter steps aside peeks 10 s instead of
  closing it; otherwise the icon opens and closes the meter as before.
- A meter that steps aside is still open: fights are tracked, the lock
  shortcut works on it, and Alt+Shift+CapsLock showing everything again leaves
  it aside unless a peek or a summary applies.
- A reminder card goes under the toolbar while the meter steps aside, as while
  it is closed, so notifications are never hidden with it. The other windows
  are unaffected.
- The pointer, the "…" menu and an unlocked meter do not keep it up.

The earlier Tauri overlay hid through a window call on the snapshot thread, which
deadlocked against `stop_dps_meter` joining that thread. The rule stays: no
window call on the snapshot thread; the core only emits the event, and
`bridge.rs` hands it to the UI thread.

**Off the UI thread** (TASK-25.6). The PvP window reads the meter's watch list
and kill stats on the background executor, one read at a time with the ticks
skipped meanwhile, and drops answers that predate a name change or a clear.
Saved and live fight reports are built on the background executor, and the
report window opens when the report is ready. History delete and reload stay
memory-only on the UI thread: the core writes the file on its own thread.

## Boss fights (2.3.0)

**English names.** `crates/core/data/npc_names_en.json` (mob code -> name, about
8,000) replaces the inherited Traditional Chinese catalogue, in the backend and in
History alike. It is built by `scripts/build-npc-names.mjs` from
Kuroukihime/AIon2-Dps-Meter's `mobs.json` (GPL-3.0), with NOIA2's partial
English names as a fallback; placeholder names ("None") are dropped. A target
the catalogue does not name is labelled `Boss <code>` or `Mob <code>`. The
Fighter's skill names, the server codes, and the healing-skill labels come from
the same project. The snapshot builder used to clone the whole name map five
times a second; it now looks names up one at a time
(`DataStorage::mob_name`).

**French names.** `crates/core/data/npc_names_fr.json` (about 7,900 mob codes) and
`crates/app/assets/game/data/skill_names_fr.json` (about 6,900 skill ids) hold
French names for ids that `npc_names_en.json` and `skill_names_en.json` already
have; no id is added and no English name changes. The core, the saved history
and the personal bests keep English. The app looks a name up by id when it
shows it (`game_data::skill_name`, `game_data::mob_name`) in the language in
force, English where an id has no French name, and follows the language switch
live. A record stores a mob code with its English name, so old history shows
French too. `scripts/build-fr-names.mjs` builds both files, per id: QuestLog.gg
by id (its public API, fetched in batches into a cache outside the repository),
then an optional community name list by id (free online sources, kept outside
the repository and read only for ids we already have), then
QuestLog's French for another of our ids with the same English name, then
the community list's French for the same English name, else none. "(DNT)" is ignored when
names are compared, and the "???" both sources hold for untranslated text is
not a name. Names are (c) NCSOFT. Today: mobs 7,885 / 0 / 24 / 0 with 82 left
English, skills 305 / 6,285 / 15 / 325 with 175 left English (layer order as
above). The packet explorer still reads English only.

Dungeon names and difficulties in `dungeons.json` have French too (QuestLog's
`database.getDungeon`; `game_data::dungeon_of` follows the language, English per
field where one is missing). Four empty English dungeon names were filled from
QuestLog, and four that came from the inherited translations (Phantoms Hall, Fallen
Guardian's City, Abyssal Rock Grotto, Reforged Abyss) now use the Global client's
names (Hall of Illusion, Citadel of the Fallen Daeva, Abyssal Horn Den, Abyssal
Forge: Ludra).

**Personal bests** (`crates/core/src/personal_best.rs`, tested) keep your best DPS
per character and boss in `history/personal_bests.json`, apart from the
history so deleting it keeps them. They learn from every record the history
saves, and are seeded from the history on disk on first run. A fight counts
when the target is a boss and it lasted at least 15 seconds; shorter kills
produce DPS figures no real fight reaches. The meter now saves the fight on
`stop_dps_meter` too, which used to throw it away.

**Saving history and personal bests** (`crates/core/src/background_save.rs`).
Neither store writes on the thread that changed it: a save or a delete changes
the records in memory, which is what every reader sees, and sets a flag; a
writer thread per file copies the records under the lock, releases it, then
serialises, compresses and writes (atomically, `*.tmp` then rename). A failed
write is logged and retried; `DpsMeter::save_now` writes what is still pending
on the calling thread and runs on `stop_dps_meter`, on quit and on drop. A
`combat_history.bin` or `personal_bests.json` that cannot be read (bad magic,
version, checksum, compression or JSON) is renamed `<name>.bad-<seconds>` and
the store starts empty; it is never overwritten, and the bests are not rebuilt
from the history. Only a missing bests file (a first run) is built from the
history. `personal_bests.json` is `{"version": 1, "bests": [...]}`; the bare
list older builds wrote still loads.

**Overlay frames** go out when their content differs from the last frame sent
(`CombatSnapshot::shows_the_same_as`), not only when the total damage changes:
a name, a boss's health, a buff starting or ending all count. The frame's
clock (`time_now`) and a buff's `coverage`, which follow the wall clock, do
not count, so an idle overlay is sent nothing.

**The overlay** asks for the best once per boss and character
(`get_personal_best`), shows the pace in the footer after 10 seconds of a boss
fight, and drops its cache on `personal-bests-updated`.

**Fight summary**, in the overlay. A boss whose health reaches zero gets a final
summary. A boss that goes quiet for 15 seconds gets a provisional one, which
gives way if the fight resumes and is replaced if the boss then dies, so a
phase transition cannot swallow the real summary. Skill names load on first
use (`lib/skill-names.js`, a split chunk), not with the overlay.

**Capsule mode** is `overlay.layout = "compact"`: one line with your DPS,
place, and time, expanded while the pointer is on the card or a summary is
up. The collapse waits 350 ms so crossing the card's edge does not make the
window jump.

## Staying clear of anti-cheat (2.4.0)

Anti-cheat judges a program by what it does to the game's process, not by
what it shows. An audit in 2.4.0 found no injection, memory access, input, or
hooks anywhere, but three things a strict anti-cheat could still count against
the player. All three are gone.

- **`System::new_all()` in the memory snapshot loop.** It was only after
  Aether's own CPU and memory, but on Windows `sysinfo` loads every process:
  `OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ)` on each one, the
  game included, handles held for the life of the `System`, and
  `ReadProcessMemory` on each PEB for command lines. The loop now builds a
  `System` with machine totals only and refreshes Aether's own pid with
  memory and CPU only. **Never use `System::new_all()` or refresh all
  processes with `sysinfo`.**
- **`OpenProcess` on the foreground window's process**, to learn its name, in
  the Tauri-era window plugins (`aion2_focus`, `game_display`, `on_top`, gone
  with the cutover). They answered from a Toolhelp process snapshot instead
  (the kernel's process list, no handle).
- **Loading the WinDivert driver just to ask.** The startup gate, the Connection
  status, and `check_state` each opened WinDivert, which installs and starts
  its kernel service and leaves it loaded until reboot. Some anti-cheats treat
  a loaded WinDivert as a warning sign, because lag switches use it. Npcap is
  asked first everywhere now; WinDivert is opened only when Npcap fails, or
  when Settings puts it first. `windivert_capturer` counts its handles and,
  when the last closes, stops the `WinDivert` service if Aether was the one
  that started it. WinDivert marks its service for deletion on start, so
  stopping it also removes it.

Capture itself was already passive: Npcap copies packets, and WinDivert is
opened with `SNIFF | RECV_ONLY`, so it can neither drop, delay, alter, nor
inject anything.

Since then the Npcap capturer also learns where the game's traffic is before
the first packet, from two things the OS publishes to any user: the process
list by image name (a Toolhelp snapshot, like Task Manager) for the PIDs of
`aion2.exe`, and the system TCP table (`GetExtendedTcpTable`, like
`netstat -ano`) for the connections those PIDs own. It opens no handle on any
process and reads no command line or start time. The connection picks which
adapter to probe first, the loopback adapter when the remote is 127.0.0.1
(an accelerator such as ExitLag proxies through loopback), and wakes detection
when a connection appears. It is only a hint: detection still waits for the
game's own bytes.

## Removed with the Tauri app (history)

Always on top (pinning the player's own browser above the game), the YouTube and
Twitch live chat pop-ups, the light theme and the background video went with the
Tauri app; `tauri-final` has them. One lesson still holds: windows over a
borderless or optimized-fullscreen game show because DWM composes them, but true
exclusive fullscreen bypasses DWM and nothing another process draws can appear
there short of injecting into the game, which anti-cheat bans. The GPUI app only
raises its own windows above a borderless game, never the game's.

## Repository cleanup (history)

The fork inherited scaffolding and dead weight that was removed along the way:
unreferenced JSON dumps at the repository root, a Path of Exile 2 scaffold, the
Tauri starter's demo page, template tutorials and vendored agent documentation.
In 2.2.0 it was narrowed to the DPS meter, Always on top and live chat: the
interactive map, the Main Character card, the buff monitor, the event and field
boss timers and the cloud account with its upload buttons were removed. Buff
*intervals* are still recorded for the detail window's coverage timeline, and
the `01,91` opcode stays in `KNOWN_PACKET_HEADERS`, so the stall resync and the
census treat it as before.

`docs/AION2_PACKET_PROTOCOL_ANALYSIS.zh-CN.md` is kept: the protocol analysis
inherited from NOIA2, real domain knowledge, in Chinese and not yet translated.
The application icon (`crates/app/windows/icon.ico`, moved from
`src-tauri/icons/`) is NOIA2's artwork; that is a deliberate choice.

## Cutover (Tauri removed)

On 2026-10-04 the Tauri + React app was removed, earlier than the migration
plan said (the owner never used it). The last commit that still had it is
tagged `tauri-final`. The repository is now the GPUI app (`crates/app`) and the
backend (`crates/aether-core`), built with Cargo alone.

Removed: `src-tauri/`, `src/`, `public/`, `index.html`, the Vite, TypeScript,
ESLint, Prettier and shadcn configs, `package.json` with the pnpm lockfile and
workspace file, `app-icon.png`, `.vscode/extensions.json`, the Tauri release
workflow, `scripts/copy-windivert-runtime.ps1` and `scripts/release-version.mjs`.
The updater, installer, tray, Always on top, chat pop-ups, light theme, Korean
UI and history screens went with it; the GPUI plan in `docs/GPUI_MIGRATION.md`
says which return and when. The updater is back as `crates/app/src/update/`
(`docs/updates.md`): GitHub Releases, an Ed25519-signed archive, a swap done by
the installed exe after a restart.

Moved (with history): the game data the UI compiles in
(`servers.json`, `dungeons.json`, the English skill names) to
`crates/app/assets/game/data/`; class icons, the skill icon set and the combat
power icon to `crates/app/assets/game/`; `WinDivert.dll` and `WinDivert64.sys`
(byte-identical) to `crates/app/windivert/`, which `crates/app/build.rs` copies
beside the exe; `icon.ico` to `crates/app/windows/`, embedded as the exe icon
by the `.rc` files.

## Own summons and own rows (TASK-3, TASK-5)

Two small core changes. `DataStorage::append_damage_at` no longer makes a hit on
the attacker's own summon the "last target" (the Elementalist's spirit
consumption hits her own spirits, which made the header read `Mob 2920133`).
The damage itself is still counted. `engine/session.rs` folds the local
player's rows into one: actor ids change with every zone and the earlier ids
kept their damage under the same name, so the Session board listed the player
once per zone. Only ids the meter established as the local player's are folded
(`DataStorage::own_actor_ids`, the ids carried across zone changes under the
same character, reset when the character changes); a namesake, for instance on
another server, keeps its own row. The row lists the folded ids in
`SessionPlayerStat::earlier_actor_ids`, and the session skill detail sums them.

## Skill names for damage-over-time ids

A damage-over-time hit carries its skill id followed by two digits (ten-digit example: `1709014011` = skill `17090140`). Nine-digit ones are a seven-digit skill plus the two digits; the app showed them as `Skill 160467011` because the lookup only cut them to eight digits. `crates/core/src/skill_names.rs` (`resolve_skill_key`, used by `game_data::skill_name`) now also tries the id divided by 100 as the last form (Kuroukihime/AIon2-Dps-Meter resolves the same id the same way, `DotDamagePacketProcessor`: `skillCode = u32 / 100`). A nine-digit id is not, however, a player's skill: the lookup rule stays because some nine-digit ids are really credited to players (Theostone procs such as `300111511`, Damage Reflection `122705021`, `120001xxx` effects) and the `/100` form is how `122705021` reaches "Damage Reflection".

### Thamon's absorption recovery is not player damage

In Thamon (Tamon trial, mob 2300811) the boss's absorption buffs recover health in periodic ticks that the game credits to players, under the effects `160466011` and `160467011` (seven-digit skills `1604660` and `1604670`, the "Attack" that `resolve_skill_key` names, plus two digits). The 2026-10-07 18:15 record has `160467011` as 1,516,948 of one player's 3,319,419 damage (148 hits, 1,724 to 95,764 each, no crit, back or double flags). Across the saved history, every Thamon record carries them, 15,898,260 damage in 2,099 hits for `160467011` and 4,497,840 in 537 for `160466011`, and no other boss does. Another public meter is known to suppress the same ticks.

`damage.rs` drops a nine-digit code whose `/100` is `1604660` or `1604670` (`is_thamon_absorption_recovery`) on every path: `04 38` hits, `05 38` damage ticks and the `05 38` heal and damage-taken ledger records. It is deliberately not a rule for all nine-digit codes: the history holds legitimate ones (Theostones, Damage Reflection), so the match is the two evidenced skills. The reference meter also matches the tick's effect instance against an active absorption buff on the target; this meter does not track that, so a player's own use of the same two effect ids would be dropped too (none was seen). No recording covers Thamon, so which of the two paths (`04 38` or `05 38`) carries the ticks is unconfirmed; both are covered. Buff codes `16046600`, `16046601`, `16046700` and `16046701` as damage were never seen and are not matched.

`packet-explorer unnamed-skills <recording>...` replays recordings through the shipped parser and lists the skill ids that still fall back to `Skill <id>` (hits, damage, class) so their names can be looked up in the source datasets and added.

Consumable buffs (scrolls, potions) are ids from 20 to 29 million: the item skill's seven-digit id plus one digit (`22101051` = skill `2210105`, Courage Scroll; `20111011` = `2011101`, Life Potion). `resolve_skill_key` tries `code / 10` for that range, after the exact and eight-digit forms, and the 70 seven-digit consumable names (Kuroukihime/AIon2-Dps-Meter's `skills.json`, GPL-3.0) are in `skill_names_en.json`; `build-fr-names.mjs` gave 61 of them a French name, the rest fall back to English. The first Courage Scroll showed as `Skill 22101051` on the Buffs tab of an Elementalist's fight report.

## Reading the game's packets (TASK-14)

The inherited parsers read most packets by guessing: fixed skips, byte markers and value
ranges. A study of how two other public meters read the same traffic
(notes kept outside the repository; none of their code was copied) gave the
real layouts, and each change below was checked by replaying the recordings
before and after.

- **Framing.** A frame is `[varint L][opcode 2][body]`, `L + varintLen - 4`
  bytes long. The inherited code used `L - 3` with a `+1` for bundles, right only when the
  length fits one byte: frames of 128 bytes or more ended one byte early and the
  next one was read from the wrong place (half of the long frames in some
  recordings, and one flow lost 561 frames).
- **Damage `04 38`** (`parser/damage_record.rs`): a reader driven by the u16
  flags field, which must consume the frame exactly (24,031 of 24,031 recorded
  frames do). The optional u32 sits before the hit kind; kind 9 (reflected) and
  frames without an amount give no hit. One record per frame: the `01 00` the
  old loop took for a separator is a trailing u16.
- **Spawn `41 36`** (`parser/spawn_record.rs`): entity, flags (u16 on the global
  servers, u32 on Korea/Taiwan), mob code, current and max HP, the buff list,
  then the owner when `flags & 0x10`. Summon owners come only from here now;
  the marker scans are gone, and `04 8D` (a death with its killer) no longer
  makes the killer the owner.
- **HP `00 8D`** (`parser/hp_record.rs`): a mask, small `{kind, i32}` and big
  `{kind, i64}` stats; kind 0 is current HP, kind 7 max HP. HP is 64-bit in
  storage (the inherited code dropped anything above 1e9); the app's HP fields stay u32
  and saturate.
- **Death `42 36`** (`parser/death_record.rs`): flag 3 sets a known monster's HP
  to 0, the same state an HP packet of 0 leaves, so existing kill detection
  confirms deaths it missed. Flag 1 is an entity leaving range (a boss that
  sent it reported HP again minutes later); flag 7 is a removal. A flag-7
  removal of a boss somebody hit that was still alive is a wipe.
- **Boss wipes** (`storage/boss_wipe.rs`): the game sends no wipe event and
  instant-revive stones make "everyone dead" unreliable, so the boss is the
  witness. A wipe is a boss (not a summon) that took damage while fights were
  tracked and then either has its HP back at full after falling below 98%, or
  is removed with flag 7 alive. Seen in recordings: Atiel's removal at 26.7%
  and 61%/7.5%, Talisra's refill from 12.8%. The attempt is saved to History
  with `wiped: true` and the HP it had left, kept out of personal bests, and
  taken off the meter so the next pull counts from zero.
- **Damage over time `05 38`**: only mode `0x0A` is damage. Mode `0x0B` (a
  self, party or pet effect, never a monster in the recordings) used to be
  counted, with its remaining total read as the tick.
- **Buff removal `2C 38`** (`parser/buff_removal_record.rs`): ends the removed
  instance's interval instead of letting it run to its full duration. The
  apply packets are read by opcode (`2A` has a count byte, `2B` does not)
  instead of trying one offset and then another.

## The fight report's data (TASK-15.2)

The end-of-fight report for boss fights needs what damage dealt does not say:
what the party took, healed and lost. `crates/core/src/report/` records it and
builds the report; the report window is TASK-15.3 and 15.4.

- **Recorded while playing** (`report/events.rs`, in `DataStorage`, cleared by a
  reset like the stats): hits taken, heals and player deaths, each with its
  time, in bounded queues (60,000 hits, 60,000 heals, 2,000 deaths; the oldest
  go first). Nothing here touches the damage stats, the target or the boss
  focus: a monster's hit on a player used to be dropped by the 7-digit skill
  rule, and still is; it is also kept as damage taken. Replaying the seven
  recordings before and after gives identical targets, boss focus and damage.
- **Hits taken**: a `04 38` record with an amount whose skill is not a player's,
  and a `05 38` damage tick (mode `0x0A`) of such a skill, on a target that is
  not a monster, from a source that is not a known player or a player's summon.
  A boss's adds are summons owned by the boss (or by a monster not yet seen
  spawning), so their hits count at the full amount, one-shots and the last hit
  of a death included; the owner chain is followed four steps and a hit is
  skipped only when it ends at a player (a pet's hit on someone in PvP).
- **Heals**, kept per skill and kind (`HealKind`): *Direct*, a hit of a skill in
  `healing_skill_code.json` with an amount on a non-monster target (heals of
  others and of oneself; seen from a Cleric's Radiant Benediction, Healing
  Light, Radiant Recovery and a Chanter's Recuperation); *OverTime*, a `05 38`
  tick of mode `0x0B` of a class skill (8-digit code) with its tick amount
  (Light of Regeneration, Recuperation; the 7-digit self ticks of system
  effects are not a player's healing); *Drain* and *Regeneration*, the extra
  amounts of a damaging hit (`F & 0x200`, `F & 0x400`), credited to the
  attacker (only regeneration appears in the recordings: a flat recovery per
  hit). Overheal cannot be separated, and a shield's amount cannot be told from
  a heal's: no shield skill carries an amount in the recordings except three
  hits (Protection Armor 451 and 75, Crossguard 153), which count.
  `heal_amount` (the `F & 2` block) is read but not counted; it only appears
  on monster hits on monsters.
- **Deaths** (`parser/char_die_record.rs`): `04 8D` is a death of any entity
  with the killer the game names (a player by name, a monster by its mob code or
  entity), and, for a player victim, the last hit taken in the 5 s before it.
  `04 8D` was read as summon ownership before TASK-14 and had no parser after.
- **Scope**: the report covers the window from the first to the last hit on the
  boss plus 5 s (a wiped party dies just after its last blow). Players are the
  ones who hit the boss and the ones who healed them. Buff uptime is clipped to
  the window with overlaps counted once.
- **Saved**: `HistoryRecord.ledger` (`FightLedger`, serde default `None`) only
  on boss records, about 15 KB of JSON (3 KB compressed) for a five-player
  fight. Older files load unchanged and open with Healing, Damage taken and
  Deaths "not recorded" (`None`, not zero); no migration rewrites them.
- **The data API**: `FightReport::from_record(&HistoryRecord)` for History,
  `DpsMeter::fight_report(target_id)` for the fight on the meter (newest saved
  record of that target when the meter has been reset). One value carries the
  header, the five key figures, one row per player and, per player, the hit
  types as counts, the skills, buffs, heals and damage taken.
- **Summoned objects are their owner (2026-10-08)**: a boss spawns many entities (projectiles, zones, copies of itself with its HP, e.g. Talisra of the Void's 2920519, 2920520, 2920521, 2920563, 2920564, 2920936) whose spawn record names the boss as owner. Their hits used to show as separate "Mob <code>" sources and as the killer of a death. A hit and a killer now also carry `owner_mob_code`, the mob code at the top of the owner chain (`owned_by_mob_code`, four steps), and `CombatEvents::credit_unfought_sources_to_their_owners` (run in `DataStorage::fight_ledger_inputs`) swaps the source for it, unless players ever damaged the entity itself: that is a real add (Atiel's clones, Nathara's adds, the boss's own phases) and stays its own source. "Damaged by players" is `entities_hit_by_players`, filled in `append_damage_at` before the Boss only filter (so the filter cannot turn a real add into an object), forgotten when the game reuses the entity id for another mob. Not covered: an object owned by a real add that is owned by a boss credits the boss, not the add. Fights already saved keep their old rows.
- **Buffs follow the owner chain too (2026-10-08)**: `save_buff` credits a buff to the top of the caster's owner chain, as damage is. An Elementalist's spirits (hundreds of entities per fight, one per summon) used to be one buff row each; 219 of them became one row. In the report, casters it does not know (`UNKNOWN_CASTER_ID`, 0) are one row per buff with the longest uptime among them, which also folds the split rows of reports saved before.
- **One buff per name and caster (2026-10-09)**: several skill ids can show one name (Spirit's Descent 167300011 and 167300012, Elemental Fusion 163000002 and 163000003, Element Unification 167800011 and the pulses 167800021 to 167800025, whose key 16780000 differs from 16780001). `engine/buff_groups.rs` joins a caster's ids on one target by their English name (`skill_names::english_name`, from the bundled `skill_names_en.json`, which the app now reads through the core too) and takes the union of their intervals, in the fight ledger and in the live buff summaries; the id with the most uptime stays as the representative. Fights saved before keep their per-id rows, and the app merges rows with one name, icon and caster by the longest uptime (a lower bound: the saved ledger holds no intervals).

## Matching the game's Combat Analysis (2026-10-09)

Compared on one fight (Fafnir's Poison Blood, entity 43152, Elementalist Ysmera: the game lists 655 hits, 46% critical, 2,282,703 damage), three rules now follow the game:

- **Spirit specials keep their own skill id.** `normalize_skill_id` (`parser/damage.rs`) cuts the last four digits of an 8-digit skill, which merged the Elementalist spirits' specials `16001104`, `16001108`, `16001116` and `16001117` into one "Skill 16000000" row; ids `16001000..=16001999` are now kept as they are, like `30000000..=30999999`. The bundled English and French names already hold them. Other merged buckets (`16150000`: Jointstrike: Corrode with the spirits' Break and Rage Burst) are left as they are until the game's rows are seen.
- **A damage-over-time tick adds damage, not a hit.** A `05 38` tick of mode `0x0A` with a ten-digit effect id (`1614000011`) is added to the row of the skill that applied it (`skill_stats_code` in `storage/data_storage.rs`: the effect id divided by 100, then the usual normalisation, so it is the key a direct hit of that skill gets). It adds its damage and nothing else: no hit, no critical/back/front/perfect/double/multi-hit count, no minimum or maximum. Totals do not change; every rate (fight report, Combat Analysis window, boards, summary) is over direct hits, as in the game (Jointstrike: Curse = 13 hits, 106,148 with 65 ticks). A skill that only ticked has a row with damage and 0 hits (rates read "--"). Other ticks (nine-digit effects) keep their own rows, also without hits. Saved records keep what they stored.
- **A record aimed at the attacker's own side is not damage.** The Elementalist's spirit skill `16990004` is sent with the spirit as attacker and target, about 100 ms after its spawn, and counted as 21.5M of damage on one session. In `append_damage_at` a record whose target ends at the same owner chain as the attacker (itself, its owner, an own summon, a sibling summon) is dropped before any damage accounting, the fight window and the last target; the drain and regeneration amounts it carries still go to healing as before. Whether `16990004` is a heal (the game's Total Heal looks made of it) is not decided.
- **A damage-over-time tick has one reader and one owner (2026-10-09, TASK-25.8)**: `parse_dot_packet` reads each `05 38` frame once, through `read_periodic_record`, and the tick's skill field decides whose it is, for damage dealt and damage taken alike (`damage_tick_source` in `parser/damage.rs`). A tick of a player's skill is damage dealt by its actor, as before; a tick of a monster's skill (for example `1217330` on players) is damage taken by the target and no longer also damage dealt by the monster's actor id, which had given monsters rows on the Session board. A Thamon absorption recovery, named by the skill or by the effect id (214 ticks in `session-1791396464753` carry it only in the effect), is neither. Heal ticks are unchanged. The old hand-written second read accepted exactly the same 21,203 real mode `0x0A` ticks as the record reader in the 28 recordings that hold any, none more and none fewer, so no tick is lost; the compared fight stays 2,282,703 damage.

## Why capture could not start, as data (2026-10-09, TASK-25.26)

`DpsMeter::start_dps_meter` returns `CaptureStartError::NoBackend` (which driver the priority tried first, and Npcap's and WinDivert's own errors as they reported them) instead of an English sentence, so the app writes the waiting card and the Settings capture tile in the language in force; its `Display` is the old sentence, for the log. `start_for_replay` opens no driver and no longer returns a `Result`.

## Recordings off the dispatcher, and counted drops (2026-10-09, TASK-25.7)

The dispatcher used to write the session recording and the rolling recording itself, before it reassembled the stream, and the rolling recorder rotated files and pruned the recordings folder (up to 2 GiB of listing and deleting) on the same thread. A slow disk stalled it, the capture queue filled, and `try_send` dropped game packets without a sign.

- **A writer thread per recording** (`capture/recording_queue.rs`): the dispatcher calls `RecordingQueue::offer`, which asks the sink whether it wants the packet (an atomic flag for `PacketRecorder`; the game-traffic filter for `RollingRecorder`, memory only, in a lock of its own), copies it into a bounded queue of 8,192 packets and returns. The writer thread calls the existing `PacketRecorder::record`, rotates the rolling file and prunes the folder; none of it is on the dispatcher. A full queue drops the packet being offered, whole, and counts it, so a file is always a valid `.aetherpc` (format unchanged). Stopping and quitting wait for the queues to reach their files (`RecordingQueue::flush(deadline)`), then flush the buffered writer. The quit path (`stop_dps_meter` and `DpsMeter::save_now`, which the app's quit hook calls; the hook has 3 s before `quit.rs` ends the process) writes history, personal bests and mobs first, then waits for both recordings under one 1.5 s deadline; a flush that gives up marks the writer stalled until it next completes a write, so the second call costs nothing and the sink is not flushed while the writer holds it. A manual or automatic recording stopped on its own waits up to 5 s.
- **Sizes.** Measured on the recordings: game traffic peaks at 148 packets/s (largest game payload 8.8 KB, average 55 to 800 bytes), the worst older burst of unfiltered TCP is 3,900 packets/s with payloads up to 64 KB. Both queues hold 8,192 packets: 55 s of game traffic or 2 s of that burst, 6.5 MB at the 800-byte average, 72 MB if every packet were the largest game payload. The capture queue was 2,000,000 packets (gigabytes). Replay's back-pressure limit is half the new capacity (4,096).
- **Counters** (`capture/drop_report.rs`): `Channel::rejected_when_full` counts the packets a capturer's `try_send` could not queue (both Npcap and WinDivert); `RecordingQueue::dropped` counts the recording drops. `DpsMeter::packet_drops()` returns both totals (`PacketDrops`). The memory snapshot loop writes one line to aether.log at most every 30 s when something new was dropped, and stopping the meter writes the last one: `packets dropped: <n> before the dispatcher read them (queue full), <m> left out of recordings (queue full); totals <N> and <M>`. A drop before the dispatcher is missing from the game stream and can break reassembly; a recording drop leaves the meter's numbers alone.
- Not done: Npcap still forwards every TCP packet, not only candidate game flows; the stream buffer (TASK-25.9) is separate.

## Where diagnostics go (2026-10-09, TASK-25.36)

The shipped exe has no console (`windows_subsystem = "windows"` without the
`probes` feature), so `eprintln!` reached nobody. Every failure the app used to
print goes to `aether.log` instead, the file the core already writes to:

- `crates/app/src/app_log.rs` holds the one `AppLogger` of the run. `run`
  installs it before anything else (`%LOCALAPPDATA%/<app>/logs/aether.log` for a
  live run, the temp folder's `aether/gpui-app.log` for replays and captures,
  which never touch the player's log) and hands the same logger to the meter, so
  the app's lines and the core's interleave in one file. `app_log::error` and
  `app_log::warn` are free functions the windows call; a `probes` build also
  echoes them to its console. Before `install`, and in unit tests, a line is dropped.
- Levels: a file that could not be read, set aside or saved (history, personal
  bests, settings, to-dos), a window that could not be opened, the meter or the
  single-instance listener failing to start: `ERROR`. A window that could not be
  placed, fitted or locked, a shortcut, the tray or the reminder sound that is
  unavailable, a click that found no fight report: `WARN`. The core's capture
  thread failing to open a device: `ERROR`. `[dps_meter] save: ...` was deleted
  (the reset line already says it); "main actor detected" is `DEBUG`.
- Nothing on the capture, parser or snapshot threads writes per packet or per
  tick. A failed history or bests save repeats every 5 s at most (the writer
  thread's retry delay).
- `eprintln!` stays only where a console exists: tests, `crates/xtask`,
  `crates/packet-explorer`, `crates/core/examples/replay.rs` and the `probes`
  modules (`capture*.rs`, `font_probe.rs`, `probe.rs`, `fixtures.rs`).

## Version and changelog (TASK-29)

- The version restarts at 1.0.0 (`meter-core` and the app). The inherited 2.4.0
  stays as the fork point named in the README.
- Aether's release history left `CHANGELOG.md` for `docs/aether-changelog.md`,
  unedited but for a heading and one line saying what it is. `CHANGELOG.md` and
  `CHANGELOG.fr.md` now hold this app's own changelog, read by the app
  (`crates/app/src/changelog/`).

# Technical notes

How parts of the current code work, and the rules that keep them working. The
layout and the gotchas are in [AGENTS.md](../AGENTS.md); the updater is in
[updates.md](./updates.md). Earlier history is in git (tag `tauri-final` is the
last commit of the Tauri + React app).

## Staying clear of anti-cheat

Anti-cheat judges a program by what it does to the game's process, not by what
it shows. Nothing here injects, reads memory, sends input or hooks anything.

- **No process-wide `sysinfo` refresh.** On Windows `sysinfo` opens every
  process (`OpenProcess` with `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`, the
  game included) and reads each PEB for command lines. The memory snapshot loop
  builds a `System` with machine totals only and refreshes the app's own pid
  with memory and CPU only. **Never use `System::new_all()` or refresh all
  processes with `sysinfo`.**
- **No `OpenProcess` on another program.** Anything that needs another
  process's name uses a Toolhelp process snapshot (the kernel's process list, no
  handle).
- **WinDivert is loaded only when needed.** Loading it installs and starts a
  kernel service that stays until reboot, and some anti-cheats treat a loaded
  WinDivert as a warning sign, because lag switches use it. Npcap is asked first
  everywhere (startup gate, Connection status, `check_state`); WinDivert is
  opened only when Npcap fails, or when Settings puts it first.
  `windivert_capturer` counts its handles and, when the last closes, stops the
  `WinDivert` service if the app was the one that started it. WinDivert marks
  its service for deletion on start, so stopping it also removes it.
- **Capture is passive.** Npcap copies packets, and WinDivert is opened with
  `SNIFF | RECV_ONLY`, so it can neither drop, delay, alter nor inject anything.
- **Finding the game's traffic before the first packet** uses only what the OS
  publishes to any user: the process list by image name (a Toolhelp snapshot,
  like Task Manager) for the PIDs of `aion2.exe`, and the system TCP table
  (`GetExtendedTcpTable`, like `netstat -ano`) for the connections those PIDs
  own. No handle is opened on any process and no command line or start time is
  read. The connection picks which adapter to probe first (the loopback adapter
  when the remote is 127.0.0.1, as an accelerator such as ExitLag proxies
  through loopback) and wakes detection when a connection appears. It is only a
  hint: detection still waits for the game's own bytes.
- **Overlay windows over the game.** Windows over a borderless or
  optimized-fullscreen game show because DWM composes them; true exclusive
  fullscreen bypasses DWM and nothing another process draws can appear there
  short of injecting into the game. The app only raises its own windows above a
  borderless game, never the game's.

## WinDivert files

`WinDivert.dll` and `WinDivert64.sys` are bundled beside the exe
(`crates/app/windivert/`, copied beside the exe by `build.rs` and shipped in
the zip). There is no download of the driver at run time.

The vendored binaries are byte-identical to the official `WinDivert-2.2.2-A.zip`
from `basil00/WinDivert`. The company on the driver's Authenticode signature is
WinDivert's own signer. `cargo xtask dist` checks these hashes before
packaging:

|File|sha256|
|---|---|
|`WinDivert.dll`|`c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2`|
|`WinDivert64.sys`|`8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2`|

The licence text (LGPL-3.0 or GPL-2.0, your choice) is
`crates/app/windivert/LICENSE`, from the same release.

The build delay-loads `WinDivert.dll`, so the exe starts and captures with
Npcap when the two files are absent; they still ship in the zip for the
fallback.

`crates/core/src/preflight.rs` weighs the capture runtime instead of passing or
failing it:

- **Required vs optional is computed, not fixed.** Capture needs one backend, so
  Npcap is only required while WinDivert cannot stand in for it. An unavailable
  WinDivert beside a working Npcap is shown in grey and never blocks.
- **The Npcap probe enumerates adapters** rather than loading `wpcap.dll`, which
  keeps succeeding after the driver service stops.

## Region profiles

A server id is used structurally: it decides where the server field sits inside
a player-info packet. If the rule rejects the service's ids, `find_server_id`
returns `None` and no player is ever named, with no error to go on.

`crates/core/src/region.rs` holds the profiles:

|Profile|Server-id rule|
|---|---|
|`Auto` (default)|`1001..=1999`, `2001..=2999`|
|`Taiwan`|`1001..=1021`, `2001..=2021` (the bundled catalogue)|
|`Korea`, `Global`|same permissive rule as `Auto`|

The permissive rule is structural: server ids are `raceId * 1000 + index` (race
1 Elyos, race 2 Asmodian). It keeps the disambiguation the parser depends on (an
arbitrary `u16` still fails) without assuming how many servers a region runs.
The profile is always `Auto`: `DpsMeterConfig::normalized` brings a stored
region back to `Auto`, and Settings has no picker. The profiles stay because
detection still names Korea.

Detection claims a region only on evidence. Korea is identifiable by its server
block (`206.127.156.0/24`); Taiwan's and global's blocks are unknown, so
detection returns "no fingerprint matched" rather than guessing from an id
range they may share. Settings → Aion 2 → Connection shows the observed server
IPs and ids so an uncatalogued service can be characterised from a real
session. Server names fall back to `Server <id>`.

## Packet recording and replay

`crates/core/src/capture/recorder.rs` writes raw packets to
`%APPDATA%/Belisk/recordings/*.belisk`, tapped in the dispatcher before
reassembly, so a recording holds exactly what capture produced. (`.aetherpc`
files from earlier builds, magic `AETHERPC`, still read; see AGENTS.md,
"Recordings keep both names".)

Replay pushes those packets back into the same channel the capturer feeds.
Reassembly, dispatch, parsing and aggregation then run identically to a live
session: the same code path with the same input. Capture once, iterate offline.

The format is plain: a 12-byte header, then one length-prefixed record per
packet, little-endian. A recording cut short by the app exiting reads up to the
cut rather than failing. Recording is off by default and capped at 512 MB.
Replay applies backpressure when the queue runs deep: `Channel::try_send` drops
the packet it is handed when full, and a silently lossy replay would be worse
than a slow one.

### Opcode census

`crates/core/src/capture/census.rs` counts every dispatched packet by opcode,
recording payload size ranges and whether a parser claims it. A mismatch with
the known opcodes would present as an empty meter with no error, so it is
measured. Off by default; it sits on the per-packet path, so disabled costs one
relaxed atomic load.

### Automatic recording

On a server no fingerprint matches, the meter records the first two minutes of
game traffic by itself, once per meter session (`auto_record_tick` in
`engine/meter.rs`, on the memory-snapshot thread). Files are named `auto-*` and
only the newest five are kept (`recorder::prune_recordings`); a recording started
by hand is never pruned and never interrupted. Today that is every Taiwan and
global session, because neither has a fingerprint. Off switch: Settings → Aion 2
→ Connection → Advanced.

### Rolling recording of whole sessions

Settings → Capture → **Record every session** (off by default) keeps the game's
traffic of the whole live session so a game event can be investigated after the
fact (`capture/rolling_recorder.rs`). It is a second `PacketRecorder` beside the
manual / automatic one, so all of them can run at once; files are
`rolling-<unix ms>.*`, in the same recordings folder and format, and replay like
any other. A new file starts every 30 minutes, and after each start (and when
the meter starts) the oldest `rolling-*` files are deleted until their total is
at most 2 GB. `session-*` and `auto-*` files are never touched. It records only
while the meter captures live (not during a replay) and closes the file when the
switch is turned off or the meter stops.

Only game packets are written. A packet is kept when its (local port, remote
ip:port) pair is a connection `query_game_connections()` lists for `aion2.exe`
(refreshed once a second on its own thread, never per packet), when it goes to a
server port such a connection has used (so a new connection is not lost before
the next refresh), or when its flow has carried the `0E 00 36` heartbeat (the
fallback when the lookup finds nothing, as with a replayed source). Packets
before the heartbeat on a flow the lookup cannot see are not kept; the
dispatcher ignores them as well.

A rolling file also ends when the next packet would pass the per-file size cap
(512 MB, the recorder's own), and a new file starts with that packet, so the
recording never stops until the 30-minute rotation. Manual and automatic
recordings still stop at the cap.

Flows on port 80 or 443 (either side) are never game traffic: the rolling
recorder's heartbeat fallback ignores them, and so do the dispatcher, the Npcap
magic probe and WinDivert's candidate set. A CDN download on 443 can carry the
`0E 00 36` bytes by chance and was once taken for the game, filling a 512 MB
file in 20 minutes.

### Recordings off the dispatcher, and counted drops

The dispatcher does not write recordings. Each recording has a writer thread
(`capture/recording_queue.rs`): the dispatcher calls `RecordingQueue::offer`,
which asks the sink whether it wants the packet (an atomic flag for
`PacketRecorder`; the game-traffic filter for `RollingRecorder`, memory only, in
a lock of its own), copies it into a bounded queue of 8,192 packets and returns.
The writer thread calls `PacketRecorder::record`, rotates the rolling file and
prunes the folder. A full queue drops the packet being offered, whole, and
counts it, so a file is always a valid recording. Stopping and quitting wait for
the queues to reach their files (`RecordingQueue::flush(deadline)`), then flush
the buffered writer. Replay's back-pressure limit is half the queue capacity
(4,096).

Sizing, measured on recordings: game traffic peaks at 148 packets/s (largest
game payload 8.8 KB, average 55 to 800 bytes); the worst burst of unfiltered TCP
is 3,900 packets/s with payloads up to 64 KB. A queue of 8,192 packets holds
55 s of game traffic or 2 s of that burst.

Counters (`capture/drop_report.rs`): `Channel::rejected_when_full` counts the
packets a capturer's `try_send` could not queue (Npcap and WinDivert);
`RecordingQueue::dropped` counts the recording drops. `DpsMeter::packet_drops()`
returns both totals (`PacketDrops`). The memory snapshot loop writes one line to
the log at most every 30 s when something new was dropped, and stopping the
meter writes the last one: `packets dropped: <n> before the dispatcher read them
(queue full), <m> left out of recordings (queue full); totals <N> and <M>`. A
drop before the dispatcher is missing from the game stream and can break
reassembly; a recording drop leaves the meter's numbers alone. Npcap still
forwards every TCP packet, not only candidate game flows.

## Names and data

- **English names.** `crates/core/data/npc_names_en.json` (mob code → name, about
  8,000) is built by `scripts/build-npc-names.mjs` from
  Kuroukihime/AIon2-Dps-Meter's `mobs.json` (GPL-3.0), with NOIA2's partial
  English names as a fallback; placeholder names ("None") are dropped. A target
  the catalogue does not name is labelled `Boss <code>` or `Mob <code>`. The
  Fighter's skill names, the server codes and the healing-skill labels come from
  the same project. Names are looked up one at a time (`DataStorage::mob_name`),
  never by cloning the whole map.
- **French names.** `crates/core/data/npc_names_fr.json` (about 7,900 mob codes)
  and `crates/app/assets/game/data/skill_names_fr.json` (about 6,900 skill ids)
  hold French names for ids that `npc_names_en.json` and `skill_names_en.json`
  already have; no id is added and no English name changes. The core, the saved
  history and the personal bests keep English. The app looks a name up by id when
  it shows it (`game_data::skill_name`, `game_data::mob_name`) in the language in
  force, English where an id has no French name, and follows the language switch
  live. A record stores a mob code with its English name, so old history shows
  French too. `scripts/build-fr-names.mjs` builds both files, per id: QuestLog.gg
  by id (its public API, fetched in batches into a cache outside the repository),
  then an optional community name list by id (free online sources, kept outside
  the repository and read only for ids we already have), then QuestLog's French
  for another of our ids with the same English name, then the community list's
  French for the same English name, else none. "(DNT)" is ignored when names are
  compared, and the "???" both sources hold for untranslated text is not a name.
  Names are © NCSOFT. The packet explorer reads English only.
- **Dungeons.** Names and difficulties in `dungeons.json` have French too
  (QuestLog's `database.getDungeon`; `game_data::dungeon_of` follows the
  language, English per field where one is missing). They use the Global client's
  names (Hall of Illusion, Citadel of the Fallen Daeva, Abyssal Horn Den, Abyssal
  Forge: Ludra).
- **Pet names, families and drops.** `crates/app/assets/game/data/pets.json` maps
  the game's own pet id (the id in the `00 90` / `0D 90` pet messages, 1001 to 1299,
  207 pets on Global) to `{en, fr, family, mobs}`. The family is Cogni, Fera,
  Natura, Varian or Special (QuestLog's `intellect`, `feral`, `nature`, `trans`,
  `special`) and `mobs` are the mob codes that drop the pet's soul. Source:
  QuestLog.gg's public API, as for the French names (`database.getPets` in both
  languages, then the `vehiclesoul` items; a mob gives the pet named in the
  "Soul: <pet name>" item it drops). `scripts/build-pet-data.mjs` builds the file
  from a cache outside the repository and stops on a mob that would give two
  pets or a name with Chinese characters. `game_data::pet_name` (language in
  force, English as the fallback), `pet_family` and `pet_of_mob` read it; an id
  or mob it does not list gives `None`, and the window shows its own fallback.
  A mob drops a soul of only one pet, which is how souls of a pet already at
  Lv 3 (`11 90`, no pet id) can be attributed to the mob that was hit. The mob
  lists come from the drop tables, not from the recordings: pairing kills with
  soul gains by time is ambiguous when several mobs of a pack die within a
  second or two of each other. 42 pets (the 7 Specials and 35 others) have no
  mob in the source.
- **Skill names for damage-over-time ids.** A damage-over-time hit carries its
  skill id followed by two digits (`1709014011` = skill `17090140`).
  `crates/core/src/skill_names.rs` (`resolve_skill_key`, used by
  `game_data::skill_name`) tries the exact id, the eight-digit form, and the id
  divided by 100 as the last form. A nine-digit id is not necessarily a player's
  skill (some are credited to players: Theostone procs such as `300111511`,
  Damage Reflection `122705021`, `120001xxx` effects), so the lookup rule stays
  general. Consumable buffs (scrolls, potions) are ids from 20 to 29 million: the
  item skill's seven-digit id plus one digit (`22101051` = skill `2210105`);
  `resolve_skill_key` tries `code / 10` for that range, after the exact and
  eight-digit forms. `packet-explorer unnamed-skills <recording>...` replays
  recordings through the shipped parser and lists the skill ids that still fall
  back to `Skill <id>` so their names can be looked up and added.
- The bundled data contains no Chinese; regenerate it with the scripts rather
  than editing by hand.

## Parsers and fight tracking

- **Framing.** A frame is `[varint L][opcode 2][body]`, `L + varintLen - 4` bytes
  long.
- **Damage `04 38`** (`parser/damage_record.rs`): a reader driven by the u16
  flags field, which must consume the frame exactly. The optional u32 sits before
  the hit kind; kind 9 (reflected) and frames without an amount give no hit. One
  record per frame.
- **Spawn `41 36`** (`parser/spawn_record.rs`): entity, flags (u16 on the global
  servers, u32 on Korea/Taiwan), mob code, current and max HP, the buff list,
  then the owner when `flags & 0x10`. Summon owners come only from here.
- **HP `00 8D`** (`parser/hp_record.rs`): a mask, small `{kind, i32}` and big
  `{kind, i64}` stats; kind 0 is current HP, kind 7 max HP. HP is 64-bit in
  storage; the app's HP fields are u32 and saturate.
- **Death `42 36`** (`parser/death_record.rs`): flag 3 sets a known monster's HP
  to 0, the same state an HP packet of 0 leaves. Flag 1 is an entity leaving
  range; flag 7 is a removal. A flag-7 removal of a boss somebody hit that was
  still alive is a wipe.
- **Boss wipes** (`storage/boss_wipe.rs`): the game sends no wipe event and
  instant-revive stones make "everyone dead" unreliable, so the boss is the
  witness. A wipe is a boss (not a summon) that took damage while fights were
  tracked and then either has its HP back at full after falling below 98%, or is
  removed with flag 7 alive. The attempt is saved to History with `wiped: true`
  and the HP it had left, kept out of personal bests, and taken off the meter so
  the next pull counts from zero.
- **Damage over time `05 38`**: only mode `0x0A` is damage. Mode `0x0B` is a
  self, party or pet effect.
- **Buff removal `2C 38`** (`parser/buff_removal_record.rs`): ends the removed
  instance's interval. The apply packets are read by opcode (`2A` has a count
  byte, `2B` does not).
- **Death `04 8D`** (`parser/char_die_record.rs`): a death of any entity with the
  killer the game names (a player by name, a monster by its mob code or entity),
  and, for a player victim, the last hit taken in the 5 s before it.
- **Thamon's absorption recovery is not player damage.** In Thamon (mob
  2300811) the boss's absorption buffs recover health in periodic ticks that the
  game credits to players under the effects `160466011` and `160467011`
  (seven-digit skills `1604660` and `1604670`). `damage.rs` drops a nine-digit
  code whose `/100` is one of those two (`is_thamon_absorption_recovery`) on
  every path: `04 38` hits, `05 38` damage ticks and the `05 38` heal and
  damage-taken ledger records. It is deliberately not a rule for all nine-digit
  codes.
- **A damage-over-time tick has one reader and one owner.** `parse_dot_packet`
  reads each `05 38` frame once, through `read_periodic_record`, and the tick's
  skill field decides whose it is, for damage dealt and damage taken alike
  (`damage_tick_source` in `parser/damage.rs`). A tick of a player's skill is
  damage dealt by its actor; a tick of a monster's skill is damage taken by the
  target and not damage dealt by the monster's actor id.
- **Own summons and own rows.** `DataStorage::append_damage_at` does not make a
  hit on the attacker's own summon the "last target" (the damage is still
  counted). `engine/session.rs` folds the local player's rows into one: actor ids
  change with every zone. Only ids the meter established as the local player's
  are folded (`DataStorage::own_actor_ids`, reset when the character changes); a
  namesake keeps its own row. The row lists the folded ids in
  `SessionPlayerStat::earlier_actor_ids`, and the session skill detail sums them.

## Matching the game's Combat Analysis

Three rules follow the game's own Combat Analysis window:

- **Spirit specials keep their own skill id.** `normalize_skill_id`
  (`parser/damage.rs`) cuts the last four digits of an 8-digit skill, except ids
  `16001000..=16001999` (the Elementalist spirits' specials) and
  `30000000..=30999999`, which are kept as they are.
- **A damage-over-time tick adds damage, not a hit.** A `05 38` tick of mode
  `0x0A` with a ten-digit effect id (`1614000011`) is added to the row of the
  skill that applied it (`skill_stats_code` in `storage/data_storage.rs`: the
  effect id divided by 100, then the usual normalisation, so it is the key a
  direct hit of that skill gets). It adds its damage and nothing else: no hit, no
  critical/back/front/perfect/double/multi-hit count, no minimum or maximum.
  Totals do not change; every rate (fight report, Combat Analysis window, boards,
  summary) is over direct hits. A skill that only ticked has a row with damage and
  0 hits (rates read "--"). Other ticks (nine-digit effects) keep their own rows.
- **A record aimed at the attacker's own side is not damage.** In
  `append_damage_at` a record whose target ends at the same owner chain as the
  attacker (itself, its owner, an own summon, a sibling summon) is dropped before
  any damage accounting, the fight window and the last target; the drain and
  regeneration amounts it carries still go to healing.

## Fights, the meter between fights, and the fight report

**Activity, not damage, decides.** `DataStorageInner::activity_at` records the
last hit that belongs to your fight (`is_own_fight`): one you dealt, one you
took, or one your party landed on your current target. Before the game has said
who you are, every hit counts. Other people fighting nearby move nothing. The
snapshot loop then does two things each tick:

- **Idle reset.** `idle_reset_secs` (default 300, 0 = never) after the last
  activity, the fight is saved to History and the meter starts clean
  (`end_idle_fight`). A meter holding only strangers' fights is cleared without a
  record, measured from its start time.
- **Visibility.** With `hide_when_idle` on, the loop says
  `MeterEvent::DpsIdleHidden(true)` while there is no activity since the last
  reset and no peek is running, and `DpsIdleHidden(false)` at the first activity
  or when a peek starts. The core peeks 4 s after Reset; the host asks for others
  with `DpsMeter::peek_overlay(secs)`. It is off by default.

In the app, Settings › Overlay › Window › "Hide between fights"
(`windows.hideMeterBetweenFights`, off by default) sets `hide_when_idle` on the
running core at once. The event goes through `bridge.rs` to `MeterModel` on the
UI thread; the hub (`toolbar/mod.rs`) observes the model and hides the meter's
window (`SW_HIDE`) or shows it without activating it (`SW_SHOWNOACTIVATE`, then
raised in the topmost band without activation), so the game keeps the focus.

- The window stays up while a fight summary (kill or wipe) is showing, and while
  the first-open prompt waits for an answer.
- Opening the meter, from the toolbar or at launch, peeks 10 s. Clicking the
  toolbar's meter icon while the meter steps aside peeks 10 s instead of closing
  it.
- A meter that steps aside is still open: fights are tracked, the lock shortcut
  works on it, and Alt+Shift+CapsLock showing everything again leaves it aside
  unless a peek or a summary applies.
- A reminder card goes under the toolbar while the meter steps aside, as while
  it is closed, so notifications are never hidden with it.
- The pointer, the "…" menu and an unlocked meter do not keep it up.

**No window call on the snapshot thread.** It deadlocks against
`stop_dps_meter` joining that thread. The core only emits the event, and
`bridge.rs` hands it to the UI thread.

**Off the UI thread.** The PvP window reads the meter's watch list and kill
stats on the background executor, one read at a time with the ticks skipped
meanwhile, and drops answers that predate a name change or a clear. Saved and
live fight reports are built on the background executor, and the report window
opens when the report is ready. History delete and reload stay memory-only on
the UI thread: the core writes the file on its own thread.

**Overlay frames** go out when their content differs from the last frame sent
(`CombatSnapshot::shows_the_same_as`): a name, a boss's health, a buff starting
or ending all count. The frame's clock (`time_now`) and a buff's `coverage`,
which follow the wall clock, do not, so an idle overlay is sent nothing.

**Personal bests** (`crates/core/src/personal_best.rs`) keep your best DPS per
character and boss in `history/personal_bests.json`, apart from the history so
deleting it keeps them. They learn from every record the history saves, and are
seeded from the history on disk on first run. A fight counts when the target is
a boss and it lasted at least 15 seconds. The meter also saves the fight on
`stop_dps_meter`.

**Fight summary.** A boss whose health reaches zero gets a final summary. A boss
that goes quiet for 15 seconds gets a provisional one, which gives way if the
fight resumes and is replaced if the boss then dies, so a phase transition
cannot swallow the real summary.

**Capsule mode** is `overlay.layout = "compact"`: one line with your DPS, place
and time, expanded while the pointer is on the card or a summary is up. The
collapse waits 350 ms so crossing the card's edge does not make the window jump.

### The fight report's data

`crates/core/src/report/` records what the end-of-fight report needs beyond
damage dealt, and builds the report.

- **Recorded while playing** (`report/events.rs`, in `DataStorage`, cleared by a
  reset like the stats): hits taken, heals and player deaths, each with its time,
  in bounded queues (60,000 hits, 60,000 heals, 2,000 deaths; the oldest go
  first). Nothing here touches the damage stats, the target or the boss focus.
- **Hits taken**: a `04 38` record with an amount whose skill is not a player's,
  and a `05 38` damage tick (mode `0x0A`) of such a skill, on a target that is
  not a monster, from a source that is not a known player or a player's summon.
  A boss's adds are summons owned by the boss, so their hits count at the full
  amount; the owner chain is followed four steps and a hit is skipped only when
  it ends at a player.
- **Heals**, kept per skill and kind (`HealKind`): *Direct*, a hit of a skill in
  `healing_skill_code.json` with an amount on a non-monster target; *OverTime*, a
  `05 38` tick of mode `0x0B` of a class skill (8-digit code) with its tick
  amount; *Drain* and *Regeneration*, the extra amounts of a damaging hit
  (`F & 0x200`, `F & 0x400`), credited to the attacker. Overheal cannot be
  separated, and a shield's amount cannot be told from a heal's.
  `heal_amount` (the `F & 2` block) is read but not counted.
- **Scope**: the report covers the window from the first to the last hit on the
  boss plus 5 s (a wiped party dies just after its last blow). Players are the
  ones who hit the boss and the ones who healed them. Buff uptime is clipped to
  the window with overlaps counted once.
- **Saved**: `HistoryRecord.ledger` (`FightLedger`, serde default `None`) only on
  boss records. Older files load unchanged and open with Healing, Damage taken
  and Deaths "not recorded" (`None`, not zero); no migration rewrites them.
- **The data API**: `FightReport::from_record(&HistoryRecord)` for History,
  `DpsMeter::fight_report(target_id)` for the fight on the meter (newest saved
  record of that target when the meter has been reset). One value carries the
  header, the five key figures, one row per player and, per player, the hit types
  as counts, the skills, buffs, heals and damage taken.
- **Summoned objects are their owner**: a boss spawns many entities (projectiles,
  zones, copies of itself) whose spawn record names the boss as owner. A hit and
  a killer carry `owner_mob_code`, the mob code at the top of the owner chain
  (`owned_by_mob_code`, four steps), and
  `CombatEvents::credit_unfought_sources_to_their_owners` (run in
  `DataStorage::fight_ledger_inputs`) swaps the source for it, unless players ever
  damaged the entity itself: that is a real add and stays its own source.
- **Buffs follow the owner chain too**: `save_buff` credits a buff to the top of
  the caster's owner chain, as damage is. In the report, casters it does not know
  (`UNKNOWN_CASTER_ID`, 0) are one row per buff with the longest uptime among
  them.
- **One buff per name and caster**: several skill ids can show one name.
  `engine/buff_groups.rs` joins a caster's ids on one target by their English
  name (`skill_names::english_name`, from the bundled `skill_names_en.json`) and
  takes the union of their intervals, in the fight ledger and in the live buff
  summaries; the id with the most uptime stays as the representative.

## Saving history and personal bests

`crates/core/src/background_save.rs`. Neither store writes on the thread that
changed it: a save or a delete changes the records in memory, which is what every
reader sees, and sets a flag; a writer thread per file copies the records under
the lock, releases it, then serialises, compresses and writes (atomically,
`*.tmp` then rename). A failed write is logged and retried;
`DpsMeter::save_now` writes what is still pending on the calling thread and runs
on `stop_dps_meter`, on quit and on drop.

A `combat_history.bin` or `personal_bests.json` that cannot be read (bad magic,
version, checksum, compression or JSON) is renamed `<name>.bad-<seconds>` and the
store starts empty; it is never overwritten, and the bests are not rebuilt from
the history. Only a missing bests file (a first run) is built from the history.
`personal_bests.json` is `{"version": 1, "bests": [...]}`; a bare list still
loads. The same set-aside rule applies to `settings.json` and `todos.json`.

## Debug builds

The workspace `Cargo.toml` builds every dependency at `opt-level = 2` in the dev
and test profiles (`[profile.dev.package."*"]`); the workspace's own crates stay
unoptimized. GPUI is a dependency here, not a workspace member, so without it its
layout (taffy), text and image crates ran unoptimized: the History window with
500 records took 633 to 732 ms a frame in a debug test build, 86 to 135 ms with
the profile (before its list was built with GPUI's `list`). A cold test build of
the app takes about 12 minutes with it instead of about 5 and a half; rebuilds
after a change to the workspace's crates are unaffected. Release builds are
unchanged.

## Distribution

A portable zip: the exe with the Administrator manifest embedded, its data, and
`WinDivert.dll` plus `WinDivert64.sys` beside it. One build script produces it,
so there is no second list of runtime files to drift from the first. Npcap
cannot be bundled (its free licence forbids redistribution), so each user
installs it themselves, or runs on the WinDivert fallback. The executable is
`belisk.exe`.

`cargo xtask dist` builds the release exe with the static CRT (`+crt-static`
through `CARGO_ENCODED_RUSTFLAGS` for that one invocation, with
`--target x86_64-pc-windows-msvc`, so normal and dev builds and `target/release`
are untouched), then checks the PE: no VC++ runtime or Universal CRT imports, GUI
subsystem, `requireAdministrator` in `RT_MANIFEST`, and `WinDivert.dll` as a
delay-load import. It refuses a locked exe, a failed build, a WinDivert file that
differs from the official 2.2.2 hash, and a dirty working tree unless
`--allow-dirty`. The zip in `target/dist/` holds `belisk/` with the exe, both
WinDivert files, `README.txt`, `LICENSE.txt`, `THIRD-PARTY-NOTICES.txt` (bundled
components in full, plus every linked crate and its licence from
`cargo metadata`, with non-GPL-3.0-compatible ones flagged) and
`source/…-source.zip`, a `git archive` of HEAD, which is the GPL source offer.
The version comes from `cargo metadata`, so nothing else holds the number.

`cargo xtask dist --no-source [--source-contact "<text>"]` writes the zip without
`source/` to `target/dist/no-source/`. README.txt and the notices then carry a
GPL-3.0 section 6(b) written offer instead: the complete corresponding source for
that version and commit (short hash of HEAD) free on request for at least three
years from the contact given (default "the person who gave you this copy"). The
dirty-tree refusal still applies, since the offer names a commit; the offer is
only honest while that commit is kept and can be handed over.

`cargo xtask dist` also writes the lean update archive
(`belisk-<version>-windows-x64-update.zip`: the exe and the two WinDivert files,
flat) and its `.sha256` beside the zip; `cargo xtask sign` writes the `.sig` with
a private key made by `cargo xtask keygen` outside the repository. dist fails
while `crates/app/update-key.pub` is the DEVELOPMENT placeholder
(`--allow-development-key` is for local testing only, never CI). How the app
checks, downloads, verifies and swaps, and its trust model, are in
[updates.md](./updates.md); CI and releasing are in [releasing.md](./releasing.md).

Code signing is not planned, so SmartScreen warns; the player README says so.
Running the zip on a clean machine is a manual check.

The name shown to users is "Belisk". About still credits the projects this is
forked from (see "Data folder" for the move off the earlier names).

## Live capture and the manifest

- **Live mode** is launching without `--replay` (`live_session.rs`). The meter
  starts with `start_dps_meter` (Npcap first, WinDivert fallback) and is stopped
  on quit. The development checks (`--capture`, `--overlay-probe`,
  `--font-probe`) never start live capture.
- **When the meter cannot start** (not elevated, no backend), the overlay shows
  the card "The meter is not running" with the reason, status "Stopped", and never
  "Waiting for the first hit". The capsule layout stays a full card in that state.
- **Manifest.** `crates/app/build.rs` embeds `windows/admin.manifest.xml`:
  `requireAdministrator`, PerMonitorV2 (`dpiAware true/pm`), Windows 10/11
  compatibility, SegmentHeap, and the Common-Controls 6 dependency, i.e. what
  GPUI's `gpui.manifest.xml` carries, with the execution level changed. A build
  with the `probes` feature embeds the same file with `asInvoker`, so the
  development checks run from an ordinary terminal. The `.rc` files embed the
  icon beside it.
  - GPUI's `gpui-pre-platform` forces its `windows-manifest` feature on, so a
    second `RT_MANIFEST` made the link fail (LNK1123), and features cannot turn it
    off. The root `Cargo.toml` patches `embed-resource` with
    `vendor/embed-resource`, a copy of 3.0.11 whose `compile()` returns early when
    the calling package is `gpui-pre`. Drop the patch when GPUI stops embedding,
    or when the pinned snapshot changes how it does it.
  - The manifest is linked into every binary of the package, including a bin's
    unit-test harness, which then cannot start unelevated (os error 740). The
    program therefore lives in `src/lib.rs` (`run()`), `src/main.rs` is three
    lines with `test = false`, and the tests run in the library, so
    `cargo test --workspace` runs unelevated.
- **Versions.** GPUI is `gpui-pre` and `gpui-pre-platform`, both `=0.3.7` (a Zed
  snapshot at `1a28cff` repackaged on crates.io as the `gpui-pre-*` family). Both
  are pre-1.0 and exact-pinned, so upgrade them together. No component library
  (GPUI Kit) is used: windows are built from our own components on plain GPUI.
- **Idle cost.** `gpui-pre-windows` starts a `VSyncProvider` thread that wakes on
  every display refresh and calls `RedrawWindow(RDW_INVALIDATE)` on every window.
  Nothing in this app requests a frame; that thread is the floor. App code has no
  redraw poll and no animation loop: a settled overlay draws nothing, and frame
  requests exist only for running tweens and GPUI's own one-shot fades.

## Overlay windows

What the overlays (the meter, the toolbar, every secondary window) need from
Windows, and how plain GPUI gets it (`crates/app/src/overlay.rs`):

|Requirement|How|
|---|---|
|Transparent, borderless, always on top, out of the taskbar and Alt+Tab|`WindowKind::PopUp` gives `WS_EX_TOOLWINDOW \| WS_EX_TOPMOST` and no `WS_EX_APPWINDOW`; `WindowBackgroundAppearance::Transparent` is real per-pixel alpha|
|Never takes focus|`focus: false` shows the window with `SW_SHOWNOACTIVATE`; `make_overlay` then adds `WS_EX_NOACTIVATE` and subclasses the window (`SetWindowSubclass`, removed on `WM_NCDESTROY`). The style alone does not stop a click (GPUI answers `MA_ACTIVATE` itself), so the subclass answers `MA_NOACTIVATE` while the window has `WS_EX_NOACTIVATE`: the click still reaches GPUI, which is not `MA_NOACTIVATEANDEAT`|
|Lock is click-through, toggled at runtime|`WS_EX_LAYERED \| WS_EX_TRANSPARENT` (plus `SetLayeredWindowAttributes(255)`) set and cleared with `SetWindowLongPtrW` on the UI thread|
|No frame|GPUI creates its pop-up with an overlapped style, so Windows gives it a frame and, on Windows 11, DWM draws a 1 px light border with rounded corners. `overlay::open` opens the window hidden, sets `GWL_STYLE` to a plain `WS_POPUP`, `DWMWA_NCRENDERING_POLICY = DWMNCRP_DISABLED`, `DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND` and `DWMWA_BORDER_COLOR = DWMWA_COLOR_NONE` (failures ignored, for Windows 10), then places and shows it without activation|
|Window exactly as large as the card|After layout the view measures the card and `overlay::fit_client_size` / `fit_to_children` resize the client area with `SWP_NOMOVE \| SWP_NOZORDER \| SWP_NOACTIVATE` (GPUI's `Window::resize` omits the last two, so it could activate the overlay over the game). No transparent margin is left to take clicks while unlocked|
|HWND|`Window` implements `raw_window_handle::HasWindowHandle`; `RawWindowHandle::Win32` gives it (`overlay::hwnd`)|
|Drag by the header|The header's title area records the press and the window follows the pointer (`overlay::WindowDrag`). `WindowControlArea::Drag` is not used on overlays: Windows reports the pointer crossing between client and non-client area as a leave, GPUI clears its hover flag on either, and controls that show on hover flashed on every mouse move|
|Per-monitor DPI|GPUI's embedded manifest declares PerMonitorV2 and ours keeps it|

Other rules:

- `Application::run` needs no admin rights and no activation; the overlays never
  call `cx.activate`.
- No drop shadow on overlay cards or menus: over a bright scene the window edge
  clips it into a faint hard-edged rectangle, and the room for it would take
  clicks while unlocked (`HTTRANSPARENT` only passes clicks to windows of the
  same thread, never to the game).
- Popovers and menus are children of the window root and drawn over the card.
  The root fits the window to all of its children, so the window grows while a
  popover reaches below the card and shrinks back when it closes.
- A menu or dropdown closes on a press anywhere else (an occluding scrim takes
  that press, so it reaches nothing below), on a second press on its opener, and
  400 ms after the pointer leaves the window (checked every frame). The leave is
  the `WM_MOUSELEAVE` Windows sends after GPUI's own `TrackMouseEvent(TME_LEAVE)`
  on our HWND; no hooks.
- Card actions respond when the pointer enters or leaves the card, not on moves
  inside it (hover is the element's enter and leave only).
- Motion: bars, row order and the boss gauge tween over 240 ms (`meter/motion.rs`);
  tool-row swaps, gauge, chips and the capsule fades (100–180 ms) use GPUI's
  one-shot `Animation::new`. Windows' "Animation effects" setting turns all of it
  off (`cx.set_reduce_motion`; GPUI does not read the setting itself).
- Game data is compiled into the exe (`include_str!` / `include_bytes!`) from
  `crates/app/assets/game/`; icons are Lucide SVGs in `crates/app/assets/icons/`.
  The exe needs none of these paths at run time. `build.rs` lists the skill icon
  folder (`game_data::skill_icon`: a six-digit id has its own icon, any other id
  uses its first four digits) and copies the WinDivert files beside the exe.
- The one keyboard-focus exception is the PvP add-to-watch field, see "Secondary
  windows".

## Fonts

GPUI's `Font` is family, weight, style, features and fallbacks; there is no
width, and `gpui-pre-windows` matches the family name against DirectWrite
font-set family names only. So Bahnschrift SemiCondensed (the originally approved
face) is not reachable: the name is not found and GPUI silently falls back to
Segoe UI. `Segoe UI Variable` does not resolve either (the families are `Segoe UI
Variable Text`, `Display` and `Small`).

The overlays use **Barlow Semi Condensed**, the four static weights
400/500/600/700 from `google/fonts` (`ofl/barlowsemicondensed`), embedded with
`include_bytes!` and registered through `add_fonts` (`crates/app/src/fonts.rs`;
licence `crates/app/assets/fonts/OFL.txt`). DirectWrite files these faces under
the family **`Barlow`** with a semi-condensed width, so GPUI must ask for
`Barlow`; `Barlow Semi Condensed` does not resolve. There is no weight 650, so
the local player's name is 600. A system-wide Barlow install is untested.

Figures use the `tnum` feature (tabular figures): without it "1111111111" is far
narrower than "0000000000"; with it they measure the same. The `probes` build's
`--font-probe` shapes a string in each candidate family and draws a specimen sheet.

## Translations

- **Setup.** `crates/app/i18n/<lang>/belisk.ftl` (English is the fallback and the
  reference) is embedded in the exe by `rust-embed`, so the zip stays one exe
  plus the WinDivert files. `fl!("key", arg = value)` (Fluent through `i18n-embed`
  and `i18n-embed-fl`) checks the key and its arguments against the English file
  at compile time. The one setup is `crates/app/src/i18n/`.
- **Language.** Settings → General → Language is System, English or French;
  System follows Windows' display language (any French → French, else English).
  It is stored as `general.language` in `settings.json` and applied by
  `settings/effects.rs` at start and on every change, with no restart: the loader
  switches, the `Locale` entity notifies and every window redraws.
- **What is translated.** Every word the app writes: labels, units (KB → Ko),
  weekdays and months, durations ("1 d 3 h" → "1 j 3 h"), plurals (Fluent uses
  CLDR: French 0 and 1 are singular), the copied reports. Figures keep their
  format (48.10M, 9,840) in both languages. Skill, mob and dungeon names are
  looked up by id in the language in force, see "Names and data". The core's own
  messages stay English: the app wraps them in translated sentences where it
  shows them. French keeps English-style gaming abbreviations (DPS, buff) as they
  are.
- **Test.** `i18n/tests.rs` fails when a key exists in one language only, when a
  key reads different arguments, or when a file does not parse.

## The meter: data and model

### Session aggregate

`CombatSnapshot.session` (`models/session.rs`, built by `engine/session.rs` from
the per-target overview the snapshot already computes, so it adds no map clones).
Per player: damage, combat time, DPS, share and combat power; for the session:
total damage, mobs fought and the session clock. Rows are sorted by damage,
honour `hide_unknown_players` and `max_player_count` (5–20; the local player,
found by id or by name, takes the last row when ranked below the cut), and the
shares count everyone, hidden players included. `boss_only` is applied at ingest,
so a Boss-only session holds bosses only. A reset or idle reset clears it.

- **Which damage counts.** Damage to mobs. Targets that are named players (PvP, a
  mob hitting a player) are left out; the PvP boards keep them. Only true mobs
  count as mobs.
- **Targets** are counted per spawn: a pack of thirty is thirty.
- **Combat time.** On each target a player's window runs from their first hit to
  the last hit anyone landed on it, at least 1 s: the span the per-target DPS
  already divides by, so a one-target session reads like that target's board. A
  player's combat time is the length of the *union* of their windows, so two mobs
  fought at once count once and the walk between packs counts not at all. Session
  DPS is `damage / combat time`. The session clock is the union of every target's
  window.
- Replays compress time (timestamps are taken at replay time), so replayed
  session DPS figures are large; the figures are right for live play.
- `engine::session::session_skill_totals(by_target_player_skill_stats, actor,
  is_mob)` sums a player's skills per skill code across every mob (skipping
  player targets). It adds nothing to the capture or snapshot path: the app calls
  it on each snapshot only while a detail window is open on a Session source, and
  the detail model redraws only when the result changed.

### Boss rules and the mob catalogue

`mobs/catalog.rs`. `MobCatalog::is_boss(code)` is the only boss decision: the
snapshot's `TargetInfo.is_boss`, the Boss-only filter and the summon parser all
ask it. Order: the user's exclusion, the user's addition, the bundled list, then
the HP rule (HP over 1 million, `POSSIBLE_BOSS_HP_THRESHOLD`, from global
observations: bosses 3.6–6.8 million, trash at most about 0.3 million; when
`show_possible_boss` is on, which the app turns on in live and replay mode). The
HP rule's set survives a reset: it is a fact about the mob.
`set_boss_correction(code, Some(Boss | NotBoss) | None)` is the API for a
correction; the meter's "…" menu calls it. Names: the bundled name first, then a
name from `mobs.json` for codes the bundle lacks.

### `mobs.json`

In the app data folder (`%APPDATA%/Belisk/mobs.json`).

```json
{ "version": 1,
  "mobs": { "2100793": { "name": "…", "nameSource": "bundled|user|reviewed",
                          "maxHpSeen": 1234567, "firstSeen": 1790000000,
                          "lastSeen": 1790000900, "timesFought": 3,
                          "bossCorrection": "boss|notBoss" } } }
```

Every field but `timesFought` is optional. Times are epoch seconds. There is
**no zone field**: the core never learns which zone or dungeon a mob was met in.
No player data is stored, so the file is safe to share; it never goes into the
repository. The network sends mob codes, never names, so missing names are
proposed from public community data and confirmed by the user.

- **What is saved.** User corrections always. Everything else (unknown mobs, HP,
  counts, names) only when `DpsMeterConfig.collect_mobs` is on: the "Collect mob
  data" switch in Settings → Capture, off by default and applied live.
- **When it is written.** A change sets a flag; a writer thread
  (`mob-store-writer`) writes when the file is dirty and the last write is at
  least 30 s old, so the first change goes out at once and a burst costs one
  write. A fight ending (reset, idle reset, identification reset, stop) is such a
  change, and quitting calls `save_now()`, which writes on the quitting thread.
  Writes are a temp file, `sync_all`, then rename. The packet and snapshot threads
  only take in-memory locks and set the flag; a failed write is logged and
  retried.
- **Bad files.** A file that does not parse, or has a newer `version`, is renamed
  `mobs.json.bad-<epoch seconds>` and logged, and the meter starts with no mob
  data. It never crashes.

### View mode and the app model

`crates/app/src/meter/board.rs` is the pure state machine (`BoardChoice`): Auto
(Session in the open world, Target while a boss is engaged) or a pin. Clicking
the inactive side of the footer switch pins it; clicking the active side while
pinned returns to Auto, and a pin dot marks the override. A pin ends on reset, on
an emptied meter, and when a boss is engaged. A pinned Target board with no boss
engaged follows the current target. `MeterModel` exposes `active_board()`,
`is_board_pinned()`, `select_board(side)` (the caller notifies), `session()` and
`live()` (rows), `header_name()`, `elapsed()`, `footer()` (`Session { damage,
targets }`, `TargetPace` or `TargetPartyDamage`) and `identified()`.

`crates/app/src/bridge.rs` is the `EventSink`: it only pushes into an unbounded
channel, so the meter's threads never block and never touch a window. One
foreground task sleeps on the channel, applies whatever has piled up as one batch
to the `MeterModel` entity, and calls `cx.notify()` only when something a view
draws changed (memory figures, history and personal-best events do not redraw).

### Sticky boss focus

The game's "current target" is whatever was hit last (by you, or by anyone when
you are not identified), so in a boss fight with adds it jumps between the boss
and trash. `meter/focus.rs` (`BossFocus`) keeps the Target board on one boss. The
first snapshot whose current target is a live boss **that is your fight** engages
it, and the Target board, its gauge, time, pace and the header name then follow
*that boss*, from the snapshot's per-target overview (`by_target_player_stats`,
`target_infos`), whatever is hit meanwhile.

"Your fight" (`Involvement`): once the game has named the local player, a boss
qualifies only if **you hit it** (your row under the boss in
`by_target_player_stats[boss]`) or **it hit you**. The core records every hit as
`by_target_player_stats[target][attacker]`, mobs as attackers included, so "it hit
you" is the boss's id under your own actor id
(`by_target_player_stats[you][boss]`); that is how a tank or healer who only takes
hits still engages it. Being merely the fallback "last target" is not enough:
until you hit something, the core's current target is *anyone's* last target,
which once put the board on a world boss in a PvP zone that the player never
touched. Before identification the meter cannot tell, so any boss target engages;
if one was engaged then and, once identified, you have neither hit it nor been
hit by it, it is released ("not your fight"). A stranger's boss that dies gets no
summary. The decision is logged (reason "you hit it" or "it hit you"). The focus
is let go when:

- the boss **dies** (the final summary follows, then the automatic board is
  Session again; a dead boss is never engaged again);
- **you move to another boss**, by the rule below (`Release::MovedToAnotherBoss`);
- it goes `QUIET_RELEASE` (2 minutes) without a single hit on it, which also puts
  up the provisional "Fight ended" summary; a boss let go this way is engaged
  again when someone hits it;
- it is gone from the snapshot, or the user ruled it "not a boss";
- the meter is **reset** (button, shortcut, idle reset or character change);
- the local player's **actor id changes**: ids are per-zone handles, so that shows
  a zone change. The identification packet is not re-sent on a zone change, so
  the core finds your actor from your own remain-HP packet instead
  (`00 8d <id> 01 01 03|06|01 <hp>`: the first skip varint after the target is 1;
  `StreamProcessor::own_actor`, `DataStorage::bind_own_actor`), at every capture
  start and every new game connection, which the app sees as this change. A
  connection keeps the first actor the signature showed, unless `0x33 0x36` names
  the character: that stays authoritative for id and name. A name already known
  (this run) carries over to the new id only provisionally (a new connection may
  be another character picked at character select): once the actor's skills show
  a class, the name stays if that was the class the name had, otherwise the most
  recent remembered character of the shown class takes over, else "You". The core
  parses no despawn or entity-removed packet, so the quiet rule covers a boss
  left behind. A re-sent identification with the same id is not a zone change.

Bosses fought together (two sub-bosses of an expedition or conquest, a sub-boss
beside the main boss) must not flip the board: the focused boss A stays, and
another qualifying live boss B takes over only after **you have hit only B for
`SWITCH_AFTER` = 20 s**: you hit B since your last hit on A, and that last hit on
A is at least 20 s old (counted from the engagement if you never hit A, e.g. it
only hit you). Any hit of yours on A restarts the count. "Your last hit on a
target" is `TargetInfo::target_last_time[you]` (the core's per-player last-hit
time, summons merged into the owner) against the snapshot's `time_now`. Before
identification your hits are unknown, so B takes over once it has been the
*current target* without a break for 20 s. Moving away releases A without a
provisional summary and does not record it as finished, so it can come back under
the same rule. When A dies, its final summary shows as usual and B (the boss you
hit last, or the current target) engages at once; B does not take A's summary
down. With no boss focused, the boss you hit within the last 20 s engages even if
the current target is trash.

A user pin overrides the board but does not release the boss: unpinning goes back
to it. Hits are noticed by the boss's fight length growing; the quiet limit is
armed on the model's timer, since snapshots stop when nothing is hit.

### Whose summary, whose row

- The summary is built for the target the fight tracker named (the boss), not the
  current target, and from the full per-target overview. The Target board ranks
  that full overview (not the core's player-limited list) and keeps your row in
  the last place if you rank below the cap.
- "You" on any board is found by **name** first. An actor id is trusted only on a
  row nobody has named, because ids change between zones and a stale one can
  belong to someone else by now. If the meter knows your actor but not your name,
  that nameless row is yours and reads "You" (meter, summary text, detail,
  history), resolved in the app by `format::unnamed_player`; nothing writes "You"
  into the core's data or the history records, so a name that arrives later is
  still a real name. If the meter does not know your actor, or you did no damage
  to the boss, the summary is about the party ("Party DPS", player count, no
  rank, share, crit, skills or personal best) and the copied text starts "Party
  ...". Nobody else is shown as you. The capsule's lead is `Lead::You` or
  `Lead::Party`.
- **Your name without the name packet.** The game does not send `0x33 0x36` on a
  zone change, and a meter started after login may not see it for hours. Whenever
  `0x33 0x36` names your character the app remembers it in `settings.json`
  (`knownCharacters`: name, server, class, and the 6-byte character key found in
  the packet; 20 at most, most recent first; kept by "Reset settings"), and hands
  the list to the core. When your skills show a class, the core names your
  unnamed actor after the most recently seen remembered character of that class,
  never before (a class is not guessed); with none it stays "You". A same-class
  alt shows the remembered name until its own `0x33 0x36` arrives. Matching the
  key against the `02 97` party roster is not done (roster entries are not parsed
  and the packet is rare). Until the meter knows your actor the card says "Start
  before login, or relog via character select"; the actor usually shows within
  seconds from your own remain-HP packet.

### Class shades

`meter/shades.rs`: when several players share a class, each gets a shade of the
class colour (rule and step values in `docs/gpui-mockups/approved/tokens.md`,
class shades). The model keeps one `ShadeBook` for both boards, keyed by player
name (by id while unnamed), so a friend has the same shade on Session and Target
and keeps it when ranks swap; an emptied meter or a reset clears it. Shades are
assigned when the boards are rebuilt from a snapshot, in board order for players
first seen in the same snapshot, and the row carries the final colour, so drawing
does no colour work. The capsule badge uses your shade; summary skill bars keep
the plain class colour. A sixth player of a class with four shades (or a fifth
ranger or cleric) takes the class colour again.

A player's class colour is their meter shade wherever the meter has drawn them
since the reset (`ShadeBook::known`, read through `MeterModel::player_colour` /
`actor_colour`; nothing is assigned by looking): live and Session detail skill
bars, buff coverage by caster, PvP damage rows. Anyone the meter has not drawn
gets the plain class colour. A saved fight shades its own players afresh, most
damage first (`history::record_colours`), so a record's player list and its
history-mode detail agree.

### Decision log

The "Decision log" switch (Settings → Capture, stored) turns on the app's own
decision log. It does not touch the core logger's debug level, which also writes
a line per hit and buff on the packet thread and stays off. It writes lines
starting `[meter]` to `belisk.log` (live) or `belisk/gpui-app.log` in the temp
folder (replay), only when the decision changes: the board and why (automatic or
pinned), boss focus engaged and released with the reason, whether and how your
row is on the Target and Session boards (not identified, matched by name or by
id, below the row cap, absent), identification and zone changes (the signature,
the remembered name applied or rejected, the game's name replacing them),
summaries and who they are about, and boss corrections. Lines may hold player
names, as the log already does; the log stays on the machine. With the switch off
the cost is one flag check.

### The meter's rules and layout

`crates/app/src/meter/`: `view.rs` (the card, capsule and summary), `model.rs`
(events folded into prepared rows; personal bests asked of the core once per
character and boss), `live.rs` and `summary.rs` (what each card shows, worked out
once per snapshot), `fight.rs` (when the fight summary goes up and comes down, a
unit-tested state machine), `motion.rs`, `settings.rs` (`OverlaySettings`) and
`shades.rs`.

- Snapshots of a target without a mob code are skipped; at most `max_player_count`
  rows and your row is never the one dropped; bars are damage over top damage with
  a 6 % floor; contribution or share; combat power grouped, DPS compact; masking
  covers players and the target; personal-best pace after 10 s of a boss fight;
  the summary is final when the boss dies, provisional after the quiet limit,
  never for fights under 15 s, taken down when the fight resumes, the next boss
  starts, or after a minute; Copy writes the battle report (live) or the summary
  text.
- **Header.** The current target's name on the left (`muted` on the Session
  board, `text` on the Target board, cut with an ellipsis only when it runs out of
  room), the board's clock on the right. Under the pointer the clock cross-fades
  to the tools: reset · capsule · lock · copy · … · close, evenly spaced. Copy
  turns into a check for 1.5 s. Reset shows only with a running meter, lock only
  when `Alt+CapsLock` registered (otherwise nothing could unlock the overlay).
  The window never resizes on hover.
- **Boards.** Session: rows of name + combat power, quiet DPS with a faint "/s",
  damage in bold, share %. Target: the same with the boss gauge line. A board
  change cross-fades in 150 ms and clears the row tweens; the card's height
  follows the board.
- **Footer, always shown.** Session: damage and the target glyph with the count.
  Target: the personal-best pace once a boss fight is 10 s old and a best exists,
  else the party's damage. Then ping and the fixed-width (100 px) [Session |
  Target] switch; its highlight slides in 120 ms. Locked, the switch shows and has
  no handlers (the window is click-through anyway). Its tooltip is drawn inside
  the card above the footer after 500 ms of rest, and none shows for the active
  side while the choice is automatic.
- **Capsule.** Session: your rank, your damage, "Session" and the clock. Target:
  your DPS, the clock and the pace. Until the meter knows which row is you it
  shows the party's figure without a rank badge ("party damage" / "party DPS"),
  as the summary does. Waiting: "DPS Meter · Standing by". It opens by resizing
  the window at once and fading the card in over 180 ms.
- **Boss correction ("…").** A click opens a menu: "Treat <mob> as a boss", "Not a
  boss", "Automatic", the current one ticked (`MeterModel::correction_target`,
  `set_boss_choice`). Choosing saves through `MobCatalog::set_boss_correction`,
  patches `is_boss` on the snapshot the model holds and re-runs the board choice,
  so marking a mob a boss turns an automatic Session board into the Target board
  at once; the view pin stays independent. Without a target with a mob code (or
  without a meter) the items are faint and inert under a note. The menu is drawn
  in the overlay window, which grows to hold it and shrinks back when it closes.
- **Lock (Alt+CapsLock) applies to the meter only.** `hotkey.rs` registers it with
  `global-hotkey`; its hidden window gets `WM_HOTKEY` through GPUI's message loop,
  the handler only queues, and the toggle runs on the UI thread.

## Secondary windows

History, detail, report, PvP, Settings, the first-open prompt, the reminder card,
What's new, to-dos, Resources, the Pets window and the soul tracker card live in
`crates/app/src/secondary/`.

- **One window per kind**, in a registry (`registry.rs`, `WindowKind`). Opening a
  kind that is open **raises it without activating it** (`SetWindowPos(HWND_TOPMOST,
  SWP_NOACTIVATE)`); it does not toggle, since a window can sit behind others. The
  detail window is also pointed at the new player or board. A window is forgotten
  when its view is released, however it ends (its ×, or the app quitting, which
  closes them all).
- **Overlay windows** use `overlay::popup_options` and `overlay::make_overlay` as
  the meter does, and are as large as their card. Settings is the exception: a
  normal window with its own 36 px title bar (`WindowControlArea::Drag`/`Min`/
  `Close`: crossing into them clears GPUI's hover flag, which nothing in that
  window reads), a taskbar button, activated. It opens just after the menu click,
  since it reads the meter's lock while the meter's window is still dispatching
  that click.
- **Placement** (`placement.rs`, tested): a tool opens beside the meter while it
  is open (detail to its right, history below it, PvP to its left; each flips to
  the opposite side when it does not fit, then is pulled inside the work area),
  else under the toolbar (`secondary::Stacking`, `placement::Stack`). Tools do not
  remember a place of their own and do not follow the meter when it moves, except
  the reminder card (below).
- **Entry points.** The toolbar opens tools. A press on a meter row opens the
  detail window: on the Target board, that player on the board's target (the
  engaged boss, else the one being hit); on the Session board, that player's
  session totals. Rows are not drag areas (the header's name area is), so a click
  and a drag do not conflict. A locked meter is click-through, so rows are not
  reachable then. History players open the detail in history mode; PvP rows open
  it too (a damage-taken row: that player on you; a damage-dealt row: you on that
  player).
- **PvP mode** (`DpsMeter::set_pvp_mode`) is on from the moment the PvP window
  opens until it closes.
- **Shared drawing.** `src/style.rs` (`Scale`, the class-coloured bar, the health
  fill) and `secondary/chrome.rs` (the overlay.css classes as functions: card,
  header, buttons, chips, tabs, table head, rank badge, inputs and selects,
  popover and menu items, empty state, footer, tooltip, `scroll_list`,
  `scroll_rows`, `scrollbar`). `chrome::Anchor` records where a select or field
  is in a canvas prepaint and asks for one more frame only when it moved.
- **Scrollbar** (`chrome::scrollbar`, used by `scroll_list` and `scroll_rows`, so
  by every secondary window but Settings). It paints the approved 3 px thumb in
  an 8 px gutter from the scroll state of the same frame (GPUI draws none), so
  scrolling redraws it without another render; views know their row heights, so
  whether a list overflows is decided before layout. It takes the pointer as
  Zed's scrollbar does: the gutter's hitbox is `BlockMouseExceptScroll`; a press
  in it, heard in the capture phase before any element, is the thumb's alone (no
  window drag, no focus): on the thumb it grabs it, beside it the thumb's middle
  comes to the pointer and is grabbed there. While it is held, every move in the
  window (over the rows, outside the list) drives the scroll and the release ends
  it, both in the capture phase; the grab is the element's keyed state. The
  previous thumb heard everything in the bubble phase, the press only while its
  own hitbox was hovered; players could not drag it, for a reason no check here
  reproduced. The wheel is the list's own.
  `--capture --fixtures` checks it on History and the Pets window through
  Windows' mouse messages posted to the window (`capture_secondary::check_scrollbar`:
  a press on the thumb, moves with the button held down the gutter, over the
  rows and past the window's bottom edge, the release, then a press above the
  thumb), so GPUI's own message handlers and mouse capture run. The previous
  thumb passed that check too, so it does not show what stopped a physical drag.
- **Long lists** (History's records, the Pets window's lines): GPUI's `list`
  through `chrome::Rows`, which builds only the items in and near the view each
  frame. A view gives `Rows::set` each item's height (known before layout:
  fixed rows, labels, gaps, an opened record's players) and the focus handle of
  the control it holds; the list's items are replaced only when those change, at
  the same scroll in pixels. The thumb, `Rows::reveal` (the keyboard bringing a
  row into view) and `Rows::shows` work from those exact heights, not from the
  heights the list has measured so far. An item whose control has the keyboard is
  drawn wherever it is scrolled. Items need `w_full()`: `list` lays each one out
  as a root, and without a definite width a row does not truncate. History drew
  all 500 records each frame: 633 to 732 ms a frame in a debug build, 86 to
  135 ms with optimized dependencies, about 2.5 ms with `list`. A control out of
  view is not drawn, so GPUI's own Tab order cannot reach it: History moves Tab
  and Shift+Tab itself, through the boss records' report buttons in order and
  round, and scrolls the reached record in (`Rows::reveal`), as the Pets window's
  arrows do along its pins.
- **Data per window.** History (`history/list.rs`, pure and tested): records
  newest first, filters by target and by main player with counts (a filter nobody
  has any more falls back to all), local times in the Windows zone (`when.rs`:
  "Today 21:42", "Yesterday 22:07", the weekday for two to six days back, then
  "3 Oct 20:42"); deleting asks again in the button itself (reverts after 4 s);
  it refreshes on `MeterEvent::HistoryUpdated` through the bridge. Detail
  (`detail/data.rs`): the Target board's figures for one player on one target
  (skills with hits, share, min, max, average, the hit-type rates, multi-hit
  breakdown, spec slots; buffs of the player and of the target), the Session
  board's skills summed across every mob, or a saved record, frozen. PvP
  (`pvp/data.rs`): the watch list (add and remove by name, shown in the order
  added, "Not in range" for a name the game has not shown; **your last attacker is
  pinned on top, is not removable, and shows once even when also watched**),
  suggestions from players seen this session, kill stats sorted by damage or
  kills, damage dealt and taken; read from the meter every 500 ms and redrawn only
  on a change.

### Typing: the one focus exception

The text fields are the only places an overlay takes keyboard focus
(`secondary/typing.rs`, used by `pvp/view.rs`, the to-do and Resources windows'
fields and the Pets window's search).

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
   activate the next window in the z-order, and it skips `WS_EX_NOACTIVATE`
   windows when it chooses, so our meter is not picked; that is the game, or
   whatever the player was using. `SetForegroundWindow` is never called on another
   process's window. The window calls run on the next turn rather than inside a
   GPUI event handler, because `SetWindowPos` and `ShowWindow` send messages back
   into GPUI synchronously.
4. **The request in flight.** The foreground request runs on the turn after the
   press, so the edit can stop before it is answered. `typing::Edit` (Idle,
   Starting, Editing; unit-tested) tracks it: the request is made only while still
   Starting, and if the edit stopped while it was answered (or the view is gone),
   focus is given back at once. Closing the window (×) and quitting mid-edit also
   give it back (`on_release`, `on_app_quit`), with the window handle kept from
   creation so a failed lookup cannot skip it.

That Windows really hands focus back to the game is a manual check: an agent run
must not take the user's foreground (type a name, press Enter, keep playing
without clicking the game).

The text input (`secondary/text_input.rs`) is minimal, adapted from GPUI's own
`examples/input.rs` (Apache-2.0): typing, IME composition, caret keys, Ctrl+V,
Enter, Escape, a 32-character limit, no selections or mouse dragging. It draws
only text and caret; the PvP view draws the box around it.

## Settings

- **Store.** `%APPDATA%/Belisk/settings.json`: versioned JSON, missing fields take
  defaults, unknown fields are ignored, values are clamped on load. Writes are
  atomic and happen 400 ms after the last change, plus once on quit. Replay and
  capture runs use an in-memory store and never touch the file.
- **Live application.** `settings::Settings` is the one entity everything
  observes. A change replaces `MeterModel.settings` (the overlay and every
  secondary window read it), switches the decision log and the language, and,
  150 ms after the last change, applies the core options through
  `current_config()` + `apply_config()` (never a stored full config: the PvP
  window switches `pvp_mode_on` on the running meter). The capture backend
  priority is read only when the meter starts, so it applies at the next launch.
- **Defaults.** "Detect possible bosses" is on (the 1M HP rule). The lock is a
  live toggle, not stored. "Players shown" (5–20) caps the rows of both boards and
  the core's player limit.
- **Reset settings** puts the settings back to their defaults. History, personal
  bests, recordings, boss corrections in `mobs.json` and the remembered
  characters are kept.
- **Preview.** The real `MeterView` (`MeterView::preview`: zoomed so the plain
  card is 236 px at 100%, never under the pointer, never fitting its window) on a
  model fed `sample.rs`'s boss fight, re-set on every change. A card wider than
  the stage (Total damage, a larger content scale) is drawn at the largest
  content scale that fits.
- **Own controls, no Kit** (`settings/widgets.rs`): switch, slider (press, drag,
  keys), segmented control (one tab stop, arrows), select (combobox with a
  listbox), buttons, sidebar tabs, tooltip, dialog. Each has an AccessKit role,
  label and state (toggled, numeric value/min/max/step, value text, expanded,
  selected) and the slider answers Increment, Decrement and SetValue.
- **Keyboard and focus, as in a browser.** Tab/Shift+Tab walk the sidebar then the
  page in reading order and scroll the page to the focused control; Space/Enter
  press (on release); arrows, PageUp/PageDown, Home/End move sliders, segmented
  controls, the list and the sidebar; Escape closes the list or the Reset dialog
  and returns the focus to its opener; the dialog keeps Tab inside it. A press on
  anything but a control (or outside the open list) takes the focus to the window
  and closes the list. The cream ring shows only when the focus came from the
  keyboard. GPUI's test platform measures text as zero-width, so layout is checked
  by the captures (see "Checking without showing anything"), keyboard and focus by
  the tests in `settings/view.rs`.
- **Settings window.** `Page` opens a section or About › Changelog from outside.
  Opening or reopening it brings it to the foreground; it never stays always on
  top while the player uses the game.

## Event reminders

`crates/app/src/reminders/`. A heads-up before the game's recurring timed events
(Shugo Festival, Dimensional Invasion, Rift portals and others, some on by
default). A lead of 1, 2, 3 or 5 minutes (default 1), a sound switch and a master
switch live under `settings.json` → `reminders`; events starting in the same
minute share one reminder.

- **Data source.** MetaBot's event tables (`metabot.gg/en/aion-2/events`, as
  published for game build 0.0.4387.0), kept as data in `reminders/events.rs`. Its
  times are server times; MetaBot assumes Europe = Berlin without confirming it.
- **Clock rule.** The rules are read on the PC's local clock (Windows' time zone,
  queried per instant), which equals server time for an EU player. Hourly events
  follow the real hour; fixed wall times (the 3-hourly, weekly and daily events)
  happen once per day, and a time skipped by a daylight-saving changeover happens
  at the end of the gap. Details are at the top of `reminders/schedule.rs`, with a
  test for each changeover.
- **Runtime.** `Reminders` (a GPUI entity on the UI thread) sleeps on a GPUI timer
  until the next reminder, at most 30 s at a time, so a PC that slept or a changed
  clock is noticed within 30 s; a reminder whose event has already started is
  dropped. It re-plans when the Settings change, sends `ShowReminder` for the view
  to draw and plays Windows' own `Media\chimes.wav` (`PlaySoundW`, asynchronous,
  nothing bundled; the "SystemNotification" sound if the file is missing). Replay,
  capture and probe runs have no schedule and no sound.
- **The card** (`reminders/view.rs`, approved `reminders.html`) is its own overlay
  window (`WindowKind::Reminder`): frameless, topmost, tool window,
  `WS_EX_NOACTIVATE`, shown with `SW_SHOWNOACTIVATE`, so it never takes focus.
  It sits 6 px × content scale above the meter on its left edge, below it when its
  monitor's work area has no room above, and under the toolbar while the meter is
  closed or stepping aside; it has the meter's width (310 / 340 with total damage,
  220 in Capsule) and opacity, and follows the meter if the meter is dragged while
  the card is up (checked at each tick of its timer ring, below). A newer reminder
  replaces the card in the same window. It stays 30 s, paused while the pointer is
  over it, then fades over 150 ms; a click or × dismisses it. While the meter is
  locked it is click-through (`WS_EX_TRANSPARENT`) with no ×, following
  `LockChanged` live. Rarer events come first. Notifications are delivered whether
  the meter is open or not, and never twice.
- **Timer ring** (`reminders::TimerRing`, shared with the soul tracker card;
  approved 2026-10-10, tokens.md "Timer ring"). It replaced the 2 px time-left
  line: a 12 px ring left of the ×, in a 16 × 22 px slot 8 px before it, or in the
  ×'s 22 px slot while the card is locked. The track is a full circle and the arc
  the time left, both 2 px strokes with butt ends drawn as GPUI paths
  (`PathBuilder::stroke`, two half-arcs each, so a full turn has distinct ends) in
  a `canvas`. The arc starts at `(1 − left) × 360°` clockwise from 12 o'clock and
  ends at 12 o'clock, `muted`, or `text` while the pointer pauses the card. The
  card's timer (`reminders::Countdown`, its total passed in) ticks once per degree:
  30 s / 360 for the reminder, 10 s / 360 for the soul tracker card. The last tick
  is shortened so it lands when the time runs out (`Countdown::next_tick`), and a
  tick redraws the card only while its countdown runs.
- **Equal padding on the cards** (2026-10-10, both cards). The ink sits 14 px
  from the card's outer edge on every side, measured on the drawn images: card
  padding 9 px top, 8 px sides, 12 px bottom on the reminder card and 13 px on
  the soul tracker card (the header's glyphs sit 4 px inside their 22 px row,
  the last text line 2 px inside its line, a pet's bar 1 px inside its block).
  Heights at 100%: 81 / 120 / 159 px for 1 / 2 / 3 events, 100 / 157 / 214 px
  for 1 / 2 / 3 pets. Below a last pet with no souls toward its next level the
  gap reads 26 to 29 px: its bar's empty track is the card's own colour.
- **Bodies under the header's icon** (2026-10-10, approved `pet-card-layout`,
  "A, reminder follows"). The events and the pet blocks have the header's 4 px
  inset, so names, the level badge and the bar start 13 px from the card's
  outer edge, under the bell or the paw (they were indented 23 px, under the
  header text). In a pet block the name, the level badge and the bar are 8 px
  apart as seen: the 16 px Lv line is 4 px under the name's 18 px line (the
  13 px letters sit 4 px inside it), the 4 px bar 8 px under the Lv line; 57 px
  a pet (the Lv line was right under the name, the bar 4 px under it). An
  event's fact line stays right under its name.

## Pets

What the game says about the pet collection, read from its traffic and kept per
server, and the Pets window, the soul tracker card and Settings › Pets that show
it (approved `pets.html`, values in tokens.md, "Pets").

- **Messages** (server to client, written in wire byte order; decoded by
  `crates/core/src/capture/parser/pet_record.rs`, which packet-explorer's `pets`
  command shares):
  - `00 90`, the snapshot: `u32` header (1170 in every capture, meaning unknown),
    varint N × `{u32 id, u32 id again, u32 level}`, varint M × `{u32 id, u32
    souls since the last level-up}`, `u8` K (6) × `{u8 kind, u32 value}`, then
    stat tables that are not decoded and are skipped. A pet never obtained is in
    neither list (level 0, no souls); a levelled pet without a progress entry has
    0 souls; a pet at level 3 has 0. The two ids of a level entry must match,
    which is what keeps look-alike frames out.
  - `0D 90`, souls gained: `u8` n × `{u32 pet id, u32 amount}`. The amount is a
    delta, never a total. Loot gives 1 or 2, a Black Market purchase 5.
  - `11 90`, counter gains: `u8` n × `{u8 kind, u32 amount}` and a trailing `02`.
    It adds to the six counters of the snapshot's tail and carries no pet id; it
    is fired for souls of pets at their cap, so the counters are most likely
    per-family insight crystals (not proven). The core attaches the mobs whose
    loot the player picked up just before it (below), which is how the app
    names the pet.
  - `09 90`, a pet obtained for the first time (level 0 to 1): own entity
    varint, `u32` pet id, `u32` 1, `u8` kind, `u32` 5 or 6, `u32` value, `u32` 0;
    the kind and the two numbers are not identified (the value looks like a stat)
    and are not kept. It comes in two shapes, both checked against the
    recordings and the next snapshot. Looting the soul that completes the first
    5 sends `0D 90` for the pet and the unlock 50 ms later: the gain already made
    the pet level 1, so the unlock changes nothing and adds no souls (pets 1108
    and 1109, 2026-10-10 14:38:31 and 14:39:12). Buying a pet that is still
    level 0 sends the unlock alone, with no `0D 90` for that pet: the 5 bought
    pay the first threshold, so the pet becomes level 1 with the souls it had
    (1134: level 0 with 1 soul, unlock at 14:21:10, then level 1 with 1 soul in
    the next snapshot; six unlocks in one batch at 2026-10-09 23:48:39 beside
    a `0D 90` batch of other pets, all as predicted by the next snapshot). A pet
    already above level 1 is never lowered. No recording shows the unlock before
    the `0D 90` of the same pet.
  Pet ids are their own space (1001 to 1299), not mob codes. A frame that is cut
  short, has bytes after its body, a gain count of 0, an unlock whose count is not
  1 or whose last number is not 0, or a counter trailer other than `02` is
  rejected, never decoded in part.
- **Where a gain came from** (`crates/core/src/pet_sources.rs`, decoders in
  `capture/parser/loot_record.rs`, both shared with packet-explorer and the
  `replay` example). The game does not say in the pet message, so the core reads
  the messages before it, tracked per connection and always on:
  - `1E 56`, a loot object on the ground: `u8` count, then count records of 63
    bytes (`u32` dropper entity, six `f32` positions, `u32` object id, `u32`
    serial, 9 bytes not identified, `u32` item id, the rest not identified) and a
    closing byte. The dropper is the world entity id of the mob (the same id as
    in its `41 36` spawn, so `get_mob_code` answers it). 238906 is the soul item
    in every pickup that preceded a pet message.
  - `21 56`, the player's own pickup: 2 bytes not identified, `u32` object id,
    `u32` serial, `u8` 1. Object id and serial find the `1E 56` record, so a
    pickup names the entity and mob it came from. (`1F 56` also reports a loot
    object leaving the ground, but for any player's pickup, so it is not used.)
  - `95 56`, a purchase: it comes in the same capture tick as a `0D 90` of +5
    (the Black Market buys of 14:21:52 to 14:21:58 in session-1791634894978).
    The message does not say which store, so a gain after one is reported as
    `SoulSource::BlackMarket`. `84 56` also appears around it but repeats all the
    time without a gain, so it is not a marker.
  What the tracker remembers is bounded, from the recordings of 2026-10-07 to 10
  (5,476 `1E 56` records, 508 own pickups; measured by decoding every `1E 56`
  and `21 56` and matching them by object id and serial): the busiest minute
  announced 113 loot objects; the longest gap between an object appearing and
  the player's pickup was 0.65 s; every pickup found its announcement; and the
  item before every one of the 488 pet messages was 238906 (the soul item), yet
  loot of 635 different items arrives, so nothing is filtered by item (that would
  hard-code one id the game may not keep). Loot objects are kept 30 s (about 46
  times the longest gap, and about 60 objects at the busiest rate) and at most
  2048, the oldest going first, which only matters if the clock stops advancing;
  a soul drop can therefore only be evicted by 2048 other objects within 30 s.
  A pickup is kept only while a message can still claim it (250 ms, checked on
  every new pickup, not only when a pet message arrives) and at most 64, so a
  long session of ordinary looting holds a handful of entries.
  A message follows its cause by one capture tick: 39 to 72 ms over 488 pickups
  in the recordings, so a cause counts for 250 ms and is consumed by the message
  that claims it. A `0D 90` is `Loot` after a pickup, `BlackMarket` after a
  `95 56`, otherwise `Other` (a cube, a chest, a quest; the 7-pet batch of
  2026-10-09 23:48:39 and the lone +1 gains of pets 1091, 1058 and 1192 have no
  pickup or purchase before them). Over every recording of 2026-10-07 to 10:
  370 `0D 90` as loot, 12 as purchase, 14 as other; in each loot gain the
  pickup's mob drops the gained pet in the catalogue (one mob, 2100829 beside
  2100828 for pet 1119, is not listed). The core reports what it proves and
  `Other` when unsure.
- **A meter started after login.** The game names the character with `33 36`
  only at login and zone entry, so a session begun in game has no name for the
  binder: its soul gains waited and were dropped (the log of 2026-10-10 19:25
  shows "named from the remembered characters" and no `33 36`, and its 4 `0D 90`
  never reached the app; the session of 18:34 had a `33 36` at once). When the
  meter names the player from the remembered characters (`resolve_own_name`),
  the ticket and pet binders take that character only when exactly one remembered
  character has the actor's class (two of one class, or one name on two servers,
  leave the messages waiting: the meter's pick of the most recent would bind them
  to a possibly wrong name and server) (`CharacterBinder::assume_character`); the game's own name, when it arrives,
  replaces it, and one the game already gave is never replaced. Gains from
  before that moment, and any session with no remembered character of the
  class, still wait for a `33 36`.
- **A maxed pet's souls.** Its loot gives no `0D 90`, only a `11 90` of +1 right
  after the pickup (`1E 56` entity 37308, mob 2100453 → kind 1; entity 49184, mob
  2100454 → kind 4 in session-1791634499923). `PetContent::CounterGains` carries
  `looted_mobs`: the distinct mob codes of the pickups the message claims, empty
  when it follows no pickup (the +10 to +200 gains of kind 5 and 6 are not loot)
  or the entity was not spawned in the world. The amount is not a soul count: one
  mob can drop two or three loot objects for one counter of +1. A `0D 90` and a
  `11 90` of the same tick would share one pickup and the second would find none;
  no recording has that. The app maps each mob with `game_data::pet_of_mob` and
  reports it when that pet is at level 3.
- **The snapshot only arrives at login**, in the same millisecond as the own
  identity `33 36`, and is not sent again when the Pets window opens or the
  zone changes. A client that was not capturing at login has no snapshot of the
  session and keeps counting from what it stored; this is why the app persists
  the collection. The collection is shared by the characters of a server (two
  snapshots of two characters were byte-identical), so the app keys it by
  server.
- **The binding** (`crates/core/src/character_binding.rs`, shared with the
  tickets): the messages are tied to the local player's character like the
  tickets. A snapshot belongs to the name that arrives up to 15 s after it or up
  to 2 s before it; a gain belongs to the character named now; anything that
  cannot be tied yet waits for the next name and is dropped after 15 s or when a
  new snapshot shows another world was entered. The result is
  `MeterEvent::Pets(PetReport)` with the character, the capture time and one of
  `PetContent::Snapshot`, `SoulGains`, `Unlock` or `CounterGains`, all with the game's
  numbers untouched. It is not gated by `DpsMeter::set_fight_tracking`, so it runs
  with the meter closed. A recording that holds no own identity (one cut after
  login) emits no pet events, for the same reason.
- **What the app does** (`crates/app/src/pets/`): `bridge.rs` hands every batch's
  pet reports to the `Pets` entity (`service.rs`), meter open or not. `data.rs`
  is what `pets.json` stores: per server a `PetCollection`, the pins per server,
  and the server last heard from. A snapshot
  replaces that server's collection and records its capture time; a soul gain
  adds to the pet (an unknown pet starts at level 0); an unlock is applied as the
  rule below says. Each soul gain, and each maxed pet looted through a counter
  gain, becomes a `LatestGain` (`gain.rs`): pet, amount, level and souls after,
  whether the pet was already at the cap, the source (`Loot`, `BlackMarket`,
  `Other`, `MaxedLoot`) and a sequence number so a gain that leaves a capped pet
  unchanged still reads as new. `Pets::latest_gain` is the last; the last 16 are
  kept so a batch is not lost, and `Pets::card_gains_after(sequence, settings)`
  is what the card calls: the gains after the last sequence it handled that the
  four switches admit. An unlock does not set one: the tracker card follows soul
  gains.
- **Pins** are stored per server in `pets.json`, in a `pinned` map from server id
  to the pet ids in pin order, beside `servers`. (Putting them inside each
  server's entry next to the collection's fields would need `#[serde(flatten)]`,
  which cannot read the collection's `u32` map keys, or a version 2 with a
  migration.) A file from before them loads with none and the version stays 1.
  `Pets::pin`, `unpin`, `is_pinned` and `pinned` work on the server being
  played; a snapshot keeps them.
- **Settings** (`pets/settings.rs`, the `pets` section of `settings.json`):
  `soulTracker` (master, on), `onlyPinned` (off), `maxedPets` (pets already at
  level 3, on), `blackMarket` (on). `PetsSettings::admits(gain, pet_is_pinned)`
  is the one place the rules live: master off shows nothing; only pinned needs
  the pet pinned on the gain's server; `maxedPets` off drops a gain whose pet was
  already at the cap (the gain that takes a pet to the cap still shows); black
  market off drops purchases. A missing or mistyped value keeps its default.
- **The level rule lives in the core** (`crates/core/src/pets.rs`:
  `SOULS_PER_LEVEL = [5, 25, 75]`, `MAX_LEVEL`, `PetState::gain`,
  `PetCollection`), because packet-explorer's `pets` command applies the same
  messages to print each pet's level and souls after every gain and unlock, from
  the first snapshot of the recordings given (in time order; before one it says
  the start is unknown). The thresholds are per level, not cumulative: the game's
  arithmetic checks out against the snapshots of consecutive logins. Souls past a
  level carry into the next, level 3 is the cap and souls past it are not counted
  (the game turns them into crystals).
- **Storage.** `pets.json` beside `settings.json`: versioned JSON, atomic write,
  a bad or newer file renamed `pets.json.bad-<seconds>`, a write 400 ms after the
  last change, a flush when the app quits. The read, set-aside and write protocol
  is `stored_file.rs`, shared with `settings.json` and `todos.json`. With no data
  folder (replays, captures) the collection stays in memory.
- **The Pets window** (`pets/view.rs`, `view/rows.rs`, opened by the toolbar's
  paw-print tool after To-dos; `secondary::open_pets`): left of the meter while it
  is open (the to-do window keeps its right), under the toolbar while it is
  closed, opened anew each time like the other tools. It shows the collection of
  the server being played, read at login, so it works with the meter closed.
  `pets/list.rs` builds what is listed: every pet of `pets.json` in the game's
  order (pet id), with a pet of the collection the catalogue lacks at its place
  and named "Pet <id>" through i18n; the tab (All or a family); the search, which
  matches the English or the French name whatever the language
  (`game_data::pet_name_contains`, both names lower-cased once); the Show and
  Order menu (all / not owned yet = level 0 / not at Lv 3; game order / closest to
  the next level = fewest souls left, then the larger share, then id, Lv 3 last);
  the pins of the server on top in pin order when the tab, search and menu keep
  them; and the tab's summary (owned = level ≥ 1, at Lv 3, levels of levels
  possible), counted before the search and the menu. Souls toward the next level
  come from the core's `SOULS_PER_LEVEL`; a pet at the cap reads 75/75.
  The source line says "live" for a snapshot captured during this run
  (`Pets::read_this_run`, against the time the entity opened), else "as of your
  login" with a tooltip; before any snapshot the window shows how to get the pets
  read and its search, filter and tabs are dimmed and inert.
- **Only the lines in view are built** (`pets/view/rows.rs`, GPUI's `list`
  through `chrome::Rows`, see Secondary windows). GPUI redraws a window on every
  `WM_MOVE` (`Window::bounds_changed` calls `refresh`, which also bypasses cached
  views); building all ~200 rows took about 185 ms a frame in the debug build.
  The labels and rows have heights known before layout. The pin Tab reaches is
  the cursor's when its row is in view, else the first pin in view; Up, Down,
  Home and End move the focus to the pin and `Rows::reveal` scrolls its row in;
  the row whose pin has the keyboard is drawn wherever it is scrolled. About 4 to
  6 ms a frame in a debug build with optimized dependencies.
- **A drag does not redraw the window** (`overlay.rs`, for every overlay
  window). A drag move handled by the window costs about 20 µs; the full frame
  GPUI drew for the `WM_MOVE` it caused cost about 12 ms for the Pets window in
  the debug build, so the drag stuttered. While `WindowDrag` moves a window and
  its grab is held, the overlay subclass keeps `WM_MOVE` from GPUI
  (DirectComposition moves the drawn window as it is). The drag ends at once on
  the release, a move without the button, the mouse capture lost
  (`WM_CAPTURECHANGED`, `WM_CANCELMODE`: the release itself, Alt+Tab), the
  window hidden (`WM_SHOWWINDOW`), the app or the keyboard focus lost; a grab
  let go without any of those stops keeping the moves too. The end posts
  `FINISH_DRAG` to the window, which makes the move still owed and sends GPUI
  one `WM_MOVE` with the final position from the message loop, outside any GPUI
  handler. Pointer moves faster than the window can follow replace the pending
  target, so at most one `SetWindowPos` waits at a time; the target names its
  window and is dropped if that window is no longer the one dragged.
- **The scroll thumb** is the shared `chrome::scrollbar` (Secondary windows).
- **The menu choice** (Show and Order) is kept in `pets.json` as `list`
  (`ListChoice`, `Pets::list_choice` / `set_list_choice`), the same on every
  server, as the to-do window keeps its folded sections in `todos.json`.
- **Keyboard in the window.** The window never activates except while the search
  field types (the typing exception, below): a click in it or Enter on it takes
  the keyboard; Escape clears the text, a second Escape, Enter, a click elsewhere,
  another window activating or the pointer gone 400 ms gives it back; what was
  typed stays. Tab reaches close, the search, the filter button, the open tab and
  one pin: the tabs and the pins are roving groups (only the open tab's and one
  pin's focus handle is a tab stop; Left/Right/Home/End move the tab, Up/Down the
  pin, Space or Enter pins). A row's pin shows from the pointer's enter to its
  leave, with keyboard focus, or always on a pinned pet; after a pin by pointer
  moves a row away, the row then under the pointer keeps its pin hidden until the
  pointer moves (the to-do window's rule).
- **The soul tracker card** (`pets/card.rs`, `secondary::show_pet_card`):
  `pets::watch_gains` observes the `Pets` entity and, when a newer gain arrived,
  hands the card `card_gains_after(last handled, settings)`, so Settings › Pets
  decides what reaches it; it runs whether or not the meter is open. One card,
  up to three pets in arrival order: a pet on the card is updated in place, a
  newcomer goes last, a fourth replaces the oldest. "Lv N reached" when a gain
  takes the pet to a new level (`LatestGain::level_before`), "Unlocked" when that
  level is 1; a pet already at Lv 3 reads "Lv 3 · 75/75" with no chip. The card
  is the reminder card's twin: same width (the meter's, or fixed beside the
  capsule, `reminders::card_width`), non-activating window, lock (click-through
  and no × while the meter is locked), fade, and timer ring; the countdown is
  `reminders::Countdown`, shared with the reminder card, here 10 s, its ring
  refilled by every gain and paused while the pointer is over the card. It sits where the
  reminder card goes (6 px above the meter, or under the toolbar while the meter
  is closed or steps aside) and, while a reminder card is up, 6 px beyond it on
  the side the reminder took (`secondary::Card::PetCard`). A real gain replaces
  a test card.
- **The card's window outlives the card.** Opening a window costs 0.2 to 0.4 s
  (the log's reminder lines, "due" to "opened a card window", 190 to 384 ms on
  2026-10-10), so a gain right after a card went showed its card late, while a
  gain on a card still up showed at once. When its time is up (or on a click or
  the ×) the card fades and parks: its window hides (`Stacking::hide`) and stays
  registered, its ticker ends. The next gain shows it again at once
  (`Stacking::reveal`) with that gain's pets alone and a new 10 s; a gain during
  the fade cancels it. Alt+Shift+CapsLock puts an up card away at once (no
  fade, `PetCardView::put_away`) instead of hiding it with the rest, so its time
  never runs out unseen and showing everything again never brings back a card;
  a gain shows it at once whether everything is hidden or not, as a new card
  window did before.
- **Settings › Pets** (`settings/view/pets_page.rs`, between To-dos and General):
  the four switches (the three under the master dim and leave the tab order when
  it is off, keeping their values), "Show a test card" (`Pets::show_test_card`
  emits `ShowTestCard` with the first pet pinned on the server being played, or
  the first pet in game order, as it is now; nothing is added; the badge says
  "Shown above the meter" or "under the toolbar" for 4 s), and the collection
  row: "Pets on <server>", when it was read and how many of the catalogue's pets
  are owned, or how to have it read. The page redraws only when the server, the
  read time or the owned count changes (`PetsSeen`).
- **Level badge** (`pets/badge.rs`): each level's colour precomputed as its tint
  laid over the track's dark plate, shared by the rows and the card.

## The toolbar hub

`crates/app/src/toolbar/` (approved `toolbar.html`, `prompt.html`, values in
tokens.md, "Hub"). The toolbar is a non-activating overlay window that is never
click-through.

- At launch only it opens; the meter opens with it when Settings › Overlay › "Open
  the meter at launch" is on (replays and captures always open the meter). The
  first time the meter is opened the player is asked whether it should open by
  default (`WindowKind::Prompt`, stacked on the meter; Yes or Not now is stored as
  `windows.launchPromptAnswered`).
- Closing the toolbar quits the app. The meter's × hides the meter only; its
  window is kept, hidden, so every window placed against it still finds it and it
  opens where it was left.
- **Fights are tracked only while the meter is open**: startup calls
  `DpsMeter::set_fight_tracking` with whether the meter opens, and opening or
  closing the meter switches it. Capture, identity, the world, tickets and pets
  always run.
- Dropdowns and tooltips are drawn in the toolbar's own window, which grows for
  them; when the toolbar is low on its screen they open upward and the window
  grows upward, moving so the bar stays still.
- **Shortcuts and tray**: Alt+CapsLock locks the meter only while it is open;
  Alt+Shift+CapsLock hides the toolbar and every window of the secondary layer on
  screen and shows them again; the tray's Show toolbar and a second launch bring
  them back. The hub keeps the toolbar's handle and the other windows' kinds
  (`secondary::hide_all` / `show_again`), and each kind is looked up again in the
  registry when shown: a window closed meanwhile stays closed, and no stale handle
  (which Windows may have given to another program's window) is ever shown.
- The toolbar's counts and warning (`counts.rs`) come from the to-dos; the update
  icon is below.
- Deviations from the approved pages: the bell dropdown's next reminders are
  plain rows; folded, the warning's tooltip hangs from its left edge; the toolbar
  remembers its place and "Open the meter at launch", not which tools were open
  (so tools do not reopen after a restart).

## Fight report window

The approved end-of-fight report for boss fights (`report.html`, the summary
states of `meter.html`, the boss records of `history.html`, values in tokens.md
"Fight report"). The data is the core's `FightReport` (see "The fight report's
data").

- **Getting a report onto the UI thread.** The summary card's button emits
  `ReportRequest::OnMeter { target_id }`; History's report button emits
  `ReportRequest::Saved(record)`. `secondary/` subscribes to both and calls
  `open_report`: a saved record becomes `FightReport::from_record` and a live
  fight `DpsMeter::fight_report`, both run on GPUI's background executor
  (`report::build_off_thread`), their result handed back to the UI thread, so no
  window call ever comes from a meter thread. Without a meter (dev captures) the
  report is built from the snapshot the overlay shows, with nothing recorded.
  `show_report` opens `WindowKind::Report` or hands the report to the open window:
  the same fight leaves it as it is, another fight replaces it and returns to the
  table.
- **Window** (`src/report/`): `data.rs` turns the report into what is drawn (names
  masked, `ID:<n>` and "You" as elsewhere, players the game never named left out
  when "Hide unknown players" is on, as History does; shades per fight, most
  damage first); `model.rs` keeps the report frozen as asked for and rebuilds on a
  language, settings or hide-unknown change; `view.rs` draws the single sortable
  table (sort by a header, kept while the window is open, also across a player's
  details), the deaths, and a player's details in place of the table with a
  "‹ Report" back button. Keyboard: sort headers, rows and death rows are
  focusable buttons, Up/Down move between rows, Enter opens, Left/Right switch
  tabs.
- **History** shows a crown before boss names and a report button (with its "Fight
  report" tooltip) before Delete on boss records only. The summary card has "Open
  the fight report" under the skills, on the final and the provisional card, and
  none while locked.
- Deviations: a fight saved before the report existed has no buff uptime (its
  Buffs tab shows "--" and an empty coverage bar); the Healing and Damage taken
  tables share one scroll position; scrolling lists keep their 3 px of
  focus-ring room above and below (GPUI collapses a list with negative vertical
  margins), so tables sit 3 px lower than drawn.

## Update UI

- **Toolbar** (`toolbar/update.rs`, `view.rs`): the update icon shows for
  available (also after a failed download), downloading, paused, checking the
  download and ready; never for up to date, checking, offline, limited, failed
  check or off. It sits after the rule and before fold (after the warning when
  folded). Click: download / Settings › About / restart at once. Tooltips carry the
  fight warning on Ready (`Hub::in_fight`, the core's rule). It redraws once per
  percent.
- **Settings › About › Updates** (`settings/view/updates_group.rs`): a pure
  `Status::of(snapshot, restart_failure, now, running)` and its row; one primary
  button at most; Check now stays disabled until `check_again_at` (GitHub's reset
  when limited), with a timer that redraws it; the automatic download switch;
  "Release notes" through `files::open_web_page`, which hands Windows `https://`
  addresses only. Errors are field messages: red for failed checks and restarts
  that could not start, amber for GitHub unreachable or limiting. A build without
  `UPDATE_REPO` shows one muted line instead of the status and the switch.
- `ReleaseInfo.published` (GitHub's `published_at`) feeds "Released 21 Oct"; the
  worker gets `Shutdown` at quit, which deletes a partial download. The offline,
  limited and failed-check rows say "Last checked at".
- **Bold in the changelog:** GPUI blends a highlight's colour but keeps the
  paragraph's alpha, so bold runs use the text colour made opaque
  (`changelog::bold_in_text_colour`).

## Checking without showing anything

A build with the off-by-default Cargo feature `probes` adds the development
checks; the default binary contains none of that code, and a probes build is
`asInvoker`:

- `--overlay-probe` and `--font-probe` check overlay window behaviour and fonts.
  They show a window on a monitor and send no mouse input to anything.
- `--capture <dir> (--fixtures | --replay <file>) [--language english|french]`
  asks GPUI for a hidden window (GPUI moves bounds outside every monitor back
  onto the primary one), moves it past the left edge of the virtual screen, then
  shows it without activation. Every capture first checks with `MonitorFromRect`
  that no monitor holds any of it, and hides it and stops otherwise.
  `PrintWindow(PW_RENDERFULLCONTENT)` returns the composed card over black.
  `--fixtures` draws every state of every window from synthetic snapshots
  (invented names), runs window-style, hit-test, drag, typing, single-instance
  and entry-point checks with messages posted to our own windows only, and
  reports frames built and CPU used while idle; `--replay <file>` captures the
  paced replay as it is, under the pointer, locked and as a capsule, with names
  masked, and opens the detail and history windows on the recording's local
  player. Captures go outside the repository. No global shortcut is registered in
  this mode.
- Compare captures with the approved page's own render of the same state (heights
  agree within 1–4 px). Dragging, `Alt+CapsLock`, Copy into chat, the card edge
  on a bright scene, a monitor at another DPI, live capture, and the release exe
  on a clean machine stay manual checks in the game.

## Approved designs

The UI follows the pages in `docs/gpui-mockups/approved/` (one HTML page per
screen with every state; open `index.html` in a browser, no build step) and
`approved/tokens.md` (colours, radii, type, spacing, motion, interaction patterns
and how each maps to GPUI). Only validated choices go into `approved/`; the other
folders under `docs/gpui-mockups/` are exploration history. The mockups were
reviewed in a browser with substitute fonts, so where the app deviates it is
listed here:

- Barlow Semi Condensed instead of Bahnschrift (see "Fonts").
- No drop shadow on overlay cards, menus or popovers.
- Only working controls in the meter's tool row; no tooltips on its icon buttons
  (they would need room outside the window, which ends at the card).
- The capsule opens by resizing and fading, not growing; the percent column is
  weight 400; a long name ellipses while combat power stays visible.
- The summary's best line has two tones (new or first, behind).
- The emptied gauge segment does not fade to the track over 200 ms; the drain
  itself is tweened.
- Settings: Barlow for titles and figures; credits without a component kit; the
  licence row drops "Any shared build ships with its source" (a `--no-source` zip
  ships a written offer, "Source and notice" opens README.txt); "Applies at next
  start" under Backend priority.
- The report's sort arrow is stroke 2 (the icon file's), not 2.5.

## Why capture could not start

`DpsMeter::start_dps_meter` returns `CaptureStartError::NoBackend` (which driver
the priority tried first, and Npcap's and WinDivert's own errors as they
reported them) instead of an English sentence, so the app writes the waiting card
and the Settings capture tile in the language in force; its `Display` is the
sentence for the log. `start_for_replay` opens no driver and does not return a
`Result`.

## Diagnostics

The shipped exe has no console (`windows_subsystem = "windows"` without the
`probes` feature), so failures go to `belisk.log`, the file the core also writes
to:

- `crates/app/src/app_log.rs` holds the one `AppLogger` of the run. `run`
  installs it before anything else (`%LOCALAPPDATA%/Belisk/logs/belisk.log` for a
  live run, the temp folder's `belisk/gpui-app.log` for replays and captures,
  which never touch the player's log) and hands the same logger to the meter, so
  the app's lines and the core's interleave in one file. `app_log::error` and
  `app_log::warn` are free functions the windows call; a `probes` build also
  echoes them to its console. Before `install`, and in unit tests, a line is
  dropped.
- Levels: a file that could not be read, set aside or saved (history, personal
  bests, settings, to-dos), a window that could not be opened, the meter or the
  single-instance listener failing to start: `ERROR`. A window that could not be
  placed, fitted or locked, a shortcut, the tray or the reminder sound that is
  unavailable, a click that found no fight report: `WARN`. The core's capture
  thread failing to open a device: `ERROR`.
- Nothing on the capture, parser or snapshot threads writes per packet or per
  tick. A failed history or bests save repeats every 5 s at most.
- `eprintln!` stays only where a console exists: tests, `crates/xtask`,
  `crates/packet-explorer`, `crates/core/examples/replay.rs` and the `probes`
  modules (`capture*.rs`, `font_probe.rs`, `probe.rs`, `fixtures.rs`).

## Updates

The updater is `crates/app/src/update/`; how it works and its trust model are in
[updates.md](./updates.md), and releasing is in [releasing.md](./releasing.md).

## Data folder

`%APPDATA%/Belisk` holds `settings.json`, the to-dos, `pets.json`, the fight history, the
personal bests, the recordings and `mobs.json`; `%LOCALAPPDATA%/Belisk/logs`
holds `belisk.log`. Builds before the rename used `%APPDATA%/com.helveticxa.aether`
and `%LOCALAPPDATA%/com.helveticxa.aether/logs/aether.log`.
`crates/app/src/data_folder.rs` (`settle`) moves them once, at the start of a
live run, before anything opens those files:

- the new folder is missing and the old one exists: the old folder is renamed
  into place (same volume, nothing is copied);
- the rename fails: this run keeps the old folder, says why in the log, and the
  next launch tries again;
- both exist: nothing is merged. The new folder wins, the old one is left
  untouched and the log names it. A new folder that holds only empty folders
  counts as missing;
- only the `logs` folder moves out of the old local folder.

Until `settle` has run, `data_dir()` and `log_dir()` answer `None`, so replays,
captures and the `--apply-update` helper can never create the player's folders.
Tests use temp folders, never the real ones.

## Version and changelog

The version is 1.0.0 for `meter-core` and the app. `CHANGELOG.md` and
`CHANGELOG.fr.md` hold this app's own changelog, read by the app
(`crates/app/src/changelog/`). The upstream project's release history is kept
unedited in [aether-changelog.md](./aether-changelog.md).

The app embeds both and parses them with its own parser for a small Markdown
subset; `cargo test` fails on syntax outside it and when the two languages differ
in versions, dates or bullet counts. `settings.json` holds `lastSeenVersion`: a
first run is stamped with the running version, and an older file without it
counts as older than 1.0.0.

What's new (`crates/app/src/whats_new/`, `WindowKind::WhatsNew`) opens once at
launch after a version jump, under the toolbar, and shows only the versions newer
than the last one the player ran. It never opens during a fight
(`DpsMeter::in_fight`, polled every 2 s while one is on), on a fresh install, for
a seen version, or in replays and captures; × and "Full changelog" mark the
version seen. The changelog is drawn natively in GPUI. Settings › About has an
Updates group with "Open changelog", and About › Changelog
(`settings/view/changelog_page.rs`) shows the newest version expanded and older
ones as a header, loading more through "Show older"; it draws the shown rows whole
(heights differ) rather than through GPUI's list.

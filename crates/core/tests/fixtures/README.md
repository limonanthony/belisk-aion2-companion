# Replay fixture

`fight-anonymised.belisk` is about a minute of a real fight, anonymised, that
`tests/replay_fixture.rs` replays through the whole core (recording reader, TCP
reassembly, dispatcher, framing, parsers, storage, snapshot) and checks the totals
of. It exists so that a parser change cannot keep the unit tests green while it
changes what a player reads off the meter.

## What it holds

One synthetic connection (`203.0.113.10:13328` to `192.0.2.10:50000`, addresses
from the documentation ranges) carrying, as plain frames and with the capture
times moved to start at 1,700,000,000 s, only what fight tracking reads: the
heartbeat `00 36`, `33 36` / `45 36` / `56 36` (who the players are), `41 36`
(spawns), `42 36` / `04 8D` (deaths), `04 38` / `05 38` (hits and damage ticks),
`2A 38` / `2B 38` / `2C 38` (buffs) and `00 8D` (health). Everything else the
recording held (chat, party, inventory, the client's own encrypted side) is left out.

## No player name is in it

The names are replaced, not hidden:

- The tool replays the original frames through the shipped `StreamAssembler`
  and takes every name the core stored for an actor and every killer name.
- In the frames that carry text (`33 36`, `45 36`, `41 36`, `42 36`, `04 8D`) each
  of those names is replaced, wherever its bytes occur, by `P` and a base-36
  number padded to the same byte length (`P0000`, `P0001`, ...), and every other
  text run (four or more printable characters, such as a guild name) is filled
  with `x`, character by character, so lengths and framing hold.
- Before it writes anything the tool refuses unless none of the original names
  occurs anywhere in any kept frame, and unless the core reads the anonymised
  frames the same way: the same actors with the same classes, servers and combat
  power, and the same damage for each. It prints the text left in the identity
  frames for a last look; those are placeholders and filler.

## How it was made

From the repository root, with a recording of a fight that starts with your own
character being named (a `.belisk` from the Settings recordings folder):

    cargo run -p packet-explorer -- fixture <recording> --out crates/core/tests/fixtures/fight-anonymised.belisk

The fixture in the repository came from a 70-second session recording of
2026-10-10; the tool printed 6133 frames kept of 15644, 41 names replaced and
2,068,011 damage in total, which is `TOTAL_DAMAGE` in the test. Regenerating from
another recording changes the expected values in `tests/replay_fixture.rs`
(`FIXTURE_FRAMES_REPLAYED_AS_PACKETS`, `TOTAL_DAMAGE`, `PLAYERS_SEEN`,
`OWN_PLACEHOLDER_NAME`); the values were not taken from an independent decoder:
the total is what the core makes of the real frames.

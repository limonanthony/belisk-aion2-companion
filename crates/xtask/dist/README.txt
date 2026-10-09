Belisk {version}
================

Belisk is a combat meter and companion for AION 2 (global servers). A small
overlay shows who is dealing damage, live, and summarises boss fights. It
works by passively reading network packets. It does not read or write the
game's memory, inject anything, or send input to the game.

This is a pre-release. Expect rough edges.


REQUIREMENTS
------------
- Windows 10 or 11, 64-bit.
- Run it as Administrator. Windows asks for permission when you start it.
- Npcap is recommended: install it once from https://npcap.com. The meter
  uses it to read packets.
- Without Npcap the meter falls back to the bundled WinDivert driver. That
  driver is only loaded when Npcap cannot be used, and some anti-cheat
  software dislikes it.
- Nothing else to install. No Visual C++ runtime is needed.


STARTING
--------
1. Unzip the whole folder somewhere (not inside the zip), for example
   C:\Tools\belisk.
2. Keep WinDivert.dll and WinDivert64.sys next to the exe. They are only used
   when Npcap cannot capture; with Npcap installed the meter runs without them.
3. Double-click belisk.exe and accept the Administrator prompt.
4. Start Belisk BEFORE you log in to your character, or go back to the
   character select screen and log in again. That is how it learns which
   character is yours. Until it knows, it shows a hint.


CONTROLS
--------
- Belisk opens as a small floating toolbar. Its icons open the DPS meter, the
  PvP meter, the to-dos, the fight history, the event reminders and Settings;
  beside them are your Odyle Energy and Shugo keys counts. Drag the toolbar by
  its grip to move it; the last icon quits Belisk.
- Alt+Shift+CapsLock shows or hides the toolbar and every open tool.
- Drag the meter by its header to move it.
- Alt+CapsLock locks or unlocks the meter. While it is locked, mouse clicks
  go through it to the game.
- Move the pointer over the meter's header to see the buttons: reset, capsule,
  lock, copy, the "..." menu and close.
- The meter's "..." menu holds the boss corrections ("Treat as a boss" / "Not a
  boss" / automatic).
- The Session / Target switch in the footer chooses between everything you
  fought since the last reset and the single target (a boss) you are on.
  Normally it switches by itself. A pin marks a manual choice; click the
  active side again to go back to automatic.


SETTINGS
--------
The toolbar's gear opens Settings: how the overlay looks, what the meter counts,
capture and recordings, personal bests, event reminders. Changes apply at
once and are kept in settings.json (see below). Two switches on the Capture
page are off by default: "Collect mob data" records the monsters you fight
into mobs.json, and "Decision log" writes the meter's decisions to the log
file. The Reminders page shows a small card above the meter shortly before
the game's timed events (Shugo Festival, Dimensional Invasion and Rift
portals by default, on this PC's clock), with a sound; "Show a test
reminder" tries it. Settings works with the keyboard too: Tab moves between
controls, Space or Enter presses, the arrow keys move sliders and choices,
Escape closes a list.


WHERE DATA IS STORED
--------------------
  %APPDATA%\Belisk                   settings, to-dos, fight history, recordings
  %LOCALAPPDATA%\Belisk\logs         log files (belisk.log)

Paste these paths into the Explorer address bar to open them.

Builds from before Belisk kept these in %APPDATA%\com.helveticxa.aether and
%LOCALAPPDATA%\com.helveticxa.aether\logs. The first time Belisk starts it
moves them to the folders above by itself, so nothing needs copying. If the
move fails (a file is open somewhere), Belisk keeps using the old folder for
that run and tries again at the next start. If both folders already exist it
uses the new one and leaves the old one untouched; copy anything you still
want from it by hand. Recordings made by earlier builds (.aetherpc) still
open; new ones are .belisk files.


UPDATES
-------
The meter asks GitHub once an hour (and at start) whether a newer release
exists, and that is the only thing it contacts: nothing about you or your game
is sent. A new version is downloaded in the background at 1 MB/s at most, and
not while you fight; Settings > About > Updates can turn that off, and then
you click Update yourself. The download is checked (SHA-256 and a signature)
before anything is unpacked, and nothing is installed until you click
"Restart to update". The files go into an update-staging folder beside the
exe while they wait.


WINDOWS WARNINGS
----------------
The exe is not code-signed, so Windows SmartScreen may say it is from an
unknown publisher. Choose "More info", then "Run anyway", if you trust the
person who sent you this.


UNINSTALL
---------
Delete the folder. To remove your settings and history too, delete the two
data folders above. If the WinDivert driver was ever used, it removes its own
service when the meter closes; a restart clears it for certain.


LICENCE AND SOURCE
------------------
Belisk is free software under the GNU General Public License,
version 3 (LICENSE.txt). {source_statement}

It builds on Aether (Helveticxa), NOIA2 (ZDYoung0519) and data from
Kuroukihime/AIon2-Dps-Meter. Licences of everything bundled are listed in
THIRD-PARTY-NOTICES.txt.

This is a fan-made tool and is not affiliated with the game's publisher.
Whether a third-party tool is allowed is the publisher's call; use it at your
own risk.

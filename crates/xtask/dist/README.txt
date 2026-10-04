AION 2 DPS Meter {version}
=========================

A real-time DPS meter for AION 2 (global servers). A small overlay shows who
is dealing damage, live, and summarises boss fights. It works by passively
reading network packets. It does not read or write the game's memory, inject
anything, or send input to the game.

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
   C:\Tools\aion-2-dps-meter.
2. Keep WinDivert.dll and WinDivert64.sys next to the exe. The meter does
   not start without WinDivert.dll.
3. Double-click aion-2-dps-meter.exe and accept the Administrator prompt.
4. Start the meter BEFORE you log in to your character, or go back to the
   character select screen and log in again. That is how it learns which
   character is yours. Until it knows, it shows a hint.


CONTROLS
--------
- Drag the overlay by its header to move it.
- Alt+CapsLock locks or unlocks the overlay. While it is locked, mouse clicks
  go through it to the game.
- Move the pointer over the header to see the buttons: reset, capsule, lock,
  copy, the "..." menu and close.
- The "..." menu holds fight history, the PvP meter and the boss corrections
  ("Treat as a boss" / "Not a boss").
- The Session / Target switch in the footer chooses between everything you
  fought since the last reset and the single target (a boss) you are on.
  Normally it switches by itself. A pin marks a manual choice; click the
  active side again to go back to automatic.


LAUNCH OPTIONS
--------------
Start the exe from a terminal, or from a shortcut with the option added to
its Target, to use these. Both are off by default.

  --debug-log     write the meter's decisions to the log file
  --collect-mobs  record the monsters you fight into mobs.json


WHERE DATA IS STORED
--------------------
  %APPDATA%\com.helveticxa.aether                   settings and fight history
  %LOCALAPPDATA%\com.helveticxa.aether\logs         log files

Paste these paths into the Explorer address bar to open them.


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
AION 2 DPS Meter is free software under the GNU General Public License,
version 3 (LICENSE.txt). The complete source code of this build is in
source\aion-2-dps-meter-{version}-source.zip.

It builds on Aether (Helveticxa), NOIA2 (ZDYoung0519) and data from
Kuroukihime/AIon2-Dps-Meter. Licences of everything bundled are listed in
THIRD-PARTY-NOTICES.txt.

This is a fan-made tool and is not affiliated with the game's publisher.
Whether a third-party tool is allowed is the publisher's call; use it at your
own risk.

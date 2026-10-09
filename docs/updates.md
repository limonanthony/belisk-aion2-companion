# Updates from GitHub Releases

Code: `crates/app/src/update/`. Tickets: TASK-28, TASK-28.1, TASK-28.2. Design:
`docs/gpui-mockups/approved/` (toolbar icon, Settings › About › Updates).

## What it does

- **Source.** `GET https://api.github.com/repos/<owner>/<repo>/releases/latest`
  with a `User-Agent`. `<owner>/<repo>` is the build-time environment variable
  `UPDATE_REPO` (CI sets it from `github.repository`). Without it the build has
  updates **disabled**: `UpdateState::Disabled`, no thread, no network. Replays
  and captures are never live, so they are disabled too. A build whose
  `crates/app/update-key.pub` still says DEVELOPMENT is disabled the same way
  (logged), since it could verify nothing.
- **Where it may go.** Only `https` on the standard port, to `api.github.com`,
  `github.com`, `objects.githubusercontent.com` and
  `release-assets.githubusercontent.com`. The API address, every asset address
  and every redirect hop is checked (`http.rs`, `Policy::GITHUB`); anything else
  is refused and shows as a failed check or download with that reason. "Release
  notes" opens `https://github.com/<UPDATE_REPO>/releases/tag/v<version>`, built
  by the app, never the address GitHub's answer carries.
- **Checks.** At launch, then once an hour while the app runs (GitHub allows 60
  unauthenticated requests an hour per address); the worker looks at the clock
  every 30 s, so a PC that slept checks right after it wakes. The hourly check
  goes on while an update is available or staged: a newer release replaces a
  staged one. "Check now" does nothing within a minute of a check. On 403/429
  nothing is asked until `retry-after` / `x-ratelimit-reset` (between 1 minute
  and 24 hours; an hour when GitHub gives no time). No switch turns checking off.
- **Download.** Always capped at 1 MB/s, and paused while the player is in a
  fight (`DpsMeter::in_fight`; fights are only tracked while the meter window is
  open, so with it closed there is no pause). The archive may not be larger than
  64 MiB: a release that says more fails with that reason before anything is
  requested, and a server that sends more bytes than the release announced fails
  the download. A connection lost mid-way resumes with a `Range` request, at
  most 3 times. Settings › About › "Download updates automatically" is on by
  default (`updates.downloadAutomatically` in `settings.json`; a file without
  the key reads as on): on, Available turns into Downloading by itself; off, the
  player clicks Update. Turning it off, or Cancel, during a download deletes the
  partial file; Cancel also makes the automatic download leave that version
  alone. A failed download is back to Available with its reason; automatic
  download retries only at the next hourly check.
- **Verification.** SHA-256 against the release's `.sha256` asset, and an
  Ed25519 signature against the public key built into the exe
  (`crates/app/update-key.pub`). The signature is over
  `"belisk update <version>\n"` followed by the archive's bytes (the
  version binds the signature to its release, so an old genuine archive cannot
  be served as a newer one). A mismatch deletes the staging folder and installs
  nothing. The checks run on bytes read once into memory, and what is installed
  comes from those same bytes.
- **Staging.** `<exe dir>/update-staging/ready/` holds the signed archive as
  published (`update.zip`), its published hash and signature text, and a `READY`
  file with the version. `download/` beside it is the download in progress.
  Nothing there is trusted: `READY` only says which version the archive claims,
  and every use re-reads the archive and checks its hash and signature again
  (when the app starts, before it shows "ready"; when Restart is clicked; in the
  helper, before the swap). A planted or changed file can make an update fail;
  it cannot make one install. The archive may hold only `belisk.exe`,
  `WinDivert.dll` and `WinDivert64.sys`, by exactly those names, and is refused
  otherwise.
- **Restart** (only on the player's click, also during a fight): see below.
- **Downgrades.** An older build that meets a newer `settings.json` sets it
  aside and starts from defaults, so settings are lost until the file is copied
  back.

## Trust model

- The signature is the only thing that makes bytes installable. Whoever holds
  the private key can ship code that runs elevated on every player's PC. The
  key is never in the repository: it lives in the repository secret
  `BELISK_UPDATE_SIGNING_KEY`, and the release workflow writes it to a temporary
  file for the one `cargo xtask sign` step and deletes it (docs/releasing.md).
  **Residual risk of signing in CI:** anyone who can write to the repository's
  secrets or workflows (the owner's account, or a collaborator or token with
  admin or workflow write access to `main`) can sign a build, or have a
  workflow they add read the key, so protecting `main` and the account (2FA)
  protects every installed copy. A release is only published from `main` and
  signing refuses a key that does not match `crates/app/update-key.pub`.
- The helper that swaps the files is **the installed exe**, started again as
  `belisk.exe --apply-update <parent pid>`. The staged update is never
  executed. The command line carries no path or name: the install folder is the
  helper's own exe's folder, the staging folder is `<install>/update-staging`,
  the files it may write are the three above (the exe under the running exe's
  name), and the pid must be the helper's real parent (a missing, zero or
  foreign pid is refused, never treated as "already ended"). Running the helper
  by hand cannot copy anything the signature does not cover.
- No DACL on `update-staging/` is needed with this design: its contents are
  re-verified before every use, so write access to it buys an attacker a failed
  update, nothing more.
- **Residual risk.** The app is a portable zip, and its folder is writable by
  the user who unzipped it. Anyone who can write there as that user can replace
  the exe itself (before or after an update, with or without this feature), and
  the elevated helper's `.new` files are written into that same folder. The
  updater does not widen that. Installing in a folder only Administrators can
  write (Program Files, say) closes it; the app works from there, since it runs
  elevated.
- **The running exe is held.** Windows lets a running exe be renamed, so a
  process of the same user could move the image aside, put another exe at its
  path, and have Restart start that one elevated with no prompt. At start the
  app opens its own exe file and keeps it open for its whole life with sharing
  limited to reading (`apply::ImageLock`): while it is held, renaming, deleting
  or writing that file, or renaming its folder, fails for everyone else, and
  Restart refuses unless the exe it would start is the one that was held. This
  was chosen over hashing at start and at Restart (which leaves a window between
  the check and the spawn and compares with nothing trusted), and over copying
  the image to an Administrators-only folder (which needs a place to put it and
  an ACL to get right). The hold is the only mechanism: if it cannot be taken,
  updates stay off. It starts when `update::install` runs, a moment after the
  process starts, so a swap made before that is not covered, and nor is the
  case where replacement happens while the app is not running. The helper does
  not hold the file: it starts after the app has ended, and renames its own
  running exe aside during the swap.
- The helper is the version being replaced, so a bug in its swap code is fixed
  only for the update after.

## Release assets (the lean update archive)

Per release, next to the full zip (`cargo xtask dist` writes the first two):

| Asset | What |
|---|---|
| `belisk-<version>-windows-x64-update.zip` | flat: `belisk.exe`, `WinDivert.dll`, `WinDivert64.sys` (no README, licence, notices or source) |
| `belisk-<version>-windows-x64-update.zip.sha256` | `<hex>  <file name>` |
| `belisk-<version>-windows-x64-update.zip.sig` | 128 hex digits, from `cargo xtask sign` |

The release's tag is `v<version>`, equal to the Cargo version. A release
without the archive is not offered; one without the `.sha256` or `.sig` shows
as available but its download fails with a "missing" reason.

## Signing keys

`crates/app/update-key.pub` holds a **DEVELOPMENT placeholder** until the owner
replaces it: a key whose private half was thrown away when it was made. While
it is there the app never installs an update (updates stay disabled), and
`cargo xtask dist` **fails**. `--allow-development-key` makes dist build anyway,
for local testing only: CI never passes it, and nothing players get may use it.

One-time setup by the owner (full steps in docs/releasing.md):

1. `cargo xtask keygen --out C:\Users\<you>\keys\belisk-update.key` writes the
   private key there and the public key to `belisk-update.key.pub`. The path
   must be outside the repository (the command refuses one inside; `*.key` is
   also in `.gitignore`). Keep a backup: players on builds with the old public
   key can only update to a release signed by that key's private half.
2. Copy the public key file over `crates/app/update-key.pub` (a `#` comment
   line and one line of 64 hex digits, no DEVELOPMENT in the text) and commit
   it.
3. Store the private key as the repository secret `BELISK_UPDATE_SIGNING_KEY`
   (`gh secret set`).

For each release, GitHub Actions runs `cargo xtask dist`, then
`cargo xtask sign --key <temporary file> target/dist/belisk-<version>-windows-x64-update.zip`,
which writes the `.sig`, and publishes the full zip, the archive, its `.sha256`
and its `.sig`. `sign` verifies its own signature against
`crates/app/update-key.pub` first and writes nothing on a mismatch, so a wrong
secret stops the release instead of publishing an update no app accepts. To
sign by hand (for testing a key), run the same two commands locally with the
private key outside the repository.

## Restart sequence

1. The player clicks "Restart to update" (`Updates::restart`, state `Ready`).
2. The app reads the staged archive and checks its hash and signature once more
   (a tenth of a second on the click). If that fails, nothing has changed: the
   app keeps running and `Updates::restart_failure` says why.
3. The app starts **a copy of its own exe** as `--apply-update <its pid>`
   (a child of an elevated process is elevated: no UAC prompt;
   `DETACHED_PROCESS`, and the exe has no console), then calls `cx.quit()`: the
   quit hooks stop capture (WinDivert releases its driver), save settings,
   to-dos, history, bests and recordings, and the process is ended (`quit.rs`).
4. The helper (`update::apply_if_requested`, before any window, setting or
   capture exists) takes its paths from where it runs, confirms the pid is its
   parent and waits for that process to end (up to 60 s; the single-instance
   lock goes with it). If it does not end, nothing is touched.
5. It reads the staged archive once, checks hash, signature and that the version
   is newer than its own, and unpacks the three allowed files from those same
   bytes. It writes each as `<name>.new`, renames the file it replaces to
   `<name>.old`, and the new one into place (a file identical to the installed
   one is left alone). Any failure moves everything back: the old exe stays.
6. It deletes the staging folder and starts the installed exe: the new version
   after a swap, the old one after a refusal or a rolled-back swap, whose reason
   is in `%TEMP%\belisk\gpui-app.log` (the helper runs before the data folders
   are settled, so it never writes the player's log folder). A refused or failed
   update is not kept; it is downloaded again at the next check.
7. The next start deletes the `*.old` and any unfinished `*.new` files.

What's new opens after the restart through the changelog's last-seen version.

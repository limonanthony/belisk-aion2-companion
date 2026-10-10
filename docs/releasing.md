# Releasing Belisk

Workflows: `.github/workflows/ci.yml` and `.github/workflows/release.yml`. The
pinned Rust is `rust-toolchain.toml`. Update trust model and asset formats:
[updates.md](./updates.md).

## Branches

- Work happens on `dev`. A push to `dev`, or a pull request to `dev` or `main`,
  runs **CI** (Windows runner): `cargo test --workspace --locked`, then
  `cargo build -p belisk --features probes --locked`. A new push to the same
  branch cancels the run in progress.
- A push to `main` runs **Release**. A release is a push (merge) to `main`
  whose `crates/app/Cargo.toml` version has no tag `v<version>` yet.

## What Release does on a push to main

The decisions are `cargo xtask release-plan` (steps 1-2) and `cargo xtask
release-notes <version>` (step 3), so both run locally too.

1. Reads the version from `crates/app/Cargo.toml` (via `cargo metadata`).
2. If the tag `v<version>` exists on `origin` (`git ls-remote`), it only builds
   and tests: **nothing is published**, so a merge without a version bump
   releases nothing. If `origin` cannot be reached the step fails: it never
   guesses.
3. Otherwise it fails, before building, unless the first `## ` section of both
   `CHANGELOG.md` and `CHANGELOG.fr.md` is `## [<version>] - <date>`, is not
   empty, and no `## [Unreleased]` section exists.
4. `cargo test --workspace --locked`.
5. `cargo xtask dist` with `UPDATE_REPO` set to the repository (so the released
   app has its updater on). `dist` refuses while `crates/app/update-key.pub` is
   the DEVELOPMENT placeholder or the tree is dirty; CI never passes
   `--allow-development-key` or `--allow-dirty`.
6. `cargo xtask sign` signs the `-update.zip` with the key from the repository
   secret `BELISK_UPDATE_SIGNING_KEY`. The key is written to a temporary file
   for that one step and deleted after it (also when the step fails). `sign`
   also fails, writing nothing, when the key is not the private half of
   `crates/app/update-key.pub`: a wrong secret cannot produce a release no
   player's app would accept.
7. A second job, the only one with `contents: write`, creates a **draft**
   release `v<version>` titled `Belisk <version>`, uploads the assets, then
   publishes it (the tag `v<version>` is created on that commit at that
   moment). A version with a suffix such as `1.1.0-beta.1` is published as a
   prerelease, which `/releases/latest` and so the updater skip.

Release notes: `## English` with that version's section of `CHANGELOG.md`, then
`## Français` with the section of `CHANGELOG.fr.md`.

Assets: `belisk-<version>-windows-x64.zip` (full, with `source/`),
`belisk-<version>-windows-x64-update.zip`, its `.sha256` and its `.sig`.

Actions used: `actions/checkout@v4`, `actions/upload-artifact@v4` and
`actions/download-artifact@v4` (GitHub's own), and
`Swatinem/rust-cache@v2` in CI only (the build cache; it never runs in the job
that holds the signing secret). They are pinned to a major version, not a
commit SHA; pin to SHAs if you want protection against a moved tag. The
release job builds without a cache so the shipped exe comes from a clean build.

## One-time setup (the owner)

1. Make the signing key pair, outside the repository (PowerShell shown):

   ```
   cargo xtask keygen --out C:\Users\<you>\keys\belisk-update.key
   ```

   This writes the private key there and the public key to
   `belisk-update.key.pub`. Back the private key up somewhere safe (a password
   manager): see "Lost or leaked key" below.
2. Replace the placeholder: copy `belisk-update.key.pub` over
   `crates/app/update-key.pub` and commit it on `dev`. The file must hold a `#`
   comment line and one line of 64 hex digits, with no `DEVELOPMENT` in it.
3. Store the private key as the repository secret
   `BELISK_UPDATE_SIGNING_KEY`:

   ```
   Get-Content -Raw C:\Users\<you>\keys\belisk-update.key | gh secret set BELISK_UPDATE_SIGNING_KEY --repo limonanthony/belisk-aion2-companion
   ```

   (In a POSIX shell: `gh secret set BELISK_UPDATE_SIGNING_KEY --repo limonanthony/belisk-aion2-companion < belisk-update.key`.)
4. Make `main` the default branch (Settings › General › Default branch, or
   `gh repo edit limonanthony/belisk-aion2-companion --default-branch main`
   once `main` has been pushed). The updater reads `releases/latest`, which does
   not depend on the default branch, but the Release workflow only runs for
   `main`.
5. Optional: protect `main` (Settings › Branches › Add rule): require a pull
   request and the status check **Test (Windows)**. The workflows never push to
   `main`, so the rule does not get in their way.

Before the first release, check in the Actions tab that CI passes on `dev`:
GPUI's tests on a runner without a screen and the first, uncached build have
not been run there yet.

## Releasing a version

1. On `dev`: raise `version` in `crates/app/Cargo.toml` (`Cargo.lock` follows),
   and rename the top changelog section to that version in **both**
   `CHANGELOG.md` and `CHANGELOG.fr.md` (`## [x.y.z] - YYYY-MM-DD`, with the
   summary and bullets in each).
   Run `cargo test --workspace`, then `cargo xtask release-notes <version>`
   (for example `cargo xtask release-notes 1.0.2`): it runs the changelog checks
   the Release run makes and prints the release notes, so a mistake shows now
   and not after the merge. `cargo xtask release-plan` shows what a push would
   do (it only publishes on `main`). Commit.
2. Open a pull request `dev` → `main`, wait for **Test (Windows)**, merge.
3. Watch the **Release** run. When it ends, the GitHub release holds the four
   assets; installed apps offer it at their next hourly check.

If the Release run fails after the build and leaves a draft release, delete
the draft in the Releases page and re-run the workflow (Actions › Release ›
Re-run, or `workflow_dispatch` on `main`): no tag exists until the draft is
published, so the run releases again. A published release cannot be repeated:
raise the version.

## Lost or leaked signing key

- **Your copy is lost but the secret is still set**: nothing breaks and
  releases keep being signed, but GitHub cannot show a secret back, so you
  have no copy for a new machine. Rotate as below if you want one.
- **Lost for good (the secret is gone too), or leaked**: the private key signs code that players' apps
  run elevated, so treat a leak as urgent.
  1. Remove the secret (`gh secret delete BELISK_UPDATE_SIGNING_KEY`) and, if
     it leaked, delete or mark as prerelease the releases it signed so
     `releases/latest` stops offering them.
  2. Make a new pair (`cargo xtask keygen`), commit the new public key over
     `crates/app/update-key.pub`, set the secret again, and release a new
     version signed with it.
  3. **Apps already installed trust only the old public key**, so they will
     reject the new release's signature and cannot update to it. Players must
     download the new full zip by hand once; say so in that release's notes.
     This is why the key must be backed up: with it, nothing has to be rotated
     after a mere loss of the file.

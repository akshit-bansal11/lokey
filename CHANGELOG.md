# Changelog

All notable changes to Saltcellar (called lokey until 1.0.0) are recorded here, following
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/). Versions follow
[Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-10-17

lokey is now Saltcellar, with a new look. There is no 0.4.0 release: the
rename, bumped as 0.4.0 and never tagged, ships here with the redesign. The
vault format is unchanged (v3): a lokey vault is copied to the new folder on
the first start, and opens with the same password.

### Added

- A copy of a lokey vault on the first start; see "Upgrading from lokey"
  below.
- A command palette on Ctrl+K: every command, with its shortcut, and
  "Switch to project …" for each project. Search keeps Ctrl+F and `/`.
- Ctrl+1 to Ctrl+9 open the first nine projects; Alt+↑ and Alt+↓ step
  through them, like Ctrl+PgUp and Ctrl+PgDn.
- Ctrl+Shift+C on a row copies the key's name; Ctrl+Enter in the add row
  adds the key.
- A website at https://saltcellar.akshitbansal.dev, with the documentation on
  one page at `/docs`. Like the app, it loads nothing from anywhere else.
- `SECURITY.md` (what the vault protects against, what it does not, and how to
  report a hole privately) and `CONTRIBUTING.md`.

### Changed

- **lokey is now Saltcellar.** The window, the exe and the repository carry
  the new name. Releases are named for their version: `Saltcellar-1.0.0.exe`
  beside `SHA256SUMS.txt`, under the title "Saltcellar v1.0.0". The permanent
  `releases/latest/download/lokey.exe` link stops at 0.3.0. The vault and
  backup files keep their `.lokey` names.
- The vault now lives in `%LOCALAPPDATA%\saltcellar`. `SALTCELLAR_DIR` chooses
  another folder; `LOKEY_DIR` still works.
- A new icon: a hopper crystal, the stepped, sinking squares rock salt grows
  into, which also read as stairs down to a cellar, with the keyhole at the
  bottom. Saffron and espresso, flat, with no gradients. The 16, 24 and 32 px
  sizes come from a simpler drawing (`icon-small.svg`) with fewer steps and a
  larger keyhole.
- A new look for the app: saffron on warm espresso neutrals, replacing the
  black, white and grey design (DES-19 records the change). Projects are a
  sidebar with key counts instead of a drop-down; it folds to a rail of
  initials below 760 px. Setup, unlock, restore and recovery are a centred
  card with the icon. Light text and focus rings use a dark amber, since
  saffron itself is too light to read on white.
- The shortcuts are listed in one place (`src/lib/shortcuts.ts`): the
  dispatch, the shortcuts dialog, the status bar hint and the button tooltips
  all read it, so they cannot drift apart.
- The website uses the same saffron palette, and shows the icon beside its
  name in the header, drawing the hero icon from SVG rather than a 1024 px
  PNG.
- The link previews (`og.png`, `assets/`) show the icon beside the app, and
  name the akshitbansal.dev domain.

### Upgrading from lokey

On its first start, Saltcellar copies your vault, its backup and its lockout
file from `%LOCALAPPDATA%\lokey` into `%LOCALAPPDATA%\saltcellar`, then opens
it with the same master password. The old folder is left exactly as it was;
delete it yourself once Saltcellar has opened your vault. Nothing is copied if
`%LOCALAPPDATA%\saltcellar` already has a vault, or if `SALTCELLAR_DIR` or
`LOKEY_DIR` is set. Close lokey before the first start. Each copy, or a failed
attempt, is recorded in `%LOCALAPPDATA%\saltcellar\migration.log`.

## [0.3.0] - 2026-09-27

There is no 0.2.0: the maintainer chose to number this release 0.3.0.
The vault format is unchanged (v3): nothing to upgrade.

### Added

- A password generator: the dice button or Ctrl+G. A slider sets the length
  (8 to 128), checkboxes choose uppercase, lowercase, digits and symbols, and
  every chosen kind appears at least once. Copy it (kept out of clipboard
  history, cleared after 30 seconds), or **Use as new key** to put it in the
  add row.

### Changed

- Built with Tauri 2.11.6 (was 2.11.5) and Tauri CLI 2.11.5 (was 2.11.4).

### Fixed

- The settings dialog's **Delete project** and **Delete all keys** buttons now
  show their trash icons; they were never drawn.

## [0.1.0] - 2026-09-27

### Added

- A recovery key. Setup asks what should happen if you forget the master
  password: get a recovery key (the default), shown once and typed back to
  confirm, or have no recovery at all. "Forgot the master password?" on the
  unlock screen uses the key to set a new password, then issues a new key; the
  used one stops working. Settings can turn it on, replace it or remove it.
- Encrypted backups. Settings, Export backup, asks for the master password and
  then where to save a copy of the encrypted vault. On a PC with no vault,
  Restore from a backup opens one with the password it was exported under.

### Changed

- **Vault format v3.** Entries are encrypted under a random data key, which is
  sealed under the master password's key and, optionally, the recovery key's.
  Changing the password or the recovery key replaces the data key, so an old
  password or recovery key opens only copies of the vault made before the
  change. A vault from an earlier build is upgraded the first time it is
  unlocked; earlier builds cannot open it afterwards.
- Changing the master password, exporting a backup and making or removing a
  recovery key now ask for the current master password, even with the vault
  open.
- Every GitHub Action in the build and release workflows is pinned to a full
  commit SHA.

### Removed

- The sealed inbox and the vault's public key. Only the command-line tool,
  removed before 0.0.1, ever wrote to it. A vault that still holds inbox
  records has them merged in on its first unlock.

## [0.0.2] - 2026-09-27

### Fixed

- `SHA256SUMS.txt` now uses LF line endings, so `sha256sum -c SHA256SUMS.txt`
  works on Linux, WSL and Git Bash. The app itself is unchanged from 0.0.1.

## [0.0.1] - 2026-09-24

The first release of lokey as it stands: a local-only encrypted key/value vault
for Windows, shipped as one portable `lokey.exe`. Earlier builds were
withdrawn; their history is in git.

### Added

- One master password opens the vault and allows everything: show, copy,
  add, edit and delete. Deleting a key, a project or all keys asks to confirm,
  naming what goes.
- The vault locks after 5 minutes without activity, when you press Ctrl+L, and
  whenever the app closes.
- Every action works from the keyboard. Press `F1` or `?` for the list.
- Values are encrypted with AES-256-GCM under an Argon2id key (64 MiB, 3
  passes); tampering makes the vault fail to open rather than read wrong.
- Wrong passwords slow down, and 10 in a row lock the vault for 15 minutes.
- Copied values stay out of clipboard history and clear after 30 seconds.
- The window is hidden from screenshots and screen sharing.
- No network code at all.

### Upgrading from an earlier build

A vault made by an earlier build opens as before with its master password. Its
deletion password is no longer asked for and is dropped the next time the
vault is saved. After that, the earlier builds cannot open the vault.

[Unreleased]: https://github.com/akshit-bansal11/saltcellar/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/akshit-bansal11/saltcellar/compare/v0.3.0...v1.0.0
[0.3.0]: https://github.com/akshit-bansal11/saltcellar/compare/v0.1.0...v0.3.0
[0.1.0]: https://github.com/akshit-bansal11/saltcellar/compare/v0.0.2...v0.1.0
[0.0.2]: https://github.com/akshit-bansal11/saltcellar/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/akshit-bansal11/saltcellar/releases/tag/v0.0.1

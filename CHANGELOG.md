# Changelog

All notable changes to lokey are recorded here, following
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/). Versions follow
[Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- A password generator: the dice button or Ctrl+G. A slider sets the length
  (8 to 128), checkboxes choose uppercase, lowercase, digits and symbols, and
  every chosen kind appears at least once. Copy it (kept out of clipboard
  history, cleared after 30 seconds), or **Use as new key** to put it in the
  add row.

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

[Unreleased]: https://github.com/akshit-bansal11/lokey/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/akshit-bansal11/lokey/compare/v0.0.2...v0.1.0
[0.0.2]: https://github.com/akshit-bansal11/lokey/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/akshit-bansal11/lokey/releases/tag/v0.0.1

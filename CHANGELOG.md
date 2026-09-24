# Changelog

All notable changes to lokey are recorded here, following
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/). Versions follow
[Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/akshit-bansal11/lokey/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/akshit-bansal11/lokey/releases/tag/v0.0.1

# Changelog

All notable changes to lokey are recorded here, following
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/). Versions follow
[Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0]

The first release under the name lokey. It replaces the PowerShell tool
secure-vault, which is kept at the `legacy-powershell` tag.

### Added

- `lokey` command-line tool: `set`, `get` (copies; `--view` prints), `view`,
  `projects`, `delete` (a key or a whole project), `truncate`, `import`,
  `init`, `passwd`, `status`.
- Desktop app: the vault as a three-column sheet (#, key, value), one project
  at a time, with show, copy, edit and delete on every row and an add row
  at the bottom.
- The app and the command-line tool share one vault file; changes made in a
  terminal appear in an open app window within a quarter of a second.
- `set` and `import` work without a password, sealing values with HPKE
  (RFC 9180, X25519) so only the master password can read them.
- Two passwords: master for everything, deletion for removing keys.
- Projects, so the same key name can exist once per project.
- Copying keeps the value out of Win+V history and clears it after 30 seconds
  unless something else was copied since.
- The app locks after 15 minutes idle and is hidden from screenshots.
- One-line installer for the command-line tool, with checksum verification.

### Changed

- Vault encryption: AES-256-GCM under an Argon2id (64 MiB, 3 passes) key,
  replacing PBKDF2. Vaults from secure-vault cannot be opened.
- Passwords must be at least 15 characters and not in public leaked-password
  lists.

### Removed

- The PowerShell implementation and its separate decrypt password.

### Security

- Header fields are bound into the encrypted body, so a downgraded file fails
  to open instead of opening with weaker settings.
- A copy of the vault's public key inside the encrypted body detects and
  reverses a swapped key, and the next unlock warns about it.
- Deleting a key or changing a password also rewrites the backup file, so the
  old value or password does not survive in it.

[Unreleased]: https://github.com/akshit-bansal11/lokey/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/akshit-bansal11/lokey/releases/tag/v0.1.0

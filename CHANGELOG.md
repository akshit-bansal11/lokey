# Changelog

All notable changes to lokey are recorded here, following
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/). Versions follow
[Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Keyboard shortcuts for everything in the vault.** `Ctrl+N` adds a key,
  `Ctrl+Shift+N` makes a project, `Ctrl+PgUp` / `Ctrl+PgDn` switch projects,
  `Ctrl+F` or `Ctrl+K` search, `Ctrl+,` opens Settings. In the rows, `Home` and
  `End` jump to the first and last key, and `Esc` hides a shown value, then
  goes back to search.
- A keyboard shortcuts list: press `F1` or `?`, or use the keyboard button in
  the toolbar.

## [0.3.0] - 2026-09-20

### Removed

- **The `lokey` command-line tool.** lokey is the desktop app now. `lokey.exe`,
  its installer script and the `lokey set`, `get`, `view`, `projects`,
  `delete`, `truncate`, `import`, `init`, `passwd` and `status` commands are
  gone. Your vault is untouched: the app opens the same file, with the same
  passwords, and merges anything the command line had left pending.
- **The setup program.** A release is now one portable `lokey.exe` you run from
  wherever you put it, plus `SHA256SUMS.txt`. If you installed 0.2.0, uninstall
  it from Settings › Apps as usual; the vault is not part of it.

### Changed

- The app's own binary is now `lokey.exe`, not `lokey-app.exe`.

## [0.2.0] - 2026-09-20

### Fixed

- Password prompts in PowerShell, cmd, Windows Terminal and VS Code showed
  nothing while you typed, so typing and pasting looked ignored. Each
  character now shows as `*`.
- Password prompts hung in a Git Bash, MSYS2 or Cygwin (mintty) window. They
  now read with echo turned off, and refuse rather than show a password where
  echo cannot be turned off.
- Changing only the master or only the deletion password could make the two
  equal, so the master password alone authorised deletes. It is now refused,
  in the command-line tool and the app.
- `lokey import` pasted into a terminal stopped at the first blank line and
  left the rest of the paste for the shell to run. It now reads to Ctrl+Z
  (Ctrl+D in Git Bash).
- `lokey import` dropped a trailing `;` from `KEY=VALUE` lines.
- `get` and `delete` without `project=` said a missing key was not in
  `default` when every project had been searched; the help now says so too.
- The app: a change from a terminal no longer wipes a value you are editing.
- The app: errors while opening a value for editing are shown instead of lost.
- The app: if a vault appears while the setup screen is open, Create goes to
  the unlock screen instead of failing.
- The app retries reading the vault after a failed read, instead of missing
  that change until the next write.
- The app window no longer freezes while a password is being checked.
- `install.ps1` no longer leaves its settings in your PowerShell session, and
  upgrades `lokey.exe` while a clipboard-clear helper is still running.
- A release tag that does not match the version in the code now fails the
  release instead of shipping mislabelled files.

## [0.1.0] - 2026-09-19

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

[Unreleased]: https://github.com/akshit-bansal11/lokey/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/akshit-bansal11/lokey/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/akshit-bansal11/lokey/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/akshit-bansal11/lokey/releases/tag/v0.1.0

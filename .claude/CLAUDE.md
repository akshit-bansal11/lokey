This project inherits every rule in ~/.claude/CLAUDE.md. Rules below add to or override it. Nothing here restates it.

# Saltcellar

A local-only encrypted key/value vault for Windows: a Tauri desktop app over
one engine, `saltcellar-core`. Public repo, released from tags as a single
portable `Saltcellar-<version>.exe` (`tauri build --no-bundle`; there is no
installer). Running `release.yml` by hand makes a private draft release. It was called lokey until 0.4.0. The `lokey`
command-line tool was removed on 2026-09-20 — do not reintroduce a second
front-end without asking.

## Facts a session needs before touching code

- **No Rust toolchain on the maintainer's PC, by choice.** Compiling happens on
  GitHub Actions. Verify Rust changes through CI, not locally.
- **Any Cargo.toml change** is followed by `gh workflow run lockfile.yml --ref <branch>`
  and committing the `Cargo.lock` artifact it uploads. CI installs `--locked`.
- **Formatting Rust without a toolchain:** a standalone `rustfmt` binary (with
  its rustc DLLs) can format files locally; `cargo fmt --check` in CI is the gate.
- **The maintainer may be gaming.** Ask before anything long-running or
  memory-hungry locally; prefer CI.
- **Never add network code.** "No network, ever" is the product claim: no
  telemetry, update checks, crash reporting or remote fonts. The Tauri CSP only
  allows `self` and the IPC channel.
- **Password rules are product decisions, not bugs:** one master password
  opens the vault, is asked for again after every lock, and then allows
  everything. The deletion password was removed on 2026-09-24 by the
  maintainer's choice; deletes only confirm in a dialog (a chosen exception to
  UX-05). Do not reintroduce a second password without asking. The app locks
  after 5 minutes idle and on exit.
- **Recovery is a recovery key or nothing** (2026-09-27, `DECISIONS.md`): no
  hints, no security questions. Exporting, changing the master password and
  making or removing a recovery key re-ask the master password. Any change to
  the password or the recovery key must replace the data key (`rekey` in
  `vault.rs`).
- **Vault format v3 has no inbox.** `hpke` and `merge_inbox` remain only to
  upgrade v1/v2 vaults; see `OPEN_ITEMS.md` before removing them.

- **The rename to Saltcellar (0.4.0) kept the vault format.** The format tag
  `"lokey"` (`format.rs`, bound into every vault's associated data), the HPKE
  label `lokey inbox v1` and the `vault.lokey*` / `.lokey` file names stay as
  they are; changing any of them makes existing vaults or backups unreadable.
  `Store::open_default` copies a vault from `%LOCALAPPDATA%\lokey` once
  (copy, never move; see `Migration` in `store.rs`). `LOKEY_DIR` is still read
  after `SALTCELLAR_DIR`.

- **`site/` is the website** (saltcellar.vercel.app), a separate Next.js app
  with its own lockfile and gate (`cd site && npm run check:ci`, `site.yml`).
  It follows the same rule: no analytics, trackers, remote fonts or third-party
  requests.

## Gate

- `npm run check` fixes formatting in place; `npm run check:ci` is the
  non-mutating twin that CI runs. Keep them in step.
- `main` is protected: the `gate (lint, types, tests)` check must pass.

## History

Project history before 2026-09-19 (the Env Protect / secure-vault era, the
rename, every design decision) is on the Notion page with reference ID
`yfu0idauhvj2nm9nkp0lcgkd7-lcmdjsfbbr`, in the Mini Projects Index. The
PowerShell original is at the `legacy-powershell` tag.

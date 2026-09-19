This project inherits every rule in ~/.claude/CLAUDE.md. Rules below add to or override it. Nothing here restates it.

# lokey

A local-only encrypted key/value vault for Windows: the `lokey` CLI and a Tauri
desktop app over one engine, `lokey-core`. Public repo, released from tags.

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
- **Password rules are product decisions, not bugs:** `set` and `import` need
  no password; every other command asks for the master password every time;
  `delete` and `truncate` also need the deletion password.

## Gate

- `npm run check` fixes formatting in place; `npm run check:ci` is the
  non-mutating twin that CI runs. Keep them in step.
- `main` is protected: the `gate (lint, types, tests)` check must pass.

## History

Project history before 2026-09-19 (the Env Protect / secure-vault era, the
rename, every design decision) is on the Notion page with reference ID
`yfu0idauhvj2nm9nkp0lcgkd7-lcmdjsfbbr`, in the Mini Projects Index. The
PowerShell original is at the `legacy-powershell` tag.

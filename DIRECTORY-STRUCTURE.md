# DIRECTORY-STRUCTURE.md — lokey

**Generated:** 2026-09-19 · **Command:** `git ls-files`, rendered as a tree by a short Python loop (no `tree` binary on the machine)

## Excluded from the walk

Untracked paths: `.git` · `node_modules` · `target` · `dist` · `src-tauri/gen`

## Tree

```text
lokey/
├── .cargo/
│   └── audit.toml
├── .claude/
│   └── CLAUDE.md
├── .github/
│   ├── workflows/
│   │   ├── ci.yml
│   │   ├── lockfile.yml
│   │   └── release.yml
│   └── dependabot.yml
├── crates/
│   ├── lokey-cli/
│   │   ├── src/
│   │   │   ├── args.rs
│   │   │   ├── commands.rs
│   │   │   ├── import.rs
│   │   │   ├── main.rs
│   │   │   └── prompt.rs
│   │   ├── tests/
│   │   │   └── cli.rs
│   │   └── Cargo.toml
│   └── lokey-core/
│       ├── src/
│       │   ├── clipboard.rs
│       │   ├── common-passwords.txt
│       │   ├── crypto.rs
│       │   ├── error.rs
│       │   ├── format.rs
│       │   ├── lib.rs
│       │   ├── lockout.rs
│       │   ├── names.rs
│       │   ├── password.rs
│       │   ├── store.rs
│       │   └── vault.rs
│       └── Cargo.toml
├── scripts/
│   └── install.ps1
├── src/
│   ├── lib/
│   │   ├── api.ts
│   │   ├── dom.ts
│   │   ├── icons.ts
│   │   ├── rows.test.ts
│   │   ├── rows.ts
│   │   ├── status.ts
│   │   └── types.ts
│   ├── styles/
│   │   ├── app.css
│   │   └── tokens.css
│   ├── views/
│   │   ├── dialogs.ts
│   │   ├── setup.ts
│   │   ├── unlock.ts
│   │   └── vault.ts
│   └── main.ts
├── src-tauri/
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/
│   │   ├── 128x128.png
│   │   ├── 128x128@2x.png
│   │   ├── 32x32.png
│   │   ├── icon.ico
│   │   └── icon.png
│   ├── src/
│   │   ├── dto.rs
│   │   └── main.rs
│   ├── build.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── .gitattributes
├── .gitignore
├── .nvmrc
├── biome.json
├── Cargo.lock
├── Cargo.toml
├── CHANGELOG.md
├── index.html
├── LICENSE
├── package-lock.json
├── package.json
├── README.md
├── STATE.md
├── TECH-STACK.md
├── tsconfig.json
└── vite.config.ts
```

## Rationale — one line per directory

| Directory | Why it exists |
| --- | --- |
| `crates/lokey-core/` | The engine. The only code that touches cryptography or the vault file, shared by CLI and app so they cannot disagree about the format. |
| `crates/lokey-cli/` | The `lokey` command. Parses, prompts, prints; no vault logic of its own. |
| `crates/lokey-cli/tests/` | End-to-end tests that run the real binary. Rust's convention for integration tests. |
| `src-tauri/` | The desktop app's Rust side: commands the page calls, the file watcher, window config. |
| `src-tauri/capabilities/` | Tauri permissions: the page may only listen to vault events. |
| `src-tauri/icons/` | App and installer icons. |
| `src/` | The page shown in the app window (Vite root is the repo root, which is Tauri's standard layout). |
| `src/lib/` | Page modules without screens: the typed IPC client, DOM helpers, icons, status bar, pure row logic. |
| `src/views/` | One module per screen or dialog group. |
| `src/styles/` | The token file and the stylesheet that consumes it. |
| `scripts/` | `install.ps1`, published with every release. |
| `.github/workflows/` | CI (`ci.yml`), release on tag (`release.yml`), lockfile resolution (`lockfile.yml`). |
| `.cargo/` | `audit.toml`: accepted RustSec warnings with reasons and expiry. |
| `.claude/` | Project rules for agents. |

## Known deviations

| Path | Deviates how | Reason | Closes when |
| --- | --- | --- | --- |
| `crates/*/src/*.rs` | Rust unit tests live in `#[cfg(test)]` modules inside the source file, not beside it | Rust's convention; it is the same file, so trivially co-located | never |

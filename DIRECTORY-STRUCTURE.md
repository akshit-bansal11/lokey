# DIRECTORY-STRUCTURE.md — lokey

**Generated:** 2026-09-20 · **Command:** `git ls-files`, rendered as a tree by a short Python loop (no `tree` binary on the machine)

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
├── DIRECTORY-STRUCTURE.md
├── index.html
├── LICENSE
├── OPEN_ITEMS.md
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
| `crates/lokey-core/` | The engine. The only code that touches cryptography or the vault file. Separate from the app so the vault logic can be tested without a window. |
| `src-tauri/` | The desktop app's Rust side: commands the page calls, the file watcher, window config. |
| `src-tauri/capabilities/` | Tauri permissions: the page may only listen to vault events. |
| `src-tauri/icons/` | App icons, embedded in the exe. |
| `src/` | The page shown in the app window (Vite root is the repo root, which is Tauri's standard layout). |
| `src/lib/` | Page modules without screens: the typed IPC client, DOM helpers, icons, status bar, pure row logic. |
| `src/views/` | One module per screen or dialog group. |
| `src/styles/` | The token file and the stylesheet that consumes it. |
| `.github/workflows/` | CI (`ci.yml`), release on tag (`release.yml`), lockfile resolution (`lockfile.yml`). |
| `.cargo/` | `audit.toml`: accepted RustSec warnings with reasons and expiry. |
| `.claude/` | Project rules for agents. |

## Known deviations

| Path | Deviates how | Reason | Closes when |
| --- | --- | --- | --- |
| `crates/*/src/*.rs` | Rust unit tests live in `#[cfg(test)]` modules inside the source file, not beside it | Rust's convention; it is the same file, so trivially co-located | never |

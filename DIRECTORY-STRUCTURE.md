# DIRECTORY-STRUCTURE.md — Saltcellar

**Generated:** 2026-10-16 · **Command:** `git ls-files`, rendered as a tree by a short Python loop (no `tree` binary on the machine)

## Excluded from the walk

Untracked paths: `.git` · `node_modules` · `target` · `dist` · `src-tauri/gen` · `site/.next` · `site/out` · `site/node_modules`

## Tree

```text
saltcellar/
├── .cargo/
│   └── audit.toml
├── .claude/
│   └── CLAUDE.md
├── .github/
│   ├── workflows/
│   │   ├── ci.yml
│   │   ├── lockfile.yml
│   │   ├── release.yml
│   │   └── site.yml
│   └── dependabot.yml
├── crates/
│   └── saltcellar-core/
│       ├── src/
│       │   ├── clipboard.rs
│       │   ├── common-passwords.txt
│       │   ├── crypto.rs
│       │   ├── error.rs
│       │   ├── format.rs
│       │   ├── generator.rs
│       │   ├── lib.rs
│       │   ├── lockout.rs
│       │   ├── names.rs
│       │   ├── password.rs
│       │   ├── store.rs
│       │   └── vault.rs
│       └── Cargo.toml
├── site/
│   ├── app/
│   │   ├── docs/
│   │   │   └── page.tsx
│   │   ├── globals.css
│   │   ├── icon.png
│   │   ├── layout.tsx
│   │   ├── page.tsx
│   │   ├── robots.ts
│   │   └── sitemap.ts
│   ├── components/
│   │   ├── icons/
│   │   │   └── github.tsx
│   │   ├── sections/
│   │   │   └── docs.tsx
│   │   ├── ui/
│   │   │   ├── animated-shiny-text.tsx
│   │   │   ├── badge.tsx
│   │   │   ├── blur-fade.tsx
│   │   │   ├── button.tsx
│   │   │   ├── card.tsx
│   │   │   ├── grid-pattern.tsx
│   │   │   └── tabs.tsx
│   │   ├── code-block.tsx
│   │   ├── copy-button.tsx
│   │   ├── section.tsx
│   │   ├── site-footer.tsx
│   │   ├── site-header.tsx
│   │   ├── theme-provider.tsx
│   │   ├── theme-toggle.tsx
│   │   └── toc.tsx
│   ├── lib/
│   │   ├── nav.ts
│   │   └── utils.ts
│   ├── public/
│   │   ├── github.svg
│   │   ├── icon.png
│   │   └── og.png
│   ├── .editorconfig
│   ├── .gitignore
│   ├── biome.json
│   ├── components.json
│   ├── eslint.config.mjs
│   ├── next.config.ts
│   ├── package-lock.json
│   ├── package.json
│   ├── postcss.config.mjs
│   ├── README.md
│   └── tsconfig.json
├── src/
│   ├── lib/
│   │   ├── api.ts
│   │   ├── dom.ts
│   │   ├── icons.ts
│   │   ├── recovery.test.ts
│   │   ├── recovery.ts
│   │   ├── rows.test.ts
│   │   ├── rows.ts
│   │   ├── status.ts
│   │   └── types.ts
│   ├── styles/
│   │   ├── app.css
│   │   └── tokens.css
│   ├── views/
│   │   ├── dialogs.ts
│   │   ├── generator.ts
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
│   │   ├── icon-small.svg
│   │   ├── icon.ico
│   │   ├── icon.png
│   │   └── icon.svg
│   ├── src/
│   │   ├── dto.rs
│   │   ├── file_dialog.rs
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
├── CONTRIBUTING.md
├── DECISIONS.md
├── DIRECTORY-STRUCTURE.md
├── index.html
├── LICENSE
├── OPEN_ITEMS.md
├── package-lock.json
├── package.json
├── README.md
├── SECURITY.md
├── STATE.md
├── TECH-STACK.md
├── tsconfig.json
├── vercel.json
└── vite.config.ts
```

## Rationale — one line per directory

| Directory | Why it exists |
| --- | --- |
| `crates/saltcellar-core/` | The engine. The only code that touches cryptography or the vault file. Separate from the app so the vault logic can be tested without a window. |
| `src-tauri/` | The desktop app's Rust side: commands the page calls, the file watcher, window config. |
| `src-tauri/capabilities/` | Tauri permissions: the page may only listen to vault events. |
| `src-tauri/icons/` | App icons, embedded in the exe. |
| `src/` | The page shown in the app window (Vite root is the repo root, which is Tauri's standard layout). |
| `src/lib/` | Page modules without screens: the typed IPC client, DOM helpers, icons, status bar, pure row logic. |
| `src/views/` | One module per screen or dialog group. |
| `src/styles/` | The token file and the stylesheet that consumes it. |
| `.github/workflows/` | CI (`ci.yml`), release on tag or as a hand-run draft (`release.yml`), lockfile resolution (`lockfile.yml`), the website's gate (`site.yml`). |
| `site/` | The website at saltcellar.vercel.app: landing page and one-page docs. Its own Next.js app, lockfile and gate, copied from Edgepad's site; the app's gate never reads it. |
| `site/app/` | The two routes (`/`, `/docs`), the layout, robots and sitemap. |
| `site/components/` | Page parts: header, footer, sidebar, the docs sections; `ui/` holds the shadcn-style primitives. |
| `.cargo/` | `audit.toml`: accepted RustSec warnings with reasons and expiry. |
| `.claude/` | Project rules for agents. |

## Known deviations

| Path | Deviates how | Reason | Closes when |
| --- | --- | --- | --- |
| `crates/*/src/*.rs` | Rust unit tests live in `#[cfg(test)]` modules inside the source file, not beside it | Rust's convention; it is the same file, so trivially co-located | never |

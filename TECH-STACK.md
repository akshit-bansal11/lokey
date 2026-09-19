# TECH-STACK.md — lokey

**Last verified:** 2026-09-19 · **Verified with:** `Cargo.lock`, `package-lock.json`

## Runtime target

| Thing | Version | Support ends | Why this one |
| --- | --- | --- | --- |
| Rust | stable (1.98.1 at release) | rolling | `rust-version = 1.89` is the floor: `File::lock` and let-chains need it |
| Node.js (build only) | 26 (`.nvmrc`); `engines >=24` | 26: 27 Oct 2027 | Active LTS. Node never ships to users; it builds the page |
| Package manager | npm 12 | n/a | default |
| Lockfiles | `Cargo.lock` and `package-lock.json` committed | n/a | CI uses `--locked` and `npm ci` only |
| Target OS | Windows 10/11 x64 with WebView2 | n/a | the clipboard code and the app installer are Windows-only |

## Core

| Layer | Chosen | Version | Chosen over | Because |
| --- | --- | --- | --- | --- |
| Desktop shell | Tauri | 2.11.5 (`@tauri-apps/api` 2.11.1, CLI 2.11.4) | C#/.NET WPF, Electron | Owner's choice; small binaries using the system WebView2 |
| Language | Rust (edition 2024) + TypeScript | TS 7.0.2 | PowerShell (the original) | one engine shared by CLI and app |
| Page build | Vite | 8.3.0 | a framework (React etc.) | four screens; 22 KB of JS without one |
| Styling | plain CSS with custom-property tokens | n/a | Tailwind | one tokens file, no build plugin |
| Icons | Lucide geometry, inlined | lucide-static 1.47.0 | an icon package | no dependency, no request |
| Cipher | aes-gcm | 0.11.1 | ChaCha20-Poly1305 | hardware AES on x64; standard |
| Password KDF | argon2 (Argon2id) | 0.6.0 | PBKDF2 (the original) | memory-hard; SECURITY-SPEC SEC-05 |
| Password-less `set` | hpke (RFC 9180, X25519/HKDF-SHA256/AES-256-GCM) | 0.14.1 | hand-rolled ECIES | a standard, reviewed construction |
| Secret wiping | zeroize | 1.9.0 | nothing | zeroes keys and values on drop |
| Serialisation | serde + serde_json | 1.0.229 / 1.0.151 | a binary format | readable header, simple versioning |
| Randomness | getrandom | 0.4.3 | rand | OS RNG directly, smallest API |
| Encoding | base64 | 0.23.1 | hex | shorter vault file |
| Hidden prompt | rpassword | 7.5.4 | a TUI library | one function needed |
| Windows API | windows-sys | 0.61.2 | the `windows` crate, arboard | raw clipboard calls needed for the history-exclusion formats |
| Unit / integration tests | cargo test, node:test | built in | Vitest | no extra dependency |
| Browser tests | none | n/a | Playwright | see "Deliberately not used" |
| Formatter | rustfmt, Biome | Biome 2.5.14 | Prettier | Biome also lints JS and CSS |
| Linter — language | clippy (`-D warnings`), Biome | | ESLint | one tool per language |
| Linter — stylesheets | Biome | 2.5.14 | Stylelint | Biome covers CSS |
| Types for tooling | @types/node | 26.6.2 | none | node:test and vite.config typing |
| CI | GitHub Actions | n/a | default | |
| Hosting | GitHub Releases | n/a | Vercel (the pipeline default) | a native app has nothing to host |

## Services and their environment variables

| Service | What it does | Env vars | Set in | Rotation owner |
| --- | --- | --- | --- | --- |
| none | lokey has no network service | `LOKEY_DIR` (optional: vault folder, read by CLI and app) | user's environment | n/a |

## Scripts

| Script | Command | What it is for |
| --- | --- | --- |
| `dev` | `vite` | page dev server for `tauri dev` |
| `build` | `vite build` | the page, embedded by the app build |
| `check` | Biome write, tsc, node:test, vite build, cargo fmt, clippy, cargo test | **The quality gate.** |
| `check:ci` | same, non-mutating (`biome ci`, `cargo fmt --check`, `--locked`) | what CI and the release run |
| `test:ui` | `node --test "src/**/*.test.ts"` | page logic tests |

## Values this project must record

| Value | Setting | Recorded because |
| --- | --- | --- |
| Argon2id | 64 MiB, t=3, p=1 | SECURITY-SPEC SEC-05; stored per vault in the header |
| Master / deletion password minimum | 15 characters, max 1024, leaked-list check | SECURITY-SPEC SEC-06 (NIST SP 800-63B-4) |
| Lockout | 10 wrong passwords then 15 minutes; backoff 1s doubling to 30s | shared by CLI and app via `lockout.json` |
| App idle lock | 15 minutes | `IDLE_LOCK` in `src-tauri/src/main.rs` |
| Clipboard clear | 30 seconds, only if unchanged | `CLEAR_AFTER` in `clipboard.rs` |
| Value size cap | 64 KiB | keeps the file every command re-reads small |
| Coverage threshold | not set | UNKNOWN: no coverage tool wired yet |
| SLSA build level target | L2 (hosted build, signed provenance via `attest-build-provenance`) | release.yml |

## Deliberately not used

| Not used | Why | Revisit when |
| --- | --- | --- |
| Any network service (Better Stack, analytics, update check) | "No network, ever" is the product | never, without a new product decision |
| Code signing | the certificate costs money yearly | there are enough users that SmartScreen warnings cost adoption |
| Browser/UI tests (Playwright, WebDriver) | Tauri WebDriver on Windows adds a driver dependency | the app grows past a handful of screens |
| MSI bundle | NSIS per-user install needs no admin | an organisation asks for MSI |

## Dead dependency sweep

**Last run:** never. Every runtime dependency above has a named importer as of 2026-09-19 (read from source), but no tool has run.

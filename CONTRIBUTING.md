# Contributing

Thanks for looking. Saltcellar is small on purpose; the bar for a change is that it makes the vault better for someone using it, and that it passes the same gate CI runs.

## Rules that are not up for debate

- **No network code, ever.** No telemetry, update checks, crash reporting or remote fonts. The Tauri content security policy allows only `self` and the IPC channel, and stays that way.
- **The vault format does not change casually.** A change bumps `VERSION` in `format.rs` and keeps reading the old version. The format tag `"lokey"`, the HPKE label `lokey inbox v1` and the `vault.lokey*` / `.lokey` file names stay as they are: existing vaults and backups depend on them.
- **The Argon2id cost is never lowered,** not even for tests.
- **Every cryptographic primitive and every source of randomness lives in `saltcellar-core`.** The page shows what the Rust side gives it.
- **A new dependency needs a reason** in the pull request.

## Setting up

Fork the repository on GitHub and clone your fork.

```powershell
git clone https://github.com/<you>/saltcellar.git
cd saltcellar
git remote add upstream https://github.com/akshit-bansal11/saltcellar.git
npm ci
```

You need Rust (stable, at least the `rust-version` in `Cargo.toml`) and Node.js 24 or newer, on Windows 10 or 11: the clipboard and file-dialog code only compiles there. `npx tauri dev` runs the app against the page's dev server.

No Rust toolchain on your machine? That is how the maintainer works. Push to a branch with a pull request open and CI runs the whole gate on Windows. After any change to a `Cargo.toml`, run the **lockfile** workflow on your branch and commit the `Cargo.lock` it uploads: CI installs with `--locked`.

## Making a change

1. Branch from an up-to-date `main` on your fork.
2. Add or change a test when you add logic, a branch or fix a bug. The engine in `crates/saltcellar-core` is plain Rust and cheap to test; page logic that needs no DOM goes in a `*.ts` module under `src/lib` with a `node:test` file beside it.
3. Run `npm run check` until it is clean. It formats in place, then lints, type-checks, runs every test and builds the page. CI runs `npm run check:ci`, the same checks without writing to any file. Do not suppress a finding to get green: no `#[allow]`, no `biome-ignore`, no `any`, no `@ts-ignore`, no `console.log`.
4. Commit with a [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) subject (`feat:`, `fix:`, `docs:`, `ci:`, with an optional scope) and a body that says what changed and why, including what you rejected.
5. Add a line under **Unreleased** in [CHANGELOG.md](CHANGELOG.md).
6. Open a pull request against `main`. The `gate (lint, types, tests)` check must pass before it can merge.

## Style

- Constants are named. A number that appears twice is a constant.
- Doc comments say why, not what the next line already says.
- Errors say what happened and what to do, in a sentence a person can act on.

## Releases

The maintainer builds and publishes releases from tags. A pull request does not bump the version, tag, or build a release; a merged change ships in the next one.

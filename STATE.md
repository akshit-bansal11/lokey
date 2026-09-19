# STATE.md — lokey

**Updated:** 2026-09-19 · **Branch:** `release/0.2.0` · **Last release:** `v0.1.0`

## Bootstrap record

| Field | Value |
| --- | --- |
| Bootstrapped | 2026-09-19 (moved from `F:/projects/mini-projects/lokey-lcmdjsfbbr`) |
| Stack detected as | Rust 2024 workspace (lokey-core, lokey-cli, src-tauri) + Vite/TypeScript page, from Cargo.lock and package-lock.json |
| Quality gate command | `npm run check` (CI: `npm run check:ci`) |
| Deployed at | not deployed; released at https://github.com/akshit-bansal11/lokey/releases/tag/v0.1.0 |

## Where the work is right now

- **In flight:** 0.2.0 on PR #2: password prompts show `*` and work in mintty; 11 CLI/app fixes from a code audit (see CHANGELOG).
- **Blocked on:** Nothing.
- **Next action:** the maintainer runs the PR build's `lokey init` in PowerShell and in a Git Bash window, then the PR merges and `v0.2.0` is tagged.

## Verified facts

| Fact | Proved by | Verified |
| --- | --- | --- |
| All five v0.1.0 assets download from `releases/latest/download/` | `curl -sL -o f -w '%{http_code}' .../latest/download/<file>` returned 200 each | 2026-09-19 |
| Release checksums match the files | `sha256sum -c SHA256SUMS.txt` | 2026-09-19 |
| The three exes carry a valid build attestation | `gh attestation verify <exe> --repo akshit-bansal11/lokey` exit 0 | 2026-09-19 |
| 80 tests pass: 50 core, 11 CLI unit, 9 CLI end-to-end, 10 page | CI gate log, run 35438355151 | 2026-09-19 |
| Sizes: lokey.exe 796,160 B, lokey-app.exe 8,071,680 B, installer 1,887,352 B | `gh release view v0.1.0 --json assets` | 2026-09-19 |
| The downloaded CLI runs: `--version`, `status`, no-vault `set` | run locally against a scratch `LOKEY_DIR` | 2026-09-19 |

## Standing hazards

| Hazard | Why it is dangerous here | Safe alternative |
| --- | --- | --- |
| Editing `Cargo.toml` without refreshing `Cargo.lock` | CI runs `--locked` and fails; nobody can regenerate it locally | Dispatch `lockfile.yml` on the branch, commit its artifact |
| Changing the vault format | Existing users' vaults stop opening | Bump `VERSION` in `format.rs` and read the old version |
| Lowering Argon2id parameters, even in tests | Weakens every vault created afterwards | Keep 64 MiB / t=3 / p=1; tests use fewer password operations |

## Environment state

| Thing | State | Verified |
| --- | --- | --- |
| Branch protection on default branch | on; force-push and delete disabled | `gh api repos/akshit-bansal11/lokey/branches/main/protection`, 2026-09-19 |
| Required status checks | `gate (lint, types, tests)`, strict | same call, 2026-09-19 |
| Deploy approval environment exists with a reviewer | no; releases run on tag push, solo maintainer | 2026-09-19 |
| Secrets present in CI | none needed (GITHUB_TOKEN only) | 2026-09-19 |
| Backup provisioned and drilled | n/a: no server data | 2026-09-19 |

## Test status, honestly

| Suite | Status | Note |
| --- | --- | --- |
| unit / integration | green | Rust + node:test, CI run 35438355151 |
| CLI password-prompt commands end to end | never run | need a console; the 0.2.0 prompt (`*` feedback, mintty `stty` path) is unverified on a real terminal |
| desktop window | never run | no automated UI test; needs a manual pass |
| `install.ps1` end to end | never run | parse-checked on PS 5.1 and 7; running it would change the user PATH |

## Continuity file health

| File | Exists | Last updated | Current? |
| --- | --- | --- | --- |
| `TECH-STACK.md` | yes | 2026-09-19 | yes |
| `DIRECTORY-STRUCTURE.md` | yes | 2026-09-19 | yes |
| `DECISIONS.md` | no | | decisions to date are on the Notion page |
| `DRIFT.md` | no | | no known drift |
| `OPEN_ITEMS.md` | no | | accepted RustSec warnings live in `.cargo/audit.toml` (expire 2026-12-19) |

## Session handoff

- **Do not touch:** `F:/projects/mini-projects/lokey-lcmdjsfbbr` and `env-protect-lcmdjsfbbr` are stale copies; delete them, never edit them.
- **Half-applied changes:** none.
- **Open questions:** whether the app window behaves correctly on a real desktop (the maintainer answers by using it).

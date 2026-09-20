# STATE.md — lokey

**Updated:** 2026-09-20 · **Branch:** `chore/remove-cli` · **Last release:** `v0.2.0`

## Bootstrap record

| Field | Value |
| --- | --- |
| Bootstrapped | 2026-09-19 (moved from `F:/projects/mini-projects/lokey-lcmdjsfbbr`) |
| Stack detected as | Rust 2024 workspace (lokey-core, lokey-cli, src-tauri) + Vite/TypeScript page, from Cargo.lock and package-lock.json. `lokey-cli` removed 2026-09-20 |
| Quality gate command | `npm run check` (CI: `npm run check:ci`) |
| Deployed at | not deployed; released at https://github.com/akshit-bansal11/lokey/releases/tag/v0.2.0 |

## Where the work is right now

- **In flight:** `chore/remove-cli`: the command-line tool, its installer script and the NSIS bundle are gone; a release is one portable `lokey.exe`. Staged under `[Unreleased]`, not versioned yet.
- **Blocked on:** Nothing.
- **Next action:** open the PR, let the gate and the build job run, and check the artifact holds exactly one exe named `lokey.exe`. Then decide the version (`OPEN_ITEMS.md`).

## Verified facts

| Fact | Proved by | Verified |
| --- | --- | --- |
| All five v0.1.0 assets downloaded from `releases/latest/download/` (that release shipped the CLI; a release now ships two files) | `curl -sL -o f -w '%{http_code}' .../latest/download/<file>` returned 200 each | 2026-09-19 |
| Release checksums match the files | `sha256sum -c SHA256SUMS.txt` | 2026-09-19 |
| The three exes carry a valid build attestation | `gh attestation verify <exe> --repo akshit-bansal11/lokey` exit 0 | 2026-09-19 |
| 80 tests pass: 50 core, 11 CLI unit, 9 CLI end-to-end, 10 page | CI gate log, run 35438355151 | 2026-09-19 |
| Sizes: lokey.exe 796,160 B, lokey-app.exe 8,071,680 B, installer 1,887,352 B | `gh release view v0.1.0 --json assets` | 2026-09-19 |
| Removing `lokey-cli` costs the lock exactly three packages (`lokey-cli`, `rpassword`, `rtoolbox`) and moves no version | `lockfile.yml` run 35504833456, diff read | 2026-09-20 |

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
| page logic (node:test) | green | 10 tests, run locally 2026-09-20 |
| Rust unit / integration | not run since the CLI was removed | no local toolchain; CI is the proof, and the 9 end-to-end CLI tests went with the crate |
| desktop window | never run | no automated UI test; needs a manual pass |
| the app opens a vault written by the old CLI | never run | the inbox merge is the path that matters; needs a manual pass with a 0.2.0 vault |

## Continuity file health

| File | Exists | Last updated | Current? |
| --- | --- | --- | --- |
| `TECH-STACK.md` | yes | 2026-09-20 | yes |
| `DIRECTORY-STRUCTURE.md` | yes | 2026-09-20 | yes |
| `DECISIONS.md` | no | | decisions to date are on the Notion page |
| `DRIFT.md` | no | | no known drift |
| `OPEN_ITEMS.md` | yes | 2026-09-20 | yes |

## Session handoff

- **Do not touch:** `F:/projects/mini-projects/lokey-lcmdjsfbbr` and `env-protect-lcmdjsfbbr` are stale copies; delete them, never edit them.
- **Half-applied changes:** none.
- **Open questions:** whether the app window behaves correctly on a real desktop (the maintainer answers by using it); whether the sealed inbox should leave the format now that nothing writes it (`OPEN_ITEMS.md`).

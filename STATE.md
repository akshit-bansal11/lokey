# STATE.md — lokey

**Updated:** 2026-09-27 · **Branch:** `feat/recovery-key-and-export` · **Last release:** `v0.0.2` (v0.1.0 in flight)

## Bootstrap record

| Field | Value |
| --- | --- |
| Bootstrapped | 2026-09-19 (moved from `F:/projects/mini-projects/lokey-lcmdjsfbbr`) |
| Stack detected as | Rust 2024 workspace (lokey-core, lokey-cli, src-tauri) + Vite/TypeScript page, from Cargo.lock and package-lock.json. `lokey-cli` removed 2026-09-20 |
| Quality gate command | `npm run check` (CI: `npm run check:ci`) |
| Deployed at | not deployed; released from tags at https://github.com/akshit-bansal11/lokey/releases |

## Where the work is right now

- **In flight:** v0.1.0 on `feat/recovery-key-and-export` (PR #13): vault format v3 (random data key, optional recovery key, inbox dropped), encrypted backup export and restore, the master password re-asked before export, password change and recovery-key changes, and every GitHub Action pinned to a commit SHA. v0.0.2 (2026-09-27) fixed the checksum file's line endings.
- **Blocked on:** Nothing.
- **Next action:** the manual pass in the built exe listed in `OPEN_ITEMS.md` (recovery, export, restore, the Save/Open dialogs), plus the older items: every shortcut once, and opening a vault made by an earlier build to see it upgrade to v3.

## Verified facts

| Fact | Proved by | Verified |
| --- | --- | --- |
| v0.0.2 ships `lokey.exe` 8,139,776 B and `SHA256SUMS.txt` 76 B (LF, no CR); both download with 200 | `gh release view v0.0.2 --json assets`; `curl` on `releases/download/v0.0.2/`; `od -c`; release run 36266056743 green | 2026-09-27 |
| `sha256sum -c SHA256SUMS.txt` passes on the v0.0.2 file as downloaded, unmodified, in Git Bash | ran it: `lokey.exe: OK` | 2026-09-27 |
| v0.0.2 `lokey.exe` carries a valid attestation | `gh attestation verify` exit 0 | 2026-09-27 |
| `releases/latest/download/lokey.exe` serves the v0.0.2 exe | `curl` 200, `cmp` equal to the v0.0.2 download | 2026-09-27 |
| A v1 vault carrying a deletion check opens and is re-sealed as v2 | `v1_vault_with_a_deletion_check_opens_and_is_upgraded`, CI gate on PR #6 | 2026-09-24 |
| Removing `lokey-cli` costs the lock exactly three packages (`lokey-cli`, `rpassword`, `rtoolbox`) and moves no version | `lockfile.yml` run 35504833456, diff read | 2026-09-20 |
| The workspace without `lokey-cli` passes the whole gate, and `--no-bundle` builds one exe named `lokey.exe` | CI run 35505074478, all four jobs green; artifact downloaded and listed | 2026-09-20 |
| The release pipeline still gates on the tag matching all three version files | `release.yml` run 35506826668, step "Tag matches the version the binaries report" green | 2026-09-20 |

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
| page logic (node:test) | green | 17 tests, CI gate on PR #13; 5 new for recovery-key matching |
| Rust unit / integration | green | CI gate on PR #13; covers recovery, rotation (an old password or recovery key with an old copy reads nothing newer), export and restore, v1/v2 upgrade with an inbox, header tampering, the re-asked password counting toward the lockout |
| desktop window | never run | no automated UI test; needs a manual pass: every shortcut, setup with and without a recovery key, recover, export and restore through the Windows dialogs, the delete confirm, the 5-minute lock |
| the app opens a vault written by the old CLI | unit-tested only | a v2 vault with inbox records, built in the test from the old format, opens and upgrades; a real 0.2.0 vault has not been tried |

## Continuity file health

| File | Exists | Last updated | Current? |
| --- | --- | --- | --- |
| `TECH-STACK.md` | yes | 2026-09-27 | yes |
| `DIRECTORY-STRUCTURE.md` | yes | 2026-09-27 | yes |
| `DECISIONS.md` | yes | 2026-09-27 | yes; earlier decisions are on the Notion page |
| `DRIFT.md` | no | | no known drift |
| `OPEN_ITEMS.md` | yes | 2026-09-27 | yes |

## Session handoff

- **Do not touch:** `F:/projects/mini-projects/lokey-lcmdjsfbbr` and `env-protect-lcmdjsfbbr` are stale copies; delete them, never edit them.
- **Half-applied changes:** none.
- **Open questions:** whether the app window behaves correctly on a real desktop (the maintainer answers by using it); whether the sealed inbox should leave the format now that nothing writes it (`OPEN_ITEMS.md`).

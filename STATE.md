# STATE.md — lokey

**Updated:** 2026-09-24 · **Branch:** `feat/master-password-only` · **Last release:** none live; `v0.0.1` in flight (earlier releases withdrawn)

## Bootstrap record

| Field | Value |
| --- | --- |
| Bootstrapped | 2026-09-19 (moved from `F:/projects/mini-projects/lokey-lcmdjsfbbr`) |
| Stack detected as | Rust 2024 workspace (lokey-core, lokey-cli, src-tauri) + Vite/TypeScript page, from Cargo.lock and package-lock.json. `lokey-cli` removed 2026-09-20 |
| Quality gate command | `npm run check` (CI: `npm run check:ci`) |
| Deployed at | not deployed; released from tags at https://github.com/akshit-bansal11/lokey/releases |

## Where the work is right now

- **In flight:** v0.0.1, the clean-slate first release. The deletion password is removed (vault format v2; v1 vaults open and upgrade on first write), the idle lock is 5 minutes, and every action has a keyboard shortcut. Releases v0.1.0 to v1.0.0 and their tags are to be deleted once v0.0.1 is published, by the maintainer's decision.
- **Blocked on:** Nothing.
- **Next action:** in the real window, press every shortcut once (whether WebView2 lets the page have `Ctrl+N`, `Ctrl+F` and `F1` is unverified), and open a vault made by an earlier build to see it upgrade.

## Verified facts

| Fact | Proved by | Verified |
| --- | --- | --- |
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
| page logic (node:test) | green locally | 12 tests, 2026-09-24; CI pending on this branch |
| Rust unit / integration | pending | CI on this branch; deletion-password tests replaced by a no-password delete test and a v1-upgrade test; the 9 end-to-end CLI tests went with the crate |
| desktop window | never run | no automated UI test; needs a manual pass, now including every keyboard shortcut |
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

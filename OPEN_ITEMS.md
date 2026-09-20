# OPEN_ITEMS.md — lokey

Deferred, accepted or unprovisioned work. An entry is **deleted** when it is
done, never struck through.

| Item | Why it is open | What closing it means | Opened |
| --- | --- | --- | --- |
| The sealed inbox has no writer | It existed so `lokey set` could add a value without the master password. The command-line tool was removed on 2026-09-20; nothing ships that writes an inbox record any more. The read path, `Store::set`/`set_many`/`pending` and the `hpke` dependency all remain. | Either drop the inbox from the format (a vault-format change: bump `VERSION` in `format.rs`, keep reading the old one, remove `hpke`) or give the app a reason to write it. Needs a product decision first. | 2026-09-20 |
| Removing the command-line tool is a breaking change with no version yet | The change is staged under `[Unreleased]` in `CHANGELOG.md`. | Tag `v0.3.0` after bumping `Cargo.toml`, `package.json` and `tauri.conf.json` together; `release.yml` refuses a tag that disagrees with them. | 2026-09-20 |
| Accepted RustSec advisories expire 2026-12-19 | `.cargo/audit.toml` ignores them with reasons; `cargo audit --deny warnings` fails once they expire. | Upgrade the affected crates, or re-accept with a new expiry and a written reason. | 2026-09-19 |

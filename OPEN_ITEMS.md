# OPEN_ITEMS.md — lokey

Deferred, accepted or unprovisioned work. An entry is **deleted** when it is
done, never struck through.

| Item | Why it is open | What closing it means | Opened |
| --- | --- | --- | --- |
| `hpke` stays only to upgrade old vaults | Format v3 (0.1.0) dropped the sealed inbox, and nothing writes one. `crypto::inbox_open` and `merge_inbox` remain so a v1/v2 vault whose inbox still holds records keeps them when it is first unlocked and upgraded. | Remove `hpke`, `inbox_open` and `merge_inbox` once v1/v2 vaults need no longer open with their inbox, and say so in the changelog. A maintainer decision on how long to support them. | 2026-09-27 |
| Release builds are not code-signed | SmartScreen warns on first run, and Windows cannot show a verified publisher. The checksum and build attestation only help people who run them. | Buy an Authenticode certificate (or use a signing service), sign `lokey.exe` in `release.yml`, and verify the signature on a downloaded release. Costs money every year; the maintainer decides. | 2026-09-27 |
| The recovery, export and restore screens have never run in a real window | CI builds the app and tests the engine; nothing drives the page. Unverified: the Windows Save/Open dialogs opened from `file_dialog.rs`, the recovery-key dialog refusing Escape, and each new form end to end. | A manual pass in the built exe: create with and without a recovery key, recover, change the password (new key shown), export to a folder, restore it on a machine or `LOKEY_DIR` with no vault. | 2026-09-27 |
| The password generator dialog has never run in a real window | Built without a Tauri window to drive it, and the browser check could not run (the Chrome extension was not connected). Unverified: its layout at the smallest window size, the slider by mouse and arrow keys, a typed length out of range snapping back, the last checkbox staying on, Copy clearing after 30 s and staying out of Win+V, Ctrl+C on the selected password going through the same copy, **Use as new key** filling the add row, Ctrl+G. | A manual pass in the built exe covering each of those. | 2026-09-27 |
| Accepted RustSec advisories expire 2026-12-19 | `.cargo/audit.toml` ignores them with reasons; `cargo audit --deny warnings` fails once they expire. | Upgrade the affected crates, or re-accept with a new expiry and a written reason. | 2026-09-19 |

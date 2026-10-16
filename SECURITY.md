# Security

Saltcellar holds the secrets you would least like to lose or leak, so its boundaries matter. This is what it protects against, what it does not, and how to report a hole. Everything here is taken from the code in `crates/saltcellar-core` and `src-tauri`; where the code and this file disagree, the code is right and this file is a bug.

## What it protects against

- **Someone who copies the vault file.** `vault.lokey` holds your entries encrypted with AES-256-GCM under a random 256-bit data key. The data key is stored only sealed: under a key derived from the master password, and, if you made one, under a key derived from the recovery key. Both derivations are Argon2id at 64 MiB, 3 passes, 1 lane, so every guess at the password costs that much memory and time. The cost settings are stored in the file and checked on open: a file asking for less than 19 MiB of memory (a downgraded file), or for more than 1 GiB (a file made to exhaust memory), is refused.
- **Someone who edits the vault file.** Every header field an attacker could weaken, among them the salt and the cost settings, is bound into the associated data of the sealed key. An edited file fails to open; it never opens weakened or reads wrong.
- **Old passwords and old recovery keys.** Changing the master password, or making, replacing or removing a recovery key, replaces the data key and re-encrypts everything. An old password or recovery key opens only copies of the file made before the change.
- **Guessing at the prompt.** Wrong passwords and recovery keys wait 1 s, doubling to 30 s, and 10 in a row lock the vault for 15 minutes. The count is kept in `lockout.json` beside the vault, so every window shares it.
- **Weak master passwords, partly.** A new master password must be 15 to 1024 characters and must not appear in the leaked-password list built into the exe (derived from SecLists).
- **Someone at an unlocked PC, partly.** Exporting a backup, changing the master password and making or removing a recovery key each ask for the master password again. The vault locks after 5 minutes without activity, on Ctrl+L, and whenever the app closes.
- **The clipboard.** A copied value is marked so Windows keeps it out of clipboard history (Win+V), cloud clipboard and clipboard monitors, and is cleared after 30 seconds unless you copied something else meanwhile. If it is still on the clipboard when the app exits, it is cleared then.
- **Screenshots and screen sharing.** The window asks Windows to keep its contents out of captures, recordings and shared screens.
- **The network.** There is no network code. The window's content security policy allows only the app's own files and the channel to its Rust side; no request can leave it.
- **Leaving secrets in memory longer than needed.** Keys, the decrypted entries and generated passwords are wiped (`zeroize`) when they are dropped.

## What it does not protect against

- **Malware already running as you.** A program running under your account can record the password as you type it, read the screen or read the process's memory, as it could for any password manager.
- **A weak or reused master password.** Anyone with a copy of the file can guess offline for as long as they like; only the password's strength and Argon2id's cost stand in the way. The length rule and the leaked list stop the worst choices, not every bad one. The lockout does not help here: it lives beside the vault, and someone holding a copy of the file skips it.
- **Someone who finds your recovery key.** It opens the vault as fully as the password does. Keep it on paper, away from the PC, and replace it in Settings if it may have been seen.
- **Someone at your PC while the vault is unlocked.** They can read, copy and delete entries until it locks.
- **Old copies.** A backup, or `vault.lokey.bak`, opens with the password and recovery key it had when it was made. Delete old backups after changing either.
- **Someone who can replace your vault file.** Saltcellar cannot tell an older copy of its own file from the current one. Anyone able to swap an old copy back in, who also holds a recovery key you have since replaced, could get that key working again for whatever you save afterwards. That takes write access to your files, which is the malware case above.
- **SSDs keeping old blocks** after a file is rewritten. Full-disk encryption (BitLocker) covers that, and the stolen-laptop case.
- **A lost disk without a backup.** Nothing syncs anywhere, by design. Only a backup you exported brings the vault back.

## Known limits

- **No independent audit.** The primitives come from RustCrypto's `aes-gcm` and `argon2` crates; how Saltcellar combines them has been reviewed only by its own maintainers.
- **Not code-signed.** SmartScreen warns on first run. Check the download against `SHA256SUMS.txt` and with `gh attestation verify Saltcellar-<version>.exe --repo akshit-bansal11/saltcellar`, which proves the file was built by this repository's release workflow.
- **The lokey folder.** Upgrading from lokey copies the vault into `%LOCALAPPDATA%\saltcellar` and leaves `%LOCALAPPDATA%\lokey` untouched. Until you delete it, the old copy is a second file that opens with the password and recovery key it had.

## Reporting

Open a [security advisory](https://github.com/akshit-bansal11/saltcellar/security/advisories/new) on GitHub; it reaches the maintainer privately. Please do not open a public issue for something exploitable.

<p align="center"><img src="assets/logo.svg" width="96" alt="Saltcellar"></p>

# Saltcellar

Saltcellar keeps your passwords, environment keys and backup codes in one encrypted file on your own computer: Windows, macOS or Linux. One window, one file, no network.

It is for the secrets that should never leave the machine they are used on: the `DATABASE_URL` for a side project, an API token, the backup codes a site gave you once. The alternatives are a text file anyone can read, or a password manager that wants an account and a cloud to keep your vault in. Saltcellar is neither. There is no account, no sync and no server: the vault is a file on your disk, encrypted under a master password only you know. The cost is the cloud's own benefit: nothing syncs to your phone, and if the disk dies, only a backup you made brings the vault back.

Saltcellar is a key → value sheet per project, for passwords, environment keys and backup codes. It has no username or URL fields, no browser autofill and no sync. If you want those on your own disk, KeePassXC does them; it is a full password manager.

It was called **lokey** until 1.0.0. An existing lokey vault is copied over on the first start; see [Upgrading from lokey](#upgrading-from-lokey).

[![CI](https://github.com/akshit-bansal11/saltcellar/actions/workflows/ci.yml/badge.svg)](https://github.com/akshit-bansal11/saltcellar/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/akshit-bansal11/saltcellar)](https://github.com/akshit-bansal11/saltcellar/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Documentation](https://img.shields.io/badge/docs-saltcellar.akshitbansal.dev-black.svg)](https://saltcellar.akshitbansal.dev/docs)

## What it does

- **No network requests.** The app makes none, and its content security policy blocks any. The system web view underneath (WebView2 on Windows, WebKit on macOS and Linux) is updated by the operating system, not by Saltcellar.
- **Encrypted at rest.** Entries are sealed with AES-256-GCM under a random data key, which is itself sealed under a key derived from your master password with Argon2id (64 MiB, 3 passes).
- **Projects and keys.** One project at a time as a sheet of keys and values; `DATABASE_URL` can exist once in `web` and once in `api`.
- **A password generator** (the dice button, or Ctrl+G), drawing from the operating system's random number generator.
- **Careful with the clipboard.** Copied values, even ones copied by selecting the text, are cleared after 30 seconds. On Windows they stay out of Win+V history and cloud clipboard; on macOS and Linux they are marked for clipboard managers to skip. Details per system are in [SECURITY.md](SECURITY.md).
- **Hidden from screenshots and screen sharing** on Windows, and on macOS where the system honours it, and locked after 5 minutes idle and whenever it closes.
- **Verifiable builds.** Every release file has a SHA-256 checksum and a signed build attestation tying it to the commit and workflow that built it.

## Install

Saltcellar comes from the [latest release](https://github.com/akshit-bansal11/saltcellar/releases/latest), under **Assets**. Each file is named for its release: `Saltcellar-1.1.0.exe` is 1.1.0. Download the file for your system and `SHA256SUMS.txt` from the same release, then check the download, as below. No build is code-signed, so each system warns on the first run.

The macOS and Linux builds are new in 1.1.0 and have not yet been tried on real machines beyond CI. If one fails for you, please [open an issue](https://github.com/akshit-bansal11/saltcellar/issues/new).

### Windows

Windows 10 or 11, 64-bit, with WebView2 (built into Windows 11, and into any Windows 10 kept up to date).

Download `Saltcellar-<version>.exe`. It is the whole install: one portable exe, no setup program, no administrator rights, nothing written outside your own user folder. Put it wherever you keep your tools. Run it. SmartScreen asks first: choose **More info**, then **Run anyway**.

### macOS

Download `Saltcellar-<version>-macos-universal.dmg`. One file serves Apple silicon and Intel. Open it and drag Saltcellar into Applications. The app is ad-hoc signed but not notarized, so macOS refuses to open it the first time. Open **System Settings → Privacy & Security** and choose **Open Anyway**.

### Linux

x86_64 only, with WebKitGTK 4.1. Pick one:

- `Saltcellar-<version>-linux-x86_64.AppImage`: make it executable with `chmod +x`, then run it.
- `Saltcellar-<version>-linux-amd64.deb`: install it with `sudo apt install ./Saltcellar-<version>-linux-amd64.deb`. It pulls in WebKitGTK 4.1.

### Check the download

The checksum proves the file is the one the release published. In PowerShell, in the folder you downloaded to:

```powershell
(Get-FileHash .\Saltcellar-<version>.exe -Algorithm SHA256).Hash.ToLower()
Get-Content .\SHA256SUMS.txt
```

The two hashes must be identical. On a Mac, in the folder you downloaded to, this does the comparison and prints `OK`:

```text
shasum -a 256 -c SHA256SUMS.txt --ignore-missing
```

On Linux:

```text
sha256sum -c --ignore-missing SHA256SUMS.txt
```

The attestation proves the file was built by this repository's release workflow from a tagged commit. With the [GitHub CLI](https://cli.github.com), for any of the files:

```text
gh attestation verify <file> --repo akshit-bansal11/saltcellar
```

## Uninstall

Delete the app: the exe, the app from Applications, or the AppImage (or `sudo apt remove saltcellar`). Only if you want the vault gone too, delete its folder, listed under [Where the vault lives](#where-the-vault-lives). The web view also keeps a small cache in a folder named `dev.akshitbansal.saltcellar` (on Windows under `%LOCALAPPDATA%`), which can go too.

## Upgrading from lokey

lokey kept its vault in `%LOCALAPPDATA%\lokey`; Saltcellar keeps it in `%LOCALAPPDATA%\saltcellar`. On its first start, Saltcellar copies your vault across:

1. **Close lokey first**, every window of it.
2. Start Saltcellar. If `%LOCALAPPDATA%\lokey` holds a vault and `%LOCALAPPDATA%\saltcellar` does not, Saltcellar copies `vault.lokey`, `vault.lokey.bak` and `lockout.json` across, then asks for your master password as before. The status bar says the vault was copied, and `%LOCALAPPDATA%\saltcellar\migration.log` records it.
3. Once Saltcellar has opened your vault and you have checked your entries, delete `%LOCALAPPDATA%\lokey` and `lokey.exe` yourself. Saltcellar never does.

What the copy does and does not do:

- It **copies, never moves.** The lokey folder is only read: nothing in it is changed, moved or deleted, so lokey still opens it if anything goes wrong.
- It happens **once**. If `%LOCALAPPDATA%\saltcellar` already holds a vault, nothing is copied and both are left alone.
- A vault you delete after the copy is never copied back from `%LOCALAPPDATA%\lokey`: a `migrated-from-lokey` marker beside the vault records that the copy was done.
- It is **skipped** when `SALTCELLAR_DIR` or `LOKEY_DIR` is set: Saltcellar opens the folder you chose.
- If the copy fails, Saltcellar shows the reason instead of offering to create a new vault, writes it to `migration.log`, and tries again on the next start.
- The vault format is unchanged. Vault and backup files keep their `.lokey` names, and backups made by lokey restore in Saltcellar.

Changes you make in lokey after the copy stay in lokey's folder. Use one or the other.

## First use

The first run offers to create the vault. You choose one **master password**. It must be at least 15 characters, and Saltcellar refuses passwords that appear in public lists of leaked passwords. A few random words make a strong password that is easy to remember. Once it is entered, everything is open: reading, adding, editing, copying and deleting. Deleting asks you to confirm.

Then you choose what happens if you forget it:

- **A recovery key (recommended).** Saltcellar shows a 32-character key once, and asks you to type it back to prove you saved it. Write it on paper and keep it away from the PC. If you forget the master password, the recovery key opens the vault and you choose a new password; you then get a new recovery key and the used one stops working.
- **No recovery.** Only the master password opens the vault. Forget it and everything in it is gone, for you and for anyone else.

There are no password hints and no security questions. Either would become the easiest way into the vault, so Saltcellar offers neither. You can turn the recovery key on, replace it or remove it later in Settings.

## Use

The app shows one project at a time as a sheet with three columns: number, key and value. Values stay hidden until you show one, and only one is shown at a time.

- Type a name and value in the last row and press Enter to add a key.
- Show, copy, edit and delete are on each row. Copying clears the clipboard after 30 seconds, and a thin line in the status bar counts it down.
- **Generate a password** (the dice button, or Ctrl+G): pick a length from 8 to 128 and which characters to use (uppercase, lowercase, digits, symbols). Copy it, or choose **Use as new key** to put it in the last row's value, then name the key and press Enter.
- Everything works from the keyboard. On a Mac, use Cmd wherever this says Ctrl. Press **F1** (or `?`) for the full list. Ctrl+K opens the command palette (every command, and "Switch to project X"). Ctrl+N adds a key, Ctrl+Shift+N makes a project, Ctrl+1 to Ctrl+9 jump to a project, Ctrl+PgUp/PgDn or Alt+Up/Down switch projects, Ctrl+F or `/` searches, Ctrl+G opens the generator, Ctrl+, opens settings, Ctrl+L locks; in the rows, Up/Down/Home/End move, Enter shows, Ctrl+C copies the value, Ctrl+Shift+C copies the key's name, F2 edits, Del deletes; in the add row, Ctrl+Enter (or Enter) adds the key.
- **Projects** keep the same key name apart. New keys go to `default` unless you pick another.
- **Names** use letters, digits, spaces and `_ - . / @ + :`, with no space at either end, and match regardless of case.
- The app locks itself after 5 minutes without use, and whenever it is closed: the next start asks for the master password again.

### Backups

**Settings → Export backup…** asks for your master password again, then where to save. The backup is the vault file itself, still encrypted: it opens with the master password, or the recovery key, the vault had when you exported it. Keep one on a USB drive or another disk; the vault on your PC is a single file, and a dead disk takes it with it. A folder synced by OneDrive or a similar service uploads whatever you save there.

To move to a new PC, or after losing the disk, start Saltcellar with no vault and choose **Restore from a backup…**. Saltcellar never restores over an existing vault.

### Where the vault lives

The folder is `saltcellar` under your user data folder:

- Windows: `%LOCALAPPDATA%\saltcellar`
- macOS: `~/Library/Application Support/saltcellar`
- Linux: `$XDG_DATA_HOME/saltcellar`, which is `~/.local/share/saltcellar` by default

In it is `vault.lokey`, a small `lockout.json` beside it and `vault.lokey.bak`, the version before the last change. Set `SALTCELLAR_DIR` (or the older `LOKEY_DIR`) to keep them somewhere else. On macOS and Linux the files are created readable by you only (0600), and a folder Saltcellar creates is yours only (0700).

Two Saltcellar windows can be open at once: every change takes a lock on the file, reads it fresh, applies one change and writes it back whole, so the two never overwrite each other.

## How it works

- Your entries are encrypted with **AES-256-GCM** under a random 256-bit **data key**.
- The data key is stored at most twice, each copy encrypted: once under a key derived from your master password, and once under a key derived from the recovery key if you have one. Both derivations use **Argon2id** (64 MiB, 3 passes, 1 lane), which makes every guess against a stolen file slow and costly.
- Changing the master password, or making or removing a recovery key, replaces the data key. An old password or recovery key opens only copies of the file made before the change.
- Every header field an attacker could weaken (the salt, the cost settings) is bound into the encryption, so a changed file fails to open rather than opening weakened or wrong.
- Wrong passwords and recovery keys slow down (1 s, 2 s, 4 s … up to 30 s) and lock for 15 minutes after 10 in a row.
- Exporting a backup, changing the master password and making or removing a recovery key each ask for the master password again, even with the vault open.

What Saltcellar cannot protect you from, and how to report a problem, is in [SECURITY.md](SECURITY.md). **No independent security audit** has been done yet. The encryption comes from widely used Rust libraries (RustCrypto's `aes-gcm` and `argon2`); how Saltcellar combines them has been reviewed only by its own maintainers. Everything that handles your data is in `crates/saltcellar-core` and `src-tauri`.

## Develop

You need Rust (stable) and Node.js 24 or newer. On Linux, also the WebKitGTK 4.1 and GTK development packages Tauri lists.

```powershell
npm ci
npm run check                 # format, lint, type-check, all tests, build the page
npx tauri build --no-bundle   # target/release/saltcellar.exe
```

`npm run check:ci` runs the same checks without changing any file; CI runs that.

## Contributing

- **A bug, or something Saltcellar should do:** [open an issue](https://github.com/akshit-bansal11/saltcellar/issues/new). Open one before starting anything bigger than a fix, so the design can be talked through before code exists.
- **A security hole:** report it privately, as [SECURITY.md](SECURITY.md) describes, not in a public issue.
- **A change:** see [CONTRIBUTING.md](CONTRIBUTING.md) for the gate, tests, commit messages and the rules that are not up for debate (no network code).

Releases are built and published by the maintainer; [CHANGELOG.md](CHANGELOG.md) lists what each one changed.

## License

[MIT](LICENSE).

## Credits

- Icons are from [Lucide](https://lucide.dev) (ISC), inlined into the page.
- The leaked-password list is derived from [SecLists](https://github.com/danielmiessler/SecLists) (MIT).

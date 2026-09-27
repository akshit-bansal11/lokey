# lokey

**lo**cal **key**: an encrypted vault for your passwords, environment keys and
backup codes, on your own Windows PC. One window, one file.

- **No network code, ever.** No account, no sync, no telemetry, no update
  check. The app's content security policy forbids every connection.
- **Open source.** Everything that touches your data is in this repository.
- **Verifiable builds.** Every release file has a SHA-256 checksum and a signed
  build attestation tying it to the commit and workflow that built it.
- **Local only.** Your vault is one encrypted file on your disk.

## Install

Download `lokey.exe` from
[Releases](https://github.com/akshit-bansal11/lokey/releases/latest) and run
it. That is the whole install: one portable exe, no setup program, no
administrator rights, nothing written outside your own user folder. Put it
wherever you keep your tools; to remove lokey, delete it.

> **Windows SmartScreen** may warn that the app is from an unknown publisher:
> lokey's builds are not code-signed yet. Check the file you downloaded instead:
>
> ```text
> sha256sum -c SHA256SUMS.txt
> gh attestation verify lokey.exe --repo akshit-bansal11/lokey
> ```

## First use

The first run offers to create the vault. You choose one **master password**.
It must be at least 15 characters, and lokey refuses passwords that appear in
public lists of leaked passwords. A few random words make a strong password
that is easy to remember. Once it is entered, everything is open: reading,
adding, editing, copying and deleting. Deleting asks you to confirm.

Then you choose what happens if you forget it:

- **A recovery key (recommended).** lokey shows a 32-character key once, and
  asks you to type it back to prove you saved it. Write it on paper and keep it
  away from the PC. If you forget the master password, the recovery key opens
  the vault and you choose a new password; you then get a new recovery key and
  the used one stops working.
- **No recovery.** Only the master password opens the vault. Forget it and
  everything in it is gone, for you and for anyone else.

There are no password hints and no security questions. Either would become the
easiest way into the vault, so lokey offers neither.

You can turn the recovery key on, replace it or remove it later in Settings.

## Using it

The app shows one project at a time as a sheet with three columns: number,
key and value. Values stay hidden until you show one, and only one is shown at
a time.

- Type a name and value in the last row and press Enter to add a key.
- Show, copy, edit and delete are on each row. Copying clears the clipboard
  after 30 seconds, and a thin line in the status bar counts it down.
- **Generate a password** (the dice button, or Ctrl+G): pick a length from 8
  to 128 and which characters to use (uppercase, lowercase, digits, symbols).
  Copy it, or choose **Use as new key** to put it in the last row's value,
  then name the key and press Enter. Generated passwords are copied the same
  way as saved ones: kept out of Win+V history and cleared after 30 seconds.
- Everything works from the keyboard. Press **F1** (or `?`) for the full
  list: Ctrl+N adds a key, Ctrl+Shift+N makes a project, Ctrl+G generates a
  password, Ctrl+PgUp/PgDn switch projects, Ctrl+F searches, Ctrl+, opens settings, Ctrl+L locks; in the rows,
  Up/Down/Home/End move, Enter shows, Ctrl+C copies, F2 edits, Del deletes.
- **Projects** keep the same key name apart: `DATABASE_URL` can exist once in
  `web` and once in `api`. New keys go to `default` unless you pick another.
- **Names** use letters, digits and `_ - . /`, and match regardless of case.
- The app locks itself after 5 minutes without use, and whenever it is
  closed: the next start asks for the master password again.
- The window is hidden from screenshots and screen sharing, so a value on
  screen does not leak into a recording or a video call.

## Backups

**Settings → Export backup…** asks for your master password again, then where
to save. The backup is the vault file itself, still encrypted: it opens with
the master password, or the recovery key, the vault had when you exported it.
Keep one on a USB drive or another disk; the vault on your PC is a single file,
and a dead disk takes it with it. A folder synced by OneDrive or a similar
service uploads whatever you save there.

To move to a new PC, or after losing the disk, start lokey with no vault and
choose **Restore from a backup…**. lokey never restores over an existing vault.

## Where the vault lives

`%LOCALAPPDATA%\lokey\vault.lokey`, with a small `lockout.json` beside it and
`vault.lokey.bak`, the version before the last change. Set `LOKEY_DIR` to keep
them somewhere else. Local, not roaming: Windows never syncs the folder.

Two lokey windows can be open at once: every change takes a lock on the file,
reads it fresh, applies one change and writes it back whole, so the two never
overwrite each other.

## Security

### How the vault is protected

- Your entries are encrypted with **AES-256-GCM** under a random 256-bit
  **data key**.
- The data key is stored twice at most, each copy encrypted: once under a key
  derived from your master password, and once under a key derived from the
  recovery key if you have one. Both derivations use **Argon2id** (64 MiB,
  3 passes), which makes every guess against a stolen file slow and costly.
- Changing the master password, or making or removing a recovery key, replaces
  the data key. An old password or recovery key opens only copies of the file
  made before the change.
- Any change to the file (its encrypted parts, the salt, the cost settings)
  makes it fail to open rather than open weakened or wrong.
- Wrong passwords and recovery keys slow down (1s, 2s, 4s ... up to 30s) and
  lock for 15 minutes after 10 in a row.
- Exporting a backup, changing the master password and making or removing a
  recovery key each ask for the master password again, even with the vault
  open. Someone at your unlocked PC cannot quietly take a copy or add a way in.
- Copied values stay out of Windows clipboard history (Win+V) and cloud
  clipboard, and are cleared after 30 seconds unless you copied something
  else.

### What lokey cannot protect you from

- **Malware already running as you.** Such a program can record your password
  as you type it, or read the screen, as it could for any password manager.
  lokey cannot defend a PC that is already compromised.
- **A weak or reused master password.** Anyone with a copy of the file can
  guess offline for as long as they like; only the password's strength and
  Argon2id's cost stand in the way. The length rule and the leaked-password
  list stop the worst choices, not all bad ones.
- **Someone who finds your recovery key.** It opens the vault as well as the
  password does. Keep it on paper, away from the PC, and make a new one in
  Settings if it may have been seen.
- **Someone at your PC while the vault is unlocked** can read and delete
  entries. The 5-minute idle lock and Ctrl+L are the protection.
- **Old copies.** A backup, or `vault.lokey.bak`, opens with the password and
  recovery key it had. Delete old backups after changing either. Restoring a
  backup gives it a new data key and recovery key, so a key you replaced after
  making the backup does not come back with it.
- **Someone who can replace your vault file.** lokey cannot tell an older copy
  of its own file from the current one. Anyone able to swap an old copy back
  in, who also holds a recovery key you have since replaced, could get that key
  working again for whatever you save afterwards. That takes write access to
  your files, which is the malware case above.
- **SSDs can keep old blocks** after a file is rewritten. Full-disk encryption
  (BitLocker) covers that, and the stolen-laptop case.

### What you can check yourself

- The source: every file that handles your data is in `crates/lokey-core` and
  `src-tauri`.
- The build: releases are built by GitHub Actions from a tagged commit that
  passed every check, with each third-party action pinned to an exact commit.
  Each release carries a checksum file and a signed build attestation.
- What has not happened: **no independent security audit** of lokey has been
  done yet. The encryption comes from widely used Rust libraries (RustCrypto's
  `aes-gcm` and `argon2`); how lokey combines them has been reviewed only by its
  own maintainers.

**Windows only**, for now.

## Build from source

You need Rust (stable) and Node.js 24 or newer.

```powershell
npm ci
npm run check                 # format, lint, type-check, all tests, build the page
npx tauri build --no-bundle   # target/release/lokey.exe
```

`npm run check:ci` runs the same checks without changing any file; CI runs that.

## Licence

MIT. See [LICENSE](LICENSE). Icons are from [Lucide](https://lucide.dev) (ISC).
The leaked-password list is derived from
[SecLists](https://github.com/danielmiessler/SecLists) (MIT).

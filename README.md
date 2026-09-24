# lokey

**lo**cal **key**: an encrypted vault for your environment keys, passwords and
backup codes, on your own Windows PC. One window, one file.

lokey has no network code at all. No account, no sync, no telemetry, no update
check. Your vault is one file on your disk.

## Install

Download `lokey.exe` from
[Releases](https://github.com/akshit-bansal11/lokey/releases/latest) and run
it. That is the whole install: one portable exe, no setup program, no
administrator rights, nothing written outside your own user folder. Put it
wherever you keep your tools; to remove lokey, delete it.

> **Windows SmartScreen** may warn that the app is from an unknown publisher.
> lokey's builds are not code-signed, because a signing certificate costs money
> every year. Every release file has a SHA-256 checksum in `SHA256SUMS.txt` and
> a signed build attestation you can verify:
> `gh attestation verify lokey.exe --repo akshit-bansal11/lokey`

## First use

The first run offers to create the vault. You choose one **master password**.
Once it is entered, everything is open: reading, adding, editing, copying and
deleting. Deleting asks you to confirm, naming what goes.

It must be at least 15 characters, and lokey refuses passwords that appear in
public lists of leaked passwords. A few random words make a strong password
that is easy to remember.

**There is no recovery.** If you forget the master password, nobody can open
the vault, including you.

## Using it

The app shows one project at a time as a sheet with three columns: number,
key and value. Values stay hidden until you show one, and only one is shown at
a time.

- Type a name and value in the last row and press Enter to add a key.
- Show, copy, edit and delete are on each row. Copying clears the clipboard
  after 30 seconds, and a thin line in the status bar counts it down.
- Everything works from the keyboard. Press **F1** (or `?`) for the full
  list: Ctrl+N adds a key, Ctrl+Shift+N makes a project, Ctrl+PgUp/PgDn switch
  projects, Ctrl+F searches, Ctrl+, opens settings, Ctrl+L locks; in the rows,
  Up/Down/Home/End move, Enter shows, Ctrl+C copies, F2 edits, Del deletes.
- **Projects** keep the same key name apart: `DATABASE_URL` can exist once in
  `web` and once in `api`. New keys go to `default` unless you pick another.
- **Names** use letters, digits and `_ - . /`, and match regardless of case.
- The app locks itself after 5 minutes without use, and whenever it is
  closed: the next start asks for the master password again.
- The window is hidden from screenshots and screen sharing, so a value on
  screen does not leak into a recording or a video call.

## Where the vault lives

`%LOCALAPPDATA%\lokey\vault.lokey`, with a small `lockout.json` beside it. Set
`LOKEY_DIR` to keep both somewhere else.

Copy that file to back it up; there is nowhere else to look. Two lokey windows
can be open at once: every change takes a lock on the file, reads it fresh,
applies one change and writes it back whole, so the two never overwrite each
other. An open window checks the file four times a second and redraws when it
changes.

## Security

What lokey does:

- Encrypts the vault with **AES-256-GCM** under a key derived from your master
  password with **Argon2id** (64 MiB, 3 passes). That derivation is deliberately
  slow, which is what makes guessing passwords against a stolen file expensive.
- Detects tampering: any change to the encrypted data makes it fail to open,
  rather than returning something wrong.
- Slows down wrong passwords (1s, 2s, 4s ... up to 30s) and locks for 15
  minutes after 10 in a row.
- Keeps copied values out of Windows clipboard history (Win+V) and cloud
  clipboard, and clears them after 30 seconds unless you copied something else.

What lokey does not do, deliberately or because it cannot:

- **It cannot protect you from malware already running as you.** Such a
  program can record your password as you type it, like it could for any
  password manager. Use full-disk encryption (BitLocker) for the stolen-laptop
  case; lokey covers the copied-file case.
- **Anything running as you can add a value without your password.** The vault
  carries a public key in its readable header, and a value sealed to it
  (**HPKE**, RFC 9180, X25519) is merged the next time you unlock. Only the
  private key, locked behind the master password, can open the vault, so such a
  value still cannot be read back — and the next unlock lists what was added or
  replaced since you last looked. lokey itself no longer writes there: the
  command-line tool that did is gone, and the merge remains so that a vault it
  wrote still opens cleanly.
- **Someone who can edit the vault file could swap its public key**, so values
  sealed to it afterwards would be readable to them. lokey keeps a second copy
  of that key inside the encrypted part, checks it on every unlock, puts it back
  and warns you if it was changed.
- **Anyone at your unlocked PC can delete keys.** There is no second
  password for deleting; the idle lock and Ctrl+L are the protection.
- **An old copy of the vault file still opens with the password it had.**
  After changing a password, delete any backups you made of the old file.
- **Deleting overwrites the data, but SSDs can keep old blocks.** Full-disk
  encryption is the real guarantee.
- **Windows only**, for now.

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

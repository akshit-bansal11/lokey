# lokey

**lo**cal **key**: an encrypted vault for your environment keys, passwords and
backup codes, on your own Windows PC. There is a desktop app and a command-line
tool, and they share one vault, so a key you set in the terminal shows up in
the app a moment later.

lokey has no network code at all. No account, no sync, no telemetry, no update
check. Your vault is one file on your disk.

## Pick one, or use both

| | Desktop app | Command line |
| --- | --- | --- |
| Best for | Browsing and editing keys in a table | Saving and fetching keys while you work |
| Install | Download `lokey_x.y.z_x64-setup.exe` from [Releases](https://github.com/akshit-bansal11/lokey/releases/latest) | One line in PowerShell (below) |
| Portable | `lokey-app.exe` from Releases runs without installing | `lokey.exe` from Releases, anywhere on your PATH |

### Install the command-line tool

Paste this into PowerShell (Windows PowerShell 5.1 or PowerShell 7):

```powershell
irm https://github.com/akshit-bansal11/lokey/releases/latest/download/install.ps1 | iex
```

It downloads `lokey.exe`, checks it against the release's `SHA256SUMS.txt`,
puts it in `%LOCALAPPDATA%\Programs\lokey` and adds that folder to your user
PATH. No administrator rights. Open a new terminal afterwards.

To remove it, delete that folder and remove it from your user PATH.

> **Windows SmartScreen** may warn that the app or installer is from an
> unknown publisher. lokey's builds are not code-signed, because a signing
> certificate costs money every year. Every release file has a SHA-256 checksum
> and a signed build attestation you can verify:
> `gh attestation verify lokey.exe --repo akshit-bansal11/lokey`

## First use

Create the vault once, from either the app or `lokey init`. You choose two
passwords:

| Password | What it does |
| --- | --- |
| **Master** | Opens the vault. Needed to read, list, edit or copy anything. |
| **Deletion** | Needed, together with the master password, to delete keys. |

Each must be at least 15 characters, and lokey refuses passwords that appear in
public lists of leaked passwords. A few random words make a strong password
that is easy to remember.

**There is no recovery.** If you forget the master password, nobody can open
the vault, including you.

## The command-line tool

```text
lokey set key=NAME value=VALUE [project=P]   save a value (no password needed)
lokey set key=NAME [project=P]               same, typing the value hidden
lokey get key=NAME [project=P]               copy it; the clipboard clears in 30s
lokey get key=NAME [project=P] --view        print it instead
lokey view [project=P]                       print every key and value
lokey projects                               list projects and key counts
lokey delete key=NAME [project=P]            delete one key
lokey delete project=P                       delete a project and all its keys
lokey truncate                               delete every key
lokey import [project=P]                     paste KEY=VALUE or key,value; lines

lokey init        create the vault
lokey passwd      change the master or deletion password
lokey status      vault location, pending values, lockout
lokey help
```

Rules worth knowing:

- **`set` and `import` need no password.** Everything else asks for the master
  password, every time. `delete` and `truncate` also ask for the deletion
  password.
- **Projects** keep the same key name apart: `DATABASE_URL` can exist once in
  `web` and once in `api`. Leave out `project=` and the key goes to `default`.
  `get` finds a key in any project when the name is unique.
- **Names** use letters, digits and `_ - . /`, and match regardless of case.
- **A value typed as `value=...` is saved in your shell history**, in plain
  text. Leave `value=` out and lokey asks for it with hidden input instead.
- **In scripts**, use `--view`, because plain `get` copies the value and prints
  nothing useful: `$env:API_KEY = lokey get key=API_KEY --view`
- **`import`** reads a `.env` file too: `Get-Content .env | lokey import project=web`.
  If any line is not a pair, nothing is imported.

## The desktop app

The app shows one project at a time as a sheet with three columns: number,
key and value. Values stay hidden until you show one, and only one is shown at
a time.

- Type a name and value in the last row and press Enter to add a key.
- Show, copy, edit and delete are on each row. Copying clears the clipboard
  after 30 seconds, and a thin line in the status bar counts it down.
- Keyboard: Up/Down move between rows, Enter shows, Ctrl+C copies, F2 edits,
  Delete deletes, `/` searches, Ctrl+L locks.
- The app locks itself after 15 minutes without use.
- The window is hidden from screenshots and screen sharing, so a value on
  screen does not leak into a recording or a video call.

## How the app and the terminal stay in sync

Both use the same file: `%LOCALAPPDATA%\lokey\vault.lokey`. Every change takes
a lock on the file, reads it fresh, applies one change and writes it back
whole, so the two never overwrite each other. An open app checks the file four
times a second and redraws when it changes. Set `LOKEY_DIR` to keep the vault
somewhere else; set it the same way for both.

## Security

What lokey does:

- Encrypts the vault with **AES-256-GCM** under a key derived from your master
  password with **Argon2id** (64 MiB, 3 passes). That derivation is deliberately
  slow, which is what makes guessing passwords against a stolen file expensive.
- Lets `set` work without a password by sealing each new value to a public key
  (**HPKE**, RFC 9180, X25519) held in the vault. Only the private key, locked
  behind the master password, can open them.
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
- **`set` needs no password, so anything running as you can add or replace a
  value.** It still cannot read one. The next time you unlock, lokey lists
  what was added or replaced since you last looked.
- **Someone who can edit the vault file could swap its public key**, so values
  you `set` afterwards would be readable to them. lokey keeps a second copy of
  that key inside the encrypted part, checks it on every unlock, puts it back
  and warns you if it was changed.
- **An old copy of the vault file still opens with the passwords it had.**
  After changing a password, delete any backups you made of the old file.
- **Deleting overwrites the data, but SSDs can keep old blocks.** Full-disk
  encryption is the real guarantee.
- **Windows only**, for now.

## Build from source

You need Rust (stable) and Node.js 24 or newer.

```powershell
npm ci
npm run check          # format, lint, type-check, all tests, build the page
npx tauri build        # the app and its installer, in target/release
cargo build --release -p lokey-cli   # lokey.exe
```

`npm run check:ci` runs the same checks without changing any file; CI runs that.

## Licence

MIT. See [LICENSE](LICENSE). Icons are from [Lucide](https://lucide.dev) (ISC).
The leaked-password list is derived from
[SecLists](https://github.com/danielmiessler/SecLists) (MIT).

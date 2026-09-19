# secure-vault

A local, text-only encrypted vault for env keys, backup codes and passwords.
Pure PowerShell 7 plus .NET's own crypto — no dependencies, no daemon, no
network code anywhere. Roughly 60 KB on disk.

Nothing ever leaves the machine. There is no sync, no telemetry, no HTTP call in
the source; the only files touched are your vault and its session/lockout state.

---

## Install

```bash
pwsh -File F:\env-protect\install.ps1
```

That adds a `secure-vault` function to your PowerShell 7 profile. Open a new
window, then:

```bash
secure-vault --init
```

`--init` asks for the three passwords and creates
`%USERPROFILE%\.secure-vault\vault.sv`. Take it back out any time with
`pwsh -File install.ps1 -Remove`.

---

## The three passwords

| Password     | What it does                                                        |
| ------------ | ------------------------------------------------------------------- |
| **master**   | Opens the file. You see entry names, types and notes — values stay ciphertext. |
| **decrypt**  | Reveals the values themselves.                                      |
| **deletion** | Destroys the entire vault via `--emergency`. Nothing else.          |

They must be different from each other and at least 10 characters. Reading any
secret needs **both** the master and decrypt passwords: value ciphertexts live
*inside* the master envelope, so holding one password alone gets you nothing
useful.

There is no recovery path. Lose the master or decrypt password and the data is
gone — that is the point.

---

## Commands

```bash
secure-vault                # master password -> prints the vault, still encrypted
secure-vault --unlock       # decrypt password -> values become readable
secure-vault --lock         # forget both cached keys right now
secure-vault --status       # state, TTLs, lockout — prompts for nothing

secure-vault --show NAME    # print one value in full
secure-vault --all          # print every value, nothing masked
secure-vault --use NAME     # reveal and burn the next unused backup code

secure-vault --add          # add an entry: kv | codes | pw
secure-vault --import       # bulk paste  key1,value1; key2,value2;
secure-vault --move NAME    # move an entry to another project
secure-vault --rm NAME      # delete an entry (type the name to confirm)

secure-vault --projects           # list projects and their entry counts
secure-vault --project NAME       # switch project (offers to create it if new)
secure-vault --rm-project NAME    # delete a project and everything in it

secure-vault --init         # create the vault
secure-vault --rotate       # fresh salts + nonces, optionally new passwords
secure-vault --emergency    # erase everything (--force skips the ERASE confirm)
secure-vault --help
```

`NAME` matches exactly, or by any unambiguous prefix — searched in the active
project first, then across the whole vault. Two matches is an error rather than
a guess.

### Entry types

All three are the same shape — **a name, a project, an optional note, and a
secret** — added the same way with `--add`, and listed, moved, deleted and
searched identically. Only the payload and how it renders differ:

- **kv** — an environment variable's value
- **codes** — one-time backup codes; `--use` reveals the next one and marks it
  spent, so you never burn the same code twice
- **pw** — a password or passphrase

`--import` is the one type-specific command: a pasted batch always becomes `kv`
entries.

### Names, moving, and why there is no edit

Names are **unique within a project**. `DATABASE_URL` may exist once in `web`
and once in `api`; a second one in `web` is refused, whether it arrives via
`--add`, `--import`, or a move.

`--move NAME` (or **M** in the list) re-files an entry into another project. It
lists the other projects and takes a number or a name. Moving needs only the
master password — the ciphertext is untouched, because a value's associated data
binds it to its entry id and type, never to its project.

There is no edit command. To replace a secret, `--rm NAME` and add it again.
That is deliberate: it keeps a name from silently pointing at a different value
than the one you last checked, and it means every write to an existing name is
an explicit deletion you confirmed by typing the name.

### Projects

Every entry belongs to exactly one project, so `DATABASE_URL` can exist in both
`web` and `api` without colliding. Listing, `--add`, `--import` and `--all` are
all scoped to the active project, which is shown in the banner.

```bash
secure-vault --project web      # switches; asks before creating a project that doesn't exist
secure-vault --projects         # >  web    5
```

Switching only needs the master password — which project you are in is metadata,
not a secret. `--rm-project` deletes the project *and every entry in it*, so it
makes you type the project name first, and it refuses to remove the last one.

A vault created before projects existed opens normally: everything in it lands
in `default` on first open, with no migration step and no format bump.

### Reading and copying

After `--unlock`, values render masked (`po••••••••••••pp`).

| Key | |
| --- | --- |
| up / down | move |
| **Enter** | show or hide the focused value |
| **M** | move this entry to another project |
| **D** | delete this entry |
| **Esc** | done |

**Only the focused row can be revealed, and moving off it hides it again.** At
most one secret is on screen at any moment, and it disappears the instant your
attention moves. Enter on an already-revealed row hides it without moving.

That is true in memory, not just on screen. **Drawing the list decrypts
nothing** — the masks are computed from ciphertext length, so a secret is
decrypted only when you press Enter on its row, and the plaintext is dropped
when you hide it or move away. Listing a project never puts values you didn't
look at into memory. The masks also show no characters of the value: leaking two
characters per row across a whole project adds up.

Once a value is on screen, select it with the mouse and copy the way you
normally would in your terminal — nothing touches the clipboard on your behalf.

When output is piped or the list is taller than the window, the selector falls
back to a static masked table and points you at `--show NAME`.

### Bulk paste

`--import` takes pairs in the format you'd paste into Vercel and adds them all
as `kv` entries in the active project:

```
"
key1,value1;
key2,value2;
key3,value3;
"
```

Paste the block at the prompt and press Enter on the empty line to finish. The
wrapping quotes, the trailing `;`, and any stray whitespace are all optional.
Only the **first** comma splits, so values may contain commas —
`URL,https://h/?a=1,2` gives `URL` → `https://h/?a=1,2`. A value may not contain
a `;`; that is the format's one real limit.

If any line is malformed, **nothing** is imported — it lists the bad lines and
stops, rather than importing half a batch and leaving you to work out which
half. Names that already exist in the project are skipped and reported, never
overwritten; to replace one deliberately, `--rm NAME` and add it again.

---

## How it is encrypted

```
vault.sv
├── header (cleartext)   format, salts, iteration count, deletion verifier
└── envelope             AES-256-GCM under masterKey
    ├── check            AES-256-GCM under dataKey  (verifies the decrypt password)
    └── entries[]
        ├── name/type/project/note/used   readable once the envelope is open
        └── val                           AES-256-GCM under dataKey, fresh nonce each
```

Project names and entry names are metadata: readable with the master password
alone, which is what lets you see *what* is in the vault before deciding to
decrypt it. Only `val` needs the decrypt password.

- **Cipher** AES-256-GCM, 96-bit random nonce per seal, 128-bit tag.
- **KDF** PBKDF2-HMAC-SHA256, 600,000 iterations, 128-bit random salt per layer.
- **Authenticated headers** the format version, KDF parameters, salts and
  deletion verifier are all bound in as GCM associated data. Editing any of them
  — say, dropping the iteration count to 1,000 — breaks the tag instead of
  weakening the vault.
- **Bound values** each value's associated data includes its entry id and type,
  so a ciphertext cannot be moved from one entry to another.
- **Tamper-evident** any single flipped bit fails authentication and reads as a
  wrong password rather than returning garbage.

Passwords go from `SecureString` straight to UTF-8 bytes through unmanaged
memory and are zeroed after use — no managed `String` copy is ever made.
Derived keys are zeroed the moment the command finishes.

### Why `--lock` and `--unlock` can be separate commands

They are separate processes, so the unlocked state has to survive between them.
Derived keys — never plaintext secrets — are cached in
`%LOCALAPPDATA%\secure-vault\session.bin`, wrapped with Windows DPAPI at
`CurrentUser` scope with the vault's own master salt as extra entropy. Only your
Windows account on this machine can unwrap them, and `--rotate` invalidates
every cached key automatically.

The TTL is an idle timeout that slides on each use: **2 minutes for the data
key** — the one that actually reveals secrets — and 15 for the master key, which
only opens metadata. Both are in `src/config.ps1`.

**The honest caveat:** while the vault is unlocked, anything running as your
Windows account can unwrap those keys. That is what "unlocked" means. `--lock`
ends it immediately; otherwise the idle timeout does.

### Wrong passwords

Every failure sleeps longer than the last (1s, 2s, 4s … capped at 30s). Ten
consecutive failures lock the vault for 15 minutes, and that survives closing
PowerShell. A correct password clears the counter.

### Writes never corrupt the vault

Saves go to a temp file, then `File.Replace` swaps it in atomically and the
previous good copy lands in `vault.sv.bak`.

After `--rotate` the backup still opens with the **old** passwords, which would
undo the rotation for anyone holding them, so rotation offers to wipe it and
does so by default.

### `--emergency`

Needs the deletion password only: no master, no decrypt. Its verifier sits in
the cleartext header precisely so this works when nothing else does. Cracking
that verifier only buys an attacker the ability to destroy a file they already
have write access to.

It overwrites the vault, its backup, the session and the lockout state with
random bytes three times, then deletes them. `--force` skips the `ERASE`
confirmation for genuine panic use.

---

## Known ceilings

Deliberate simplifications, each with its upgrade path:

- **There is no second factor.** Your passwords are the only thing between a
  copied `vault.sv` and its contents. Anyone who obtains the file can attack it
  offline, forever, at whatever rate their hardware allows — so the strength of
  the vault is exactly the strength of your passphrase. A keyfile (a 256-bit
  random file mixed into the KDF, kept on separate media) would make offline
  attack impossible without that file regardless of password strength. It is not
  implemented; it is the single largest available gain.
- **PBKDF2, not Argon2id.** This is the real cryptographic weak point, not the
  cipher. PBKDF2-SHA256 is cheap to run on a GPU: at 600k iterations, one modern
  card tests roughly 10⁴ guesses/sec, so anything resembling a dictionary
  passphrase falls. Argon2id at 64 MB would cut that by ~100× by forcing memory
  bandwidth, but .NET has no built-in implementation and pulling in a crypto
  dependency for a vault is its own risk. Mitigate with passphrase length —
  every extra random word is worth more than any iteration count you can afford.
  Raise `Iterations` in `src/config.ps1` and `--rotate` to apply.
- **Rollback is undetectable.** GCM authenticates the contents of the file you
  have, not its freshness. Someone with write access can restore an older
  `vault.sv` (reviving a secret you deleted, or reverting a rotation) and nothing
  will complain. Detecting that needs trusted state kept outside the file.
- **Secure erase is best-effort.** On SSDs and copy-on-write volumes, overwritten
  blocks can survive in unmapped flash. Full-disk encryption is the real
  guarantee.
- **The lockout counter is a plain file** and can be reset by anyone who can write
  to it. The actual brute-force cost is the KDF; the counter only stops a human
  or script hammering the prompt.
- **The vault is not hidden.** An attacker with the file learns how many entries
  exist and roughly how large each secret is. Padding would hide that; it is not
  implemented.
- **No memory-locking.** PowerShell is garbage-collected and its strings are
  immutable, so a revealed value can linger in the heap until collection — the
  code can drop the reference but cannot overwrite it. Byte buffers it does
  control (passwords, keys) are zeroed explicitly. Only one value is ever
  decrypted at a time, which bounds the exposure but does not remove it.
- **The terminal remembers.** `--show` and `--all` write plaintext into your
  scrollback, where it stays until the buffer rolls over. The reveal-on-focus
  list redraws in place and leaves nothing behind; prefer it.
- **An unlocked vault trusts your Windows session.** Nothing here defends
  against malware already running as you. Full-disk encryption (BitLocker)
  covers the stolen-laptop case; this covers the copied-file case.

---

## Layout

```
secure-vault.ps1              CLI: parse one flag, dispatch, print errors
install.ps1                   adds/removes the profile function
src/
├── load.ps1                  load order, shared by the CLI and the tests
├── config.ps1                every tunable constant
├── crypto.ps1                AES-GCM, PBKDF2, fixed-time compare, zeroing
├── secure-prompt.ps1         SecureString -> bytes, no managed string
├── render.ps1                all console output and masking
├── select.ps1                the reveal-on-select table
├── vault-file.ps1            paths, ACLs, atomic writes, secure wipe
├── vault-crypto.ps1          the two-layer envelope
├── entries.ps1               entry/project shaping + paste parsing (no crypto, no I/O)
├── session.ps1               DPAPI key cache + idle TTL
├── lockout.ps1               backoff and hard lockout
└── commands/
    ├── session-commands.ps1  open, unlock, lock, status
    ├── entry-commands.ps1    show, all, add, import, move, rm, use
    ├── project-commands.ps1  projects, project, rm-project
    └── vault-commands.ps1    init, rotate, emergency
tests/secure-vault.tests.ps1  101 assertions, no framework
```

The directory is only locked down with `icacls` when secure-vault creates it, so
keep the vault on NTFS — permissions cannot be set on FAT/exFAT.

---

## Tests

```bash
pwsh -File F:\env-protect\tests\secure-vault.tests.ps1
```

Runs against a throwaway vault in `%TEMP%` with the iteration count dropped, and
covers the crypto round-trips, tamper and AAD binding, entry lookup, project
scoping, moves and legacy-vault migration, bulk-paste parsing, code burning, masking,
the backoff curve, the full save/load cycle, the DPAPI session cache and the
lockout. Everything works at the byte level rather than through the prompts —
which is why the prompt layer is its own file.

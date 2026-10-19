import { CodeBlock } from "@/components/code-block";
import { C, ExternalLink, Note, P, Section, Sub } from "@/components/section";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { LATEST_RELEASE, navLabel, REPO } from "@/lib/nav";

/** A plain list in running text, matched to P's measure. Its children are <li>s. */
function List({ children }: { children: React.ReactNode }) {
  return (
    <ul className="max-w-[68ch] list-disc space-y-2 pl-5 text-base leading-relaxed">
      {children}
    </ul>
  );
}

const HASH_POWERSHELL = `(Get-FileHash .\\Saltcellar-<version>.exe -Algorithm SHA256).Hash.ToLower()
Get-Content .\\SHA256SUMS.txt`;

const HASH_MACOS = "shasum -a 256 -c SHA256SUMS.txt --ignore-missing";

const HASH_LINUX = "sha256sum -c --ignore-missing SHA256SUMS.txt";

const ATTEST = "gh attestation verify <file> --repo akshit-bansal11/saltcellar";

const BUILD = `npm ci
npm run check                 # format, lint, type-check, all tests, build the page
npx tauri build --no-bundle   # target/release/saltcellar.exe
npx tauri build --bundles app,dmg   # macOS; Linux: --bundles appimage,deb`;

export function DocsSections() {
  return (
    <>
      <Section
        id="install"
        eyebrow="Start here"
        title={navLabel("install")}
        lede="One file per system. Nothing to set up and nothing written outside your own user folder."
      >
        <P>
          <strong>Windows</strong> 10 or 11, 64-bit: the portable{" "}
          <C>Saltcellar-&lt;version&gt;.exe</C>. It needs WebView2, which is built into
          Windows 11 and into any Windows 10 kept up to date.
        </P>
        <P>
          <strong>macOS</strong>, Apple silicon or Intel: the universal{" "}
          <C>Saltcellar-&lt;version&gt;-macos-universal.dmg</C>.
        </P>
        <P>
          <strong>Linux</strong>, x86_64:{" "}
          <C>Saltcellar-&lt;version&gt;-linux-x86_64.AppImage</C> or{" "}
          <C>Saltcellar-&lt;version&gt;-linux-amd64.deb</C>. The .deb needs WebKitGTK
          4.1, which it pulls in.
        </P>
        <Note label="New in 1.1.0">
          The macOS and Linux builds are new in 1.1.0 and not yet tried on real machines
          beyond CI.
        </Note>
        <List>
          <li>
            Download your system&apos;s file and <C>SHA256SUMS.txt</C> from the same
            release, under <strong>Assets</strong> on the{" "}
            <ExternalLink href={LATEST_RELEASE}>latest release</ExternalLink>.
          </li>
          <li>Check the download, as below.</li>
          <li>
            Run it. The builds are not code-signed yet. On Windows, SmartScreen asks
            first: choose <strong>More info</strong>, then <strong>Run anyway</strong>.
            On macOS the build is ad-hoc signed but not notarized, so macOS refuses the
            first time: open <strong>System Settings → Privacy &amp; Security</strong>{" "}
            and choose <strong>Open Anyway</strong>. On Linux, make the AppImage
            executable first with <C>chmod +x</C>.
          </li>
          <li>
            On Windows, put the exe wherever you keep your tools. To remove Saltcellar,
            delete the file.
          </li>
        </List>
        <Sub>Check the download</Sub>
        <P>
          The checksum proves the file is the one the release published. The two hashes
          must be identical.
        </P>
        <Tabs defaultValue="powershell">
          <TabsList>
            <TabsTrigger value="powershell">PowerShell</TabsTrigger>
            <TabsTrigger value="macos">macOS</TabsTrigger>
            <TabsTrigger value="linux">Linux</TabsTrigger>
          </TabsList>
          <TabsContent value="powershell">
            <CodeBlock title="Compare the hash in PowerShell" code={HASH_POWERSHELL} />
          </TabsContent>
          <TabsContent value="macos">
            <CodeBlock
              title="Compare the hash with shasum"
              code={HASH_MACOS}
              caption="Prints OK next to the file when it matches."
            />
          </TabsContent>
          <TabsContent value="linux">
            <CodeBlock
              title="Compare the hash with sha256sum"
              code={HASH_LINUX}
              caption="Prints OK next to the file when it matches."
            />
          </TabsContent>
        </Tabs>
        <P>
          The attestation proves the file was built by this repository&apos;s release
          workflow from a tagged commit. It needs the{" "}
          <ExternalLink href="https://cli.github.com">GitHub CLI</ExternalLink>.
        </P>
        <CodeBlock title="Verify the build attestation" code={ATTEST} />
      </Section>

      <Section
        id="first-use"
        eyebrow="Start here"
        title={navLabel("first-use")}
        lede="The first run offers to create the vault, with one master password."
      >
        <P>
          The master password must be at least 15 characters, and Saltcellar refuses
          passwords that appear in public lists of leaked passwords. A few random words
          make a strong password that is easy to remember. Once it is entered,
          everything is open: reading, adding, editing, copying and deleting. Deleting
          asks you to confirm.
        </P>
        <P>Then you choose what happens if you forget it:</P>
        <List>
          <li>
            <strong>A recovery key (recommended).</strong> Saltcellar shows a
            32-character key once, and asks you to type it back to prove you saved it.
            Write it on paper and keep it away from the computer. If you forget the
            master password, the recovery key opens the vault and you choose a new
            password; you then get a new recovery key and the used one stops working.
          </li>
          <li>
            <strong>No recovery.</strong> Only the master password opens the vault.
            Forget it and everything in it is gone, for you and for anyone else.
          </li>
        </List>
        <Note label="No hints, no security questions">
          Either would become the easiest way into the vault, so Saltcellar offers
          neither. You can turn the recovery key on, replace it or remove it later in
          Settings.
        </Note>
      </Section>

      <Section
        id="using"
        eyebrow="Start here"
        title={navLabel("using")}
        lede="One project at a time, as a sheet with three columns: number, key and value."
      >
        <List>
          <li>Type a name and value in the last row and press Enter to add a key.</li>
          <li>
            Show, copy, edit and delete are on each row. Values stay hidden until you
            show one, and only one is shown at a time.
          </li>
          <li>
            Copying clears the clipboard after 30 seconds, if it still holds the copied
            value, and a thin line in the status bar counts it down. Selecting the text
            goes through the same protected copy. On Windows, copied values stay out of
            Win+V history and cloud clipboard. On macOS they are marked so clipboard
            managers that honour <C>org.nspasteboard.ConcealedType</C> skip them, but
            Universal Clipboard (Handoff) still carries them. On Linux, KDE&apos;s
            password-manager hint is added.
          </li>
          <li>
            <strong>Generate a password</strong> with the dice button or Ctrl+G: a
            length from 8 to 128, and which characters to use. Copy it, or choose{" "}
            <strong>Use as new key</strong> to put it in the last row.
          </li>
          <li>
            <strong>Projects</strong> keep the same key name apart: <C>DATABASE_URL</C>{" "}
            can exist once in <C>web</C> and once in <C>api</C>. New keys go to{" "}
            <C>default</C> unless you pick another.
          </li>
          <li>
            <strong>Names</strong> use letters, digits, spaces (not at either end) and{" "}
            <C>_ - . / @ + :</C>, and match regardless of case.
          </li>
          <li>
            The app locks itself after 5 minutes without use, on Ctrl+L, and whenever it
            is closed. On Windows the window is hidden from screenshots and screen
            sharing; macOS is asked to hide it too, and whether a tool honours that is
            up to macOS. Linux cannot.
          </li>
          <li>On a Mac, Cmd stands in for Ctrl in every shortcut.</li>
        </List>
        <Sub>The keyboard</Sub>
        <P>
          Everything works from the keyboard. Press <C>F1</C> (or <C>?</C>) for the full
          list. Ctrl+K opens the command palette, which lists every command and project.
          Ctrl+N adds a key, Ctrl+Shift+N makes a project, Ctrl+1 to Ctrl+9 jump to a
          project, Ctrl+PgUp and Ctrl+PgDn (or Alt+Up and Alt+Down) switch projects,
          Ctrl+F or / searches, Ctrl+G opens the generator, Ctrl+, opens settings and
          Ctrl+L locks. In the rows, Up, Down, Home and End move, Enter shows, Ctrl+C
          copies the value, Ctrl+Shift+C copies the key&apos;s name, F2 edits and Del
          deletes. In the add row, Ctrl+Enter (or Enter) adds the key.
        </P>
      </Section>

      <Section
        id="backups"
        eyebrow="Your vault"
        title={navLabel("backups")}
        lede="The vault is a single file, and a dead disk takes it with it."
      >
        <P>
          <strong>Settings → Export backup…</strong> asks for your master password
          again, then where to save. The backup is the vault file itself, still
          encrypted: it opens with the master password, or the recovery key, the vault
          had when you exported it. Keep one on a USB drive or another disk.
        </P>
        <P>
          To move to a new computer, or after losing the disk, start Saltcellar with no
          vault and choose <strong>Restore from a backup…</strong>. Saltcellar never
          restores over an existing vault.
        </P>
        <Note label="Mind where you save it">
          A folder synced by OneDrive or a similar service uploads whatever you save
          there. An old backup also opens with the password and recovery key it had, so
          delete old backups after changing either.
        </Note>
      </Section>

      <Section
        id="where"
        eyebrow="Your vault"
        title={navLabel("where")}
        lede={
          <>
            <C>%LOCALAPPDATA%\saltcellar</C> on Windows,{" "}
            <C>~/Library/Application Support/saltcellar</C> on macOS and{" "}
            <C>~/.local/share/saltcellar</C> (or <C>$XDG_DATA_HOME/saltcellar</C>) on
            Linux, in your own user profile.
          </>
        }
      >
        <List>
          <li>
            <C>vault.lokey</C>: the vault.
          </li>
          <li>
            <C>vault.lokey.bak</C>: the version before the last change.
          </li>
          <li>
            <C>lockout.json</C>: the count of wrong passwords, shared by every window.
          </li>
          <li>
            <C>migration.log</C>: only if a lokey vault was copied in, or a copy failed.
          </li>
        </List>
        <P>
          Set <C>SALTCELLAR_DIR</C> (or the older <C>LOKEY_DIR</C>) to keep them
          somewhere else. On Windows it is local, not roaming: Windows never syncs the
          folder. On macOS and Linux the files are yours only (0600). The files keep
          the <C>.lokey</C> extension from the app&apos;s previous name; the format
          inside is the same.
        </P>
        <P>
          Two Saltcellar windows can be open at once: every change takes a lock on the
          file, reads it fresh, applies one change and writes it back whole, so the two
          never overwrite each other.
        </P>
      </Section>

      <Section
        id="upgrading"
        eyebrow="Your vault"
        title={navLabel("upgrading")}
        lede="Saltcellar was called lokey until 1.0.0. The first start copies a lokey vault across, and leaves the original alone."
      >
        <List>
          <li>
            <strong>Close lokey first</strong>, every window of it.
          </li>
          <li>
            Start Saltcellar. If <C>%LOCALAPPDATA%\lokey</C> holds a vault and{" "}
            <C>%LOCALAPPDATA%\saltcellar</C> does not, it copies <C>vault.lokey</C>,{" "}
            <C>vault.lokey.bak</C> and <C>lockout.json</C> across, then asks for your
            master password as before. The status bar says the vault was copied, and{" "}
            <C>migration.log</C> records it.
          </li>
          <li>
            Once Saltcellar has opened your vault and you have checked your entries,
            delete <C>%LOCALAPPDATA%\lokey</C> and <C>lokey.exe</C> yourself. Saltcellar
            never does.
          </li>
        </List>
        <Sub>What the copy does and does not do</Sub>
        <List>
          <li>
            It copies, never moves. The lokey folder is only read: nothing in it is
            changed, moved or deleted, so lokey still opens it if anything goes wrong.
          </li>
          <li>
            It happens once. If the Saltcellar folder already holds a vault, nothing is
            copied and both are left alone.
          </li>
          <li>
            It is skipped when <C>SALTCELLAR_DIR</C> or <C>LOKEY_DIR</C> is set:
            Saltcellar opens the folder you chose.
          </li>
          <li>
            If the copy fails, Saltcellar shows the reason instead of offering to create
            a new vault, records it, and tries again on the next start.
          </li>
          <li>
            The vault format is unchanged. Backups made by lokey restore in Saltcellar.
          </li>
        </List>
        <Note label="Use one or the other">
          Changes you make in lokey after the copy stay in lokey&apos;s folder; they do
          not reach Saltcellar.
        </Note>
      </Section>

      <Section
        id="security"
        eyebrow="Under the hood"
        title={navLabel("security")}
        lede="What the vault protects against, what it does not, and what you can check yourself."
      >
        <Sub>How the vault is sealed</Sub>
        <List>
          <li>
            Your entries are encrypted with AES-256-GCM under a random 256-bit data key.
          </li>
          <li>
            The data key is stored at most twice, each copy encrypted: once under a key
            derived from your master password, and once under a key derived from the
            recovery key if you have one. Both derivations use Argon2id at 64 MiB, 3
            passes and 1 lane, with a random 16-byte salt.
          </li>
          <li>
            Changing the master password, or making or removing a recovery key, replaces
            the data key. An old password or recovery key opens only copies of the file
            made before the change.
          </li>
          <li>
            Every header field an attacker could weaken, among them the salt and the
            cost settings, is bound into the encryption. A changed file fails to open
            rather than opening weakened or wrong. A file asking for less than 19 MiB,
            or more than 1 GiB, is refused.
          </li>
          <li>
            Wrong passwords and recovery keys wait 1 second, doubling to 30, and 10 in a
            row lock the vault for 15 minutes.
          </li>
          <li>
            Exporting a backup, changing the master password and making or removing a
            recovery key each ask for the master password again, even with the vault
            open.
          </li>
          <li>
            Keys and the decrypted vault in the Rust side are wiped (<C>zeroize</C>)
            when dropped. Values shown in the window live in the web view&apos;s memory,
            which Saltcellar cannot wipe.
          </li>
          <li>
            The app makes no network requests and its content security policy blocks
            any. The system web view underneath is updated by the operating system.
          </li>
        </List>
        <Sub>What it cannot protect you from</Sub>
        <List>
          <li>
            Malware already running as you, which can record the password as you type it
            or read the screen.
          </li>
          <li>
            A weak or reused master password. Anyone with a copy of the file can guess
            offline for as long as they like; the lockout lives beside the vault and
            does not slow them.
          </li>
          <li>
            Someone who finds your recovery key. It opens the vault as fully as the
            password does.
          </li>
          <li>Someone at your computer while the vault is unlocked, until it locks.</li>
          <li>
            Old copies: a backup or vault.lokey.bak opens with the password and recovery
            key it had.
          </li>
          <li>
            SSDs keeping old blocks after a file is rewritten. Full-disk encryption
            (BitLocker, FileVault, LUKS) covers that, and the stolen-laptop case.
          </li>
        </List>
        <Note label="Not audited">
          No independent security audit has been done. The primitives come from
          RustCrypto&apos;s <C>aes-gcm</C> and <C>argon2</C> crates; how Saltcellar
          combines them has been reviewed only by its own maintainers. Everything that
          handles your data is in <C>crates/saltcellar-core</C> and <C>src-tauri</C>.
          Report a hole privately, as{" "}
          <ExternalLink href={`${REPO}/blob/main/SECURITY.md`}>
            SECURITY.md
          </ExternalLink>{" "}
          describes.
        </Note>
      </Section>

      <Section
        id="build"
        eyebrow="Under the hood"
        title={navLabel("build")}
        lede="Rust (stable) and Node.js 24 or newer, on Windows, macOS or Linux."
      >
        <CodeBlock title="Build the app" code={BUILD} />
        <P>
          <C>npm run check:ci</C> runs the same checks without changing any file; CI
          runs that. Releases are built by GitHub Actions from a tagged commit that
          passed every check, with each third-party action pinned to an exact commit,
          and carry <C>SHA256SUMS.txt</C> and a signed build attestation.{" "}
          <ExternalLink href={`${REPO}/blob/main/CONTRIBUTING.md`}>
            CONTRIBUTING.md
          </ExternalLink>{" "}
          has the rest.
        </P>
      </Section>
    </>
  );
}

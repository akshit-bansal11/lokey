import {
  ArrowRight,
  ClipboardX,
  EyeOff,
  FileLock2,
  FolderTree,
  KeyRound,
  type LucideIcon,
  ShieldCheck,
  WandSparkles,
} from "lucide-react";
import type { Metadata } from "next";
import Image from "next/image";
import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { GithubMark } from "@/components/icons/github";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { AnimatedShinyText } from "@/components/ui/animated-shiny-text";
import { Badge } from "@/components/ui/badge";
import { BlurFade } from "@/components/ui/blur-fade";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { GridPattern } from "@/components/ui/grid-pattern";
import { LATEST_RELEASE, REPO } from "@/lib/nav";

export const metadata: Metadata = {
  alternates: { canonical: "/" },
};

/** Every number on this page is a constant in crates/saltcellar-core or src-tauri. */
const FACTS: { term: string; value: string }[] = [
  { term: "Cipher", value: "AES-256-GCM" },
  { term: "Key derivation", value: "Argon2id, 64 MiB, 3 passes" },
  { term: "Network code", value: "None" },
  { term: "Runs on", value: "Windows 10 and 11, x64" },
];

const STORES: { title: string; Icon: LucideIcon; body: string }[] = [
  {
    title: "Environment keys",
    Icon: KeyRound,
    body: "The DATABASE_URL, the API token and the signing secret for each thing you build, kept by project, so the same key name can exist once in web and once in api.",
  },
  {
    title: "Passwords",
    Icon: FileLock2,
    body: "Anything you would otherwise keep in a text file. Values stay hidden until you show one, and only one is shown at a time.",
  },
  {
    title: "Backup codes",
    Icon: FolderTree,
    body: "The one-time codes a site gives you once and never again. A value can be up to 64 KiB.",
  },
  {
    title: "New passwords",
    Icon: WandSparkles,
    body: "The generator (the dice button, or Ctrl+G) draws from the operating system's random number generator: 8 to 128 characters, from the sets you choose, every chosen set present at least once.",
  },
  {
    title: "Kept off the clipboard's record",
    Icon: ClipboardX,
    body: "A copied value is kept out of Win+V history and cloud clipboard, and cleared after 30 seconds unless you copied something else.",
  },
  {
    title: "Hidden from the screen",
    Icon: EyeOff,
    body: "The window is left out of screenshots, recordings and shared screens, so a value on show does not end up in a video call.",
  },
];

const LOCAL: { title: string; body: string }[] = [
  {
    title: "There is nothing to breach",
    body: "No account, no server and no sync means no database of vaults anyone can take at once. Your vault is one file, and it is on your disk.",
  },
  {
    title: "Nothing can phone home",
    body: "The app contains no network code at all. Its content security policy allows only its own files and the channel to its Rust side, so the window cannot make a request even if asked to.",
  },
  {
    title: "Nothing syncs behind your back",
    body: "The vault lives under %LOCALAPPDATA%, not the roaming profile, so Windows never copies it to another machine. A backup goes only where you save it.",
  },
];

const ENCRYPTION: { step: string; title: string; body: string }[] = [
  {
    step: "1",
    title: "Your password becomes a key",
    body: "Argon2id turns the master password and a random 16-byte salt into a 256-bit key, using 64 MiB of memory and 3 passes. Every guess against a stolen file pays that cost again.",
  },
  {
    step: "2",
    title: "That key seals a random data key",
    body: "A random 256-bit data key is sealed with AES-256-GCM under the password's key and, if you made one, under the recovery key's. Changing either replaces the data key, so an old password opens only old copies.",
  },
  {
    step: "3",
    title: "The data key seals your entries",
    body: "The entries are sealed with AES-256-GCM under the data key. The salt and the cost settings are bound into the encryption, so an edited file fails to open instead of opening weakened.",
  },
];

const WONT: string[] = [
  "Sync. Nothing leaves the PC, so the vault is not on your phone or your other computers.",
  "Bring a vault back without a backup. If the disk dies, an exported backup is the only way back.",
  "Let you in without the master password or the recovery key. There are no hints, no security questions and no reset by email.",
  "Protect you from malware already running as you. A program that can read your keystrokes or your screen can read your vault as you use it.",
  "Run anywhere but Windows, for now. The clipboard and window protections are Windows APIs.",
  "Come signed. The exe is not code-signed yet, so SmartScreen warns the first time; the checksum and the build attestation are how you check it.",
];

function Heading({
  eyebrow,
  title,
  children,
}: {
  eyebrow: string;
  title: string;
  children?: React.ReactNode;
}) {
  return (
    <BlurFade>
      <p className="text-saffron-ink text-[0.9375rem] font-bold">{eyebrow}</p>
      <h2 className="mt-2 max-w-[20ch] text-[2rem] leading-[1.08] font-black tracking-[-0.02em] md:text-5xl">
        {title}
      </h2>
      {children ? (
        <p className="text-dim mt-4 max-w-[64ch] text-base leading-relaxed md:text-lg">
          {children}
        </p>
      ) : null}
    </BlurFade>
  );
}

/** A "read more" link into the documentation page. */
function MoreLink({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <Link
      href={href}
      className="text-saffron-ink inline-flex min-h-11 items-center gap-1.5 rounded-lg text-[0.9375rem] font-bold hover:underline hover:underline-offset-4"
    >
      {children}
      <ArrowRight aria-hidden className="size-4" />
    </Link>
  );
}

const SECTION = "mx-auto max-w-[80rem] scroll-mt-20 px-4 py-14 md:px-6 md:py-20";

const VERIFY = `gh attestation verify Saltcellar-<version>.exe --repo akshit-bansal11/saltcellar`;

export default function Page() {
  return (
    <div id="top" className="min-h-dvh">
      <SiteHeader />

      <main id="main" className="scroll-mt-16">
        <section className="relative overflow-hidden">
          <GridPattern
            width={35}
            height={35}
            className="fill-transparent stroke-line [mask-image:radial-gradient(ellipse_70%_60%_at_60%_30%,black,transparent)]"
          />
          {/* No entrance on the headline: it is the largest paint on the page, and a
              fade would leave it invisible until the JavaScript arrives. */}
          <div className="relative mx-auto grid max-w-[80rem] gap-14 px-4 pt-12 pb-16 md:px-6 md:pt-20 md:pb-24 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-center lg:gap-20">
            <div>
              <p className="bg-card shadow-card inline-flex rounded-full px-4 py-1.5 text-sm font-bold">
                <AnimatedShinyText>Local · Encrypted · Windows</AnimatedShinyText>
              </p>
              <h1 className="mt-6 max-w-[14ch] text-[2.75rem] leading-[1.02] font-black tracking-[-0.025em] sm:text-6xl lg:text-7xl">
                Your secrets, sealed on your own PC.
              </h1>
              <p className="text-dim mt-6 max-w-[56ch] text-lg leading-relaxed md:text-xl">
                Saltcellar keeps your passwords, environment keys and backup codes in
                one encrypted file on your Windows PC. One window, one file, one master
                password. No account, no sync, and no network code at all.
              </p>

              <div className="mt-8 flex flex-wrap items-center gap-3">
                <Button asChild>
                  <a href={LATEST_RELEASE} target="_blank" rel="noreferrer noopener">
                    Download
                  </a>
                </Button>
                <Button asChild variant="outline">
                  <Link href="/docs">
                    Read the docs
                    <ArrowRight aria-hidden className="size-4" />
                  </Link>
                </Button>
                <Button asChild variant="ghost">
                  <a href={REPO} target="_blank" rel="noreferrer noopener">
                    <GithubMark className="size-4" />
                    Source
                  </a>
                </Button>
              </div>

              <ul
                aria-label="What it does not need"
                className="mt-8 flex flex-wrap gap-2"
              >
                {["No account", "No cloud", "No telemetry", "MIT licensed"].map(
                  (item) => (
                    <li key={item}>
                      <Badge variant="outline">{item}</Badge>
                    </li>
                  ),
                )}
              </ul>
            </div>

            <BlurFade delay={0.1}>
              {/* The app's own icon: the repository has no screenshot of the app yet. */}
              <Image
                src="/icon.svg"
                width={256}
                height={256}
                unoptimized
                priority
                alt="The Saltcellar icon: stepped squares sinking to a keyhole, like a salt crystal or stairs down to a cellar, in saffron and dark brown."
                className="mx-auto size-48 drop-shadow-xl lg:size-64"
              />
            </BlurFade>
          </div>
        </section>

        {/* The facts, stated rather than claimed */}
        <div className="mx-auto max-w-[80rem] px-4 md:px-6">
          <BlurFade>
            <dl className="bg-line shadow-card grid gap-px overflow-hidden rounded-[var(--radius-panel)] sm:grid-cols-2 lg:grid-cols-4">
              {FACTS.map((fact) => (
                <div key={fact.term} className="bg-card px-5 py-4">
                  <dt className="label">{fact.term}</dt>
                  <dd className="mt-1 text-base font-bold">{fact.value}</dd>
                </div>
              ))}
            </dl>
          </BlurFade>
        </div>

        <section id="stores" className={SECTION}>
          <Heading eyebrow="What it stores" title="The secrets that should stay put.">
            Keys and values, grouped by project, in a sheet you drive from the keyboard.
            Everything is in one file, encrypted as a whole.
          </Heading>

          <ul className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {STORES.map(({ title, Icon, body }, index) => (
              <li key={title}>
                <BlurFade delay={(index % 3) * 0.05} className="h-full">
                  <Card className="h-full px-6 py-6">
                    <span className="bg-accent-soft text-saffron-ink grid size-11 place-items-center rounded-xl">
                      <Icon aria-hidden className="size-5" />
                    </span>
                    <div>
                      <h3 className="text-lg font-bold">{title}</h3>
                      <p className="text-dim mt-2 text-[0.9375rem] leading-relaxed">
                        {body}
                      </p>
                    </div>
                  </Card>
                </BlurFade>
              </li>
            ))}
          </ul>
        </section>

        <section id="local" className={SECTION}>
          <Heading eyebrow="Why local-only" title="No cloud is the feature.">
            A password manager with sync is a service you trust to keep a copy of every
            vault. Saltcellar has no copy to keep.
          </Heading>

          <ul className="mt-10 grid gap-4 md:grid-cols-3">
            {LOCAL.map((item, index) => (
              <li key={item.title}>
                <BlurFade delay={index * 0.05} className="h-full">
                  <Card className="h-full px-6 py-6">
                    <h3 className="text-lg leading-snug font-bold">{item.title}</h3>
                    <p className="text-dim mt-2 text-[0.9375rem] leading-relaxed">
                      {item.body}
                    </p>
                  </Card>
                </BlurFade>
              </li>
            ))}
          </ul>
        </section>

        <section id="encryption" className={SECTION}>
          <Heading eyebrow="How the encryption works" title="Two keys, one file.">
            Standard primitives from RustCrypto&apos;s aes-gcm and argon2 crates,
            combined in Rust you can read in crates/saltcellar-core.
          </Heading>

          <ol className="mt-10 grid gap-4 md:grid-cols-3">
            {ENCRYPTION.map((item, index) => (
              <li key={item.step}>
                <BlurFade delay={index * 0.05} className="h-full">
                  <Card className="h-full px-6 py-6">
                    <span className="bg-primary text-primary-foreground grid size-9 place-items-center rounded-full text-[0.9375rem] font-black">
                      {item.step}
                    </span>
                    <div>
                      <h3 className="text-lg leading-snug font-bold">{item.title}</h3>
                      <p className="text-dim mt-2 text-[0.9375rem] leading-relaxed">
                        {item.body}
                      </p>
                    </div>
                  </Card>
                </BlurFade>
              </li>
            ))}
          </ol>

          <BlurFade>
            <Card className="bg-accent-soft mt-4 px-6 py-6 shadow-none">
              <div className="flex gap-4">
                <ShieldCheck
                  aria-hidden
                  className="text-saffron-ink mt-0.5 size-6 shrink-0"
                />
                <div>
                  <h3 className="text-lg font-bold">Guessing at the prompt</h3>
                  {/* Ink on the tint, not dim: dim on accent-soft is 4.4:1 in light. */}
                  <p className="mt-2 max-w-[76ch] text-[0.9375rem] leading-relaxed">
                    Wrong passwords wait 1 second, doubling to 30, and 10 in a row lock
                    the vault for 15 minutes. That stops someone at the keyboard;
                    against a copied file only the password&apos;s strength and
                    Argon2id&apos;s cost stand in the way, which is why a new master
                    password must be at least 15 characters and not on a list of leaked
                    ones.
                  </p>
                  <p className="mt-2">
                    <MoreLink href="/docs#security">The whole security model</MoreLink>
                  </p>
                </div>
              </div>
            </Card>
          </BlurFade>
        </section>

        <section className={SECTION}>
          <div className="grid gap-10 lg:grid-cols-[minmax(0,24rem)_minmax(0,1fr)] lg:gap-16">
            <Heading eyebrow="The cost" title="What it will not do.">
              Keeping everything on one PC has a price. Nothing here is hidden because
              it is inconvenient.
            </Heading>

            <BlurFade delay={0.05}>
              <ul className="bg-card shadow-card divide-line divide-y rounded-[var(--radius-panel)] px-5">
                {WONT.map((item) => (
                  <li key={item} className="py-4 text-[0.9375rem] leading-relaxed">
                    {item}
                  </li>
                ))}
              </ul>
            </BlurFade>
          </div>
        </section>

        <section id="get" className={SECTION}>
          <Heading eyebrow="Get it" title="One portable exe.">
            No installer, no administrator rights, nothing written outside your own user
            folder. Download Saltcellar-&lt;version&gt;.exe and SHA256SUMS.txt from the
            same release, check them, and run it.
          </Heading>

          <div className="mt-10 grid grid-cols-[minmax(0,1fr)] gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
            <BlurFade className="h-full">
              <Card className="h-full px-6 py-6">
                <div className="flex flex-wrap gap-2">
                  <Badge variant="solid">Windows 10 / 11, x64</Badge>
                  <Badge variant="outline">Saltcellar-&lt;version&gt;.exe</Badge>
                </div>
                <ol className="divide-line divide-y">
                  {[
                    "Download the exe and SHA256SUMS.txt from the latest release.",
                    "Check the hash, and the attestation if you have the GitHub CLI.",
                    "Run it. SmartScreen asks first: More info, then Run anyway.",
                    "Choose a master password, and a recovery key to write on paper.",
                  ].map((step, index) => (
                    <li key={step} className="flex gap-3 py-3 first:pt-0 last:pb-0">
                      <span className="text-saffron-ink w-4 shrink-0 text-[0.9375rem] font-black">
                        {index + 1}
                      </span>
                      <span className="text-dim text-[0.9375rem] leading-relaxed">
                        {step}
                      </span>
                    </li>
                  ))}
                </ol>
              </Card>
            </BlurFade>
            <BlurFade delay={0.05} className="h-full">
              <CodeBlock
                title="Check where it was built"
                code={VERIFY}
                caption="Proves the file came from this repository's release workflow, at a tagged commit. Coming from lokey? The first start copies your vault across and leaves the old one alone."
              />
            </BlurFade>
          </div>

          <BlurFade>
            <div className="mt-10 flex flex-wrap items-center gap-3">
              <Button asChild>
                <a href={LATEST_RELEASE} target="_blank" rel="noreferrer noopener">
                  Download the latest release
                </a>
              </Button>
              <MoreLink href="/docs#upgrading">Upgrading from lokey</MoreLink>
              <MoreLink href="/docs#install">Install, step by step</MoreLink>
            </div>
          </BlurFade>
        </section>
      </main>

      <SiteFooter />
    </div>
  );
}

# Saltcellar website

Two routes: a landing page at `/`, and the whole documentation on one page at `/docs`. The stack (Next.js static export, Tailwind, shadcn-style components, Biome, ESLint, tsc) and its versions are copied from Edgepad's site, so the two are maintained the same way.

**Canonical host: [saltcellar.akshitbansal.me](https://saltcellar.akshitbansal.me).** Deployed to Vercel from the repository root (not from `site/`), using the root `vercel.json`: it installs and builds in `site/`, serves `site/out`, and sets the security headers.

It is **not** part of the app's quality gate: `npm run check` at the root checks only the app. The site's own gate and build run in `.github/workflows/site.yml` on every push and pull request that touches `site/`.

## Running it

```bash
cd site
npm ci
npm run dev        # http://localhost:3000
```

## The quality gate

```bash
npm run check      # Biome (format + import order + lint) -> ESLint -> tsc
npm run check:ci   # the same, non-mutating: fails on unformatted code
npm run build      # the static export, into out/
```

| Tool | Owns | Command |
| --- | --- | --- |
| **Biome** | Formatting, import order, general JS/TS lint | `biome check --write .` |
| **ESLint** | React hooks, JSX, accessibility, Next.js rules | `eslint .` |
| **tsc** | Type checking | `tsc --noEmit` |

`noExplicitAny` is an error, and `noConsole` is an error that allows `warn` and `error` only.

## What it must not do

The site describes an app whose promise is "no network, ever", so the site keeps to it too: no analytics, no trackers, no remote fonts and no third-party requests. Text is set in the system's own fonts, as in the app. The Content-Security-Policy in `vercel.json` allows only the site's own origin.

Every fact on it comes from the code: the cipher, the key derivation and its parameters from `crates/saltcellar-core`, the limits from the app's constants. There are no screenshots yet; the hero shows the app's icon.

import type { Metadata, Viewport } from "next";
import { ThemeProvider } from "@/components/theme-provider";
import { SITE, SOCIAL_IMAGE } from "@/lib/nav";
import "./globals.css";

/*
  The site sets text in the system's own fonts, as the app does: no font is fetched,
  at build time or in the browser. Edgepad's site loads Lato and JetBrains Mono
  through next/font/google; that is the one part of its stack not copied here.
*/

const TITLE = "Saltcellar — an encrypted vault that never leaves your computer";
const DESCRIPTION =
  "Saltcellar keeps your passwords, environment keys and backup codes in one encrypted file on your own computer, on Windows, macOS or Linux. AES-256-GCM under an Argon2id key. No account, no sync, no network requests. MIT.";

export const metadata: Metadata = {
  // Every relative URL in metadata (Open Graph, canonical) resolves against this.
  metadataBase: new URL(SITE),
  // The landing page's title is the default; app/docs/page.tsx overrides it. The
  // canonical link is each page's own, so a future page never inherits "/" by accident.
  title: TITLE,
  description: DESCRIPTION,
  applicationName: "Saltcellar",
  authors: [{ name: "akshit-bansal11" }],
  openGraph: {
    title: TITLE,
    description: DESCRIPTION,
    type: "website",
    url: "/",
    siteName: "Saltcellar",
    images: [SOCIAL_IMAGE],
  },
  twitter: {
    card: "summary_large_image",
    images: [SOCIAL_IMAGE],
  },
};

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#f2f3f7" },
    { media: "(prefers-color-scheme: dark)", color: "#0e0f12" },
  ],
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en" suppressHydrationWarning>
      {/* Without JavaScript, motion never animates the blocks in: show them as they are. */}
      <head>
        <noscript>
          <style>
            {
              ".blur-fade{opacity:1!important;filter:none!important;transform:none!important}"
            }
          </style>
        </noscript>
      </head>
      <body className="font-sans">
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          enableSystem
          disableTransitionOnChange
        >
          {children}
        </ThemeProvider>
      </body>
    </html>
  );
}

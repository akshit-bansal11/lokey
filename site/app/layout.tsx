import type { Metadata, Viewport } from "next";
import { ThemeProvider } from "@/components/theme-provider";
import { ICON, SITE } from "@/lib/nav";
import "./globals.css";

/*
  The site sets text in the system's own fonts, as the app does: no font is fetched,
  at build time or in the browser. Edgepad's site loads Lato and JetBrains Mono
  through next/font/google; that is the one part of its stack not copied here.
*/

const TITLE = "Saltcellar — an encrypted vault that never leaves your PC";
const DESCRIPTION =
  "Saltcellar keeps your passwords, environment keys and backup codes in one encrypted file on your own Windows PC. AES-256-GCM under an Argon2id key. No account, no sync, no network code. MIT.";

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
    images: [ICON],
  },
  twitter: {
    card: "summary",
    images: [ICON],
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

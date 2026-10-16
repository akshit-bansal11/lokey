import type { Metadata } from "next";
import { DocsSections } from "@/components/sections/docs";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { Toc } from "@/components/toc";
import { SOCIAL_IMAGE } from "@/lib/nav";

const TITLE = "Saltcellar — documentation";
const DESCRIPTION =
  "Saltcellar documentation on one page: install and check the download, first use, backups, where the vault lives, upgrading from lokey, the security model and building from source.";

// A page's openGraph replaces the layout's whole object rather than merging into it,
// hence type, site name and image again here.
export const metadata: Metadata = {
  title: TITLE,
  description: DESCRIPTION,
  alternates: { canonical: "/docs" },
  openGraph: {
    title: TITLE,
    description: DESCRIPTION,
    type: "website",
    url: "/docs",
    siteName: "Saltcellar",
    images: [SOCIAL_IMAGE],
  },
};

export default function DocsPage() {
  return (
    <div id="top" className="min-h-dvh">
      <SiteHeader variant="docs" />

      <main id="main" className="mx-auto scroll-mt-16 max-w-[90rem] px-4 md:px-6">
        {/* No entrance on the title block: it is the first paint, and a fade would
            hide it until the JavaScript arrives. */}
        <div className="py-10 md:py-16">
          <p className="text-primary mb-2 text-[0.9375rem] font-bold">Documentation</p>
          <h1 className="max-w-[20ch] text-[2.5rem] leading-[1.04] font-black tracking-[-0.025em] md:text-6xl">
            Everything Saltcellar does, and how.
          </h1>
          <p className="text-dim mt-5 max-w-[64ch] text-base leading-relaxed md:text-lg">
            Installing and checking the download, your first vault, everyday use,
            backups, where the files live, upgrading from lokey, the security model and
            building it yourself.
          </p>
        </div>

        <div className="lg:grid lg:grid-cols-[17rem_minmax(0,1fr)] lg:gap-12">
          <aside className="hidden lg:block">
            <div className="bg-card shadow-card sticky top-20 max-h-[calc(100dvh-6rem)] overflow-y-auto rounded-[var(--radius-panel)] p-3 pt-4">
              <Toc />
            </div>
          </aside>

          <div className="divide-line min-w-0 divide-y">
            <DocsSections />
          </div>
        </div>
      </main>

      <SiteFooter />
    </div>
  );
}

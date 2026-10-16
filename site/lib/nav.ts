/**
 * The docs page's table of contents. One list, used three times: by the sidebar, by
 * the mobile menu, and by the scroll-spy. Every `id` here must exist as a section
 * anchor on the page, which is why the sections take their heading from this list
 * rather than repeating it.
 */
export type NavItem = { id: string; label: string };
export type NavGroup = { label: string; items: NavItem[] };

export const NAV: NavGroup[] = [
  {
    label: "Start here",
    items: [
      { id: "install", label: "Install" },
      { id: "first-use", label: "First use" },
      { id: "using", label: "Using it" },
    ],
  },
  {
    label: "Your vault",
    items: [
      { id: "backups", label: "Backups" },
      { id: "where", label: "Where the vault lives" },
      { id: "upgrading", label: "Upgrading from lokey" },
    ],
  },
  {
    label: "Under the hood",
    items: [
      { id: "security", label: "Security model" },
      { id: "build", label: "Build from source" },
    ],
  },
];

export const ALL_NAV_IDS: string[] = NAV.flatMap((group) =>
  group.items.map((item) => item.id),
);

/** Looks up a section's label, so a section's heading and its sidebar entry agree. */
export function navLabel(id: string): string {
  const item = NAV.flatMap((group) => group.items).find((entry) => entry.id === id);
  if (!item) throw new Error(`no nav entry for section "${id}"`);
  return item.label;
}

export const REPO = "https://github.com/akshit-bansal11/saltcellar";
export const LATEST_RELEASE = `${REPO}/releases/latest`;

/** The site's one origin. Canonical links, the sitemap and robots.txt all resolve against it. */
export const SITE = "https://saltcellar.akshitbansal.me";

/** The link preview every page shares: the icon, the name and the one-line claim. */
export const SOCIAL_IMAGE = {
  url: "/og.png",
  width: 1200,
  height: 630,
  alt: "Saltcellar: an encrypted vault that never leaves your PC. No network, ever.",
};

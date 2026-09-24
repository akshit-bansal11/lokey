// Icon geometry from Lucide (lucide-static 1.47.0, ISC licence,
// https://lucide.dev). Inlined so the app ships no icon package and makes no
// request. One set only: every icon in lokey comes from this table.

const SVG_NS = "http://www.w3.org/2000/svg";

type Shape =
  | { tag: "path"; d: string }
  | { tag: "circle"; cx: number; cy: number; r: number }
  | { tag: "rect"; x: number; y: number; width: number; height: number; rx: number };

const ICONS = {
  eye: [
    {
      tag: "path",
      d: "M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0",
    },
    { tag: "circle", cx: 12, cy: 12, r: 3 },
  ],
  eyeOff: [
    {
      tag: "path",
      d: "M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49",
    },
    { tag: "path", d: "M14.084 14.158a3 3 0 0 1-4.242-4.242" },
    {
      tag: "path",
      d: "M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143",
    },
    { tag: "path", d: "m2 2 20 20" },
  ],
  copy: [
    { tag: "rect", x: 8, y: 8, width: 14, height: 14, rx: 2 },
    { tag: "path", d: "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" },
  ],
  pencil: [
    {
      tag: "path",
      d: "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
    },
    { tag: "path", d: "m15 5 4 4" },
  ],
  trash: [
    { tag: "path", d: "M10 11v6" },
    { tag: "path", d: "M14 11v6" },
    { tag: "path", d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" },
    { tag: "path", d: "M3 6h18" },
    { tag: "path", d: "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" },
  ],
  lock: [
    { tag: "rect", x: 3, y: 11, width: 18, height: 11, rx: 2 },
    { tag: "path", d: "M7 11V7a5 5 0 0 1 10 0v4" },
  ],
  search: [
    { tag: "path", d: "m21 21-4.34-4.34" },
    { tag: "circle", cx: 11, cy: 11, r: 8 },
  ],
  settings: [
    {
      tag: "path",
      d: "M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915",
    },
    { tag: "circle", cx: 12, cy: 12, r: 3 },
  ],
  keyboard: [
    { tag: "path", d: "M10 8h.01" },
    { tag: "path", d: "M12 12h.01" },
    { tag: "path", d: "M14 8h.01" },
    { tag: "path", d: "M16 12h.01" },
    { tag: "path", d: "M18 8h.01" },
    { tag: "path", d: "M6 8h.01" },
    { tag: "path", d: "M7 16h10" },
    { tag: "path", d: "M8 12h.01" },
    { tag: "rect", x: 2, y: 4, width: 20, height: 16, rx: 2 },
  ],
} satisfies Record<string, Shape[]>;

export type IconName = keyof typeof ICONS;

const SVG_ATTRIBUTES = {
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  "stroke-width": "2",
  "stroke-linecap": "round",
  "stroke-linejoin": "round",
  "aria-hidden": "true",
  focusable: "false",
};

function isIconName(name: string): name is IconName {
  return Object.hasOwn(ICONS, name);
}

/** A decorative icon: the control it sits in carries the accessible name. */
export function icon(name: IconName): SVGSVGElement {
  const svg = document.createElementNS(SVG_NS, "svg");
  for (const [attribute, value] of Object.entries(SVG_ATTRIBUTES)) {
    svg.setAttribute(attribute, value);
  }
  for (const shape of ICONS[name]) {
    const node = document.createElementNS(SVG_NS, shape.tag);
    for (const [attribute, value] of Object.entries(shape)) {
      if (attribute !== "tag") node.setAttribute(attribute, String(value));
    }
    svg.append(node);
  }
  return svg;
}

/** Replaces every `<span data-icon="name">` placeholder under `root`. */
export function fillIcons(root: ParentNode): void {
  for (const slot of root.querySelectorAll<HTMLElement>("[data-icon]")) {
    const name = slot.dataset.icon ?? "";
    if (isIconName(name)) slot.replaceWith(icon(name));
  }
}

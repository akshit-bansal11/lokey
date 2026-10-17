// The two shortcut surfaces: the command palette (Ctrl+K) and the generated
// list in the shortcuts dialog. Both read src/lib/shortcuts.ts.

import { byId } from "@/lib/dom.ts";
import { GROUPS, SHORTCUTS } from "@/lib/shortcuts.ts";
import { announce } from "@/lib/status.ts";

export type PaletteCommand = {
  id: string;
  label: string;
  /** Chips shown beside the label. */
  keys: string[];
  run: () => void;
};

function chips(keys: string[]): HTMLElement[] {
  return keys.map((key) => {
    const kbd = document.createElement("kbd");
    kbd.textContent = key;
    return kbd;
  });
}

/** Fills the shortcuts dialog from the registry and opens it. */
export function showShortcutList(): void {
  const host = byId("keys-groups", HTMLElement);
  const sections = GROUPS.map((group) => {
    const section = document.createElement("section");
    section.className = "settings-section";
    const heading = document.createElement("h3");
    heading.id = `keys-group-${group.toLowerCase()}`;
    heading.textContent = group;
    section.setAttribute("aria-labelledby", heading.id);
    const list = document.createElement("dl");
    list.className = "keys";
    for (const shortcut of SHORTCUTS.filter((s) => s.group === group)) {
      const term = document.createElement("dt");
      term.append(...chips(shortcut.keys));
      const detail = document.createElement("dd");
      detail.textContent = shortcut.description;
      list.append(term, detail);
    }
    section.append(heading, list);
    return section;
  });
  host.replaceChildren(...sections);
  byId("keys-dialog", HTMLDialogElement).showModal();
}

/** Every word typed must appear in the label. */
export function filterCommands(commands: PaletteCommand[], query: string): PaletteCommand[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return commands.filter((c) => words.every((w) => c.label.toLowerCase().includes(w)));
}

const dialog = byId("palette-dialog", HTMLDialogElement);
const input = byId("palette-input", HTMLInputElement);
const list = byId("palette-list", HTMLElement);
const empty = byId("palette-empty", HTMLElement);

let all: PaletteCommand[] = [];
let shown: PaletteCommand[] = [];
let active = 0;

function optionId(index: number): string {
  return `palette-option-${index}`;
}

function setActive(index: number): void {
  active = shown.length === 0 ? 0 : (index + shown.length) % shown.length;
  const options = [...list.children];
  options.forEach((option, i) => {
    option.setAttribute("aria-selected", String(i === active));
  });
  if (shown.length === 0) input.removeAttribute("aria-activedescendant");
  else input.setAttribute("aria-activedescendant", optionId(active));
  options[active]?.scrollIntoView?.({ block: "nearest" });
}

function run(command: PaletteCommand | undefined): void {
  if (!command) return;
  dialog.close();
  command.run();
}

function renderList(): void {
  shown = filterCommands(all, input.value);
  list.replaceChildren(
    ...shown.map((command, index) => {
      const item = document.createElement("div");
      item.id = optionId(index);
      item.setAttribute("role", "option");
      const label = document.createElement("span");
      label.textContent = command.label;
      const keys = document.createElement("span");
      keys.className = "palette-keys";
      keys.append(...chips(command.keys));
      item.append(label, keys);
      // Keep focus in the input while the pointer works the list.
      item.addEventListener("pointerdown", (event) => event.preventDefault());
      item.addEventListener("click", () => run(command));
      return item;
    }),
  );
  empty.hidden = shown.length > 0;
  setActive(0);
  if (shown.length === 0) announce("No matching commands.");
}

input.addEventListener("input", renderList);
input.addEventListener("keydown", (event) => {
  const bare = !event.ctrlKey && !event.metaKey && !event.altKey;
  if (event.key === "ArrowDown" && bare) {
    event.preventDefault();
    setActive(active + 1);
  } else if (event.key === "ArrowUp" && bare) {
    event.preventDefault();
    setActive(active - 1);
  } else if (event.key === "Enter") {
    event.preventDefault();
    run(shown[active]);
  } else if (event.key.toLowerCase() === "k" && (event.ctrlKey || event.metaKey)) {
    event.preventDefault();
    dialog.close();
  }
});
dialog.addEventListener("close", () => {
  input.value = "";
  list.replaceChildren();
  shown = [];
});

/** Opens the palette over the given commands (rebuilt on every open). */
export function openPalette(commands: PaletteCommand[]): void {
  all = commands;
  input.value = "";
  renderList();
  dialog.showModal();
  input.focus();
}

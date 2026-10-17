// The one list of keyboard shortcuts. The shortcuts dialog, the command
// palette, the toolbar tooltips and the status bar hint are all generated from
// it, and vault.ts dispatches through it, so none of them can drift apart.
//
// Pure data and matchers: no DOM, so node:test can run it.

export type ShortcutGroup = "Vault" | "Projects" | "Rows";

/**
 * Where a shortcut is live. "global" works anywhere in the vault, "row" only
 * with focus on a row's button, "add" in the add row's fields, and "native"
 * is listed for the dialog but handled by the browser (never matched here).
 */
export type ShortcutScope = "global" | "row" | "add" | "native";

/** The part of a KeyboardEvent the matchers read. */
export type KeyLike = {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  target?: object | null;
};

export type Shortcut = {
  id: string;
  /** Chips for display, e.g. "Ctrl+Shift+N". */
  keys: string[];
  group: ShortcutGroup;
  scope: ShortcutScope;
  description: string;
  /** Status bar hint: the key to show and what it does. */
  hint?: { key: string; label: string };
  match: (event: KeyLike) => boolean;
};

const FIELD_TAGS = new Set(["INPUT", "SELECT", "TEXTAREA"]);

/** True while typing could be happening; single-character keys must not fire (SC 2.1.4). */
export function inField(event: KeyLike): boolean {
  // A DOM EventTarget in the app, a plain object in the tests.
  const target = event.target as
    | { tagName?: string; isContentEditable?: boolean }
    | null
    | undefined;
  if (!target) return false;
  return Boolean(target.isContentEditable) || FIELD_TAGS.has((target.tagName ?? "").toUpperCase());
}

/** Ctrl on Windows and Linux, Cmd on a Mac: both are accepted. */
const mod = (event: KeyLike): boolean => event.ctrlKey || event.metaKey;
const plain = (event: KeyLike): boolean =>
  !event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey;
/** Modifier plus a letter, with Shift as given and no Alt. */
const chord = (event: KeyLike, key: string, shift = false): boolean =>
  mod(event) && !event.altKey && event.shiftKey === shift && event.key.toLowerCase() === key;

const ROW_MOVE_KEYS = ["ArrowDown", "ArrowUp", "Home", "End"] as const;
export type RowMoveKey = (typeof ROW_MOVE_KEYS)[number];

/** Which row-to-row key this is, if it is one (no modifiers). */
export function rowMoveKey(event: KeyLike): RowMoveKey | undefined {
  if (!plain(event)) return undefined;
  return ROW_MOVE_KEYS.find((key) => key === event.key);
}

/** Ctrl+1 to Ctrl+9 as a zero-based project index. */
export function projectIndex(event: KeyLike): number | undefined {
  if (!mod(event) || event.altKey || event.shiftKey) return undefined;
  return /^[1-9]$/.test(event.key) ? Number(event.key) - 1 : undefined;
}

export const SHORTCUTS: readonly Shortcut[] = [
  {
    id: "palette",
    keys: ["Ctrl+K"],
    group: "Vault",
    scope: "global",
    description: "Command palette",
    hint: { key: "Ctrl+K", label: "commands" },
    match: (e) => chord(e, "k"),
  },
  {
    id: "add-key",
    keys: ["Ctrl+N"],
    group: "Vault",
    scope: "global",
    description: "Add a key",
    match: (e) => chord(e, "n"),
  },
  {
    id: "new-project",
    keys: ["Ctrl+Shift+N"],
    group: "Vault",
    scope: "global",
    description: "New project",
    match: (e) => chord(e, "n", true),
  },
  {
    id: "generate",
    keys: ["Ctrl+G"],
    group: "Vault",
    scope: "global",
    description: "Generate a password",
    match: (e) => chord(e, "g"),
  },
  {
    id: "search",
    keys: ["Ctrl+F", "/"],
    group: "Vault",
    scope: "global",
    description: "Search keys",
    hint: { key: "/", label: "search" },
    match: (e) => chord(e, "f") || (e.key === "/" && !mod(e) && !e.altKey && !inField(e)),
  },
  {
    id: "settings",
    keys: ["Ctrl+,"],
    group: "Vault",
    scope: "global",
    description: "Settings",
    match: (e) => mod(e) && !e.altKey && !e.shiftKey && e.key === ",",
  },
  {
    id: "lock",
    keys: ["Ctrl+L"],
    group: "Vault",
    scope: "global",
    description: "Lock the vault",
    hint: { key: "Ctrl+L", label: "lock" },
    match: (e) => mod(e) && e.key.toLowerCase() === "l",
  },
  {
    id: "help",
    keys: ["F1", "?"],
    group: "Vault",
    scope: "global",
    description: "Keyboard shortcuts",
    match: (e) => e.key === "F1" || (e.key === "?" && !mod(e) && !e.altKey && !inField(e)),
  },
  {
    id: "project-prev",
    keys: ["Ctrl+PgUp", "Alt+↑"],
    group: "Projects",
    scope: "global",
    description: "Previous project",
    match: (e) =>
      (mod(e) && !e.altKey && !e.shiftKey && e.key === "PageUp") ||
      (e.altKey && !mod(e) && !e.shiftKey && e.key === "ArrowUp"),
  },
  {
    id: "project-next",
    keys: ["Ctrl+PgDn", "Alt+↓"],
    group: "Projects",
    scope: "global",
    description: "Next project",
    match: (e) =>
      (mod(e) && !e.altKey && !e.shiftKey && e.key === "PageDown") ||
      (e.altKey && !mod(e) && !e.shiftKey && e.key === "ArrowDown"),
  },
  {
    id: "project-jump",
    keys: ["Ctrl+1…9"],
    group: "Projects",
    scope: "global",
    description: "Go to the Nth project",
    match: (e) => projectIndex(e) !== undefined,
  },
  {
    id: "row-move",
    keys: ["↑", "↓", "Home", "End"],
    group: "Rows",
    scope: "row",
    description: "Move between rows",
    match: (e) => rowMoveKey(e) !== undefined,
  },
  {
    id: "row-tab",
    keys: ["Tab"],
    group: "Rows",
    scope: "native",
    description: "Move between a row's buttons",
    match: () => false,
  },
  {
    id: "row-activate",
    keys: ["Enter", "Space"],
    group: "Rows",
    scope: "native",
    description: "Use the focused button",
    match: () => false,
  },
  {
    id: "row-copy",
    keys: ["Ctrl+C"],
    group: "Rows",
    scope: "row",
    description: "Copy the value",
    match: (e) => chord(e, "c"),
  },
  {
    id: "row-copy-name",
    keys: ["Ctrl+Shift+C"],
    group: "Rows",
    scope: "row",
    description: "Copy the key's name",
    match: (e) => chord(e, "c", true),
  },
  {
    id: "row-edit",
    keys: ["F2"],
    group: "Rows",
    scope: "row",
    description: "Edit the value",
    match: (e) => plain(e) && e.key === "F2",
  },
  {
    id: "row-delete",
    keys: ["Del"],
    group: "Rows",
    scope: "row",
    description: "Delete the key",
    match: (e) => plain(e) && e.key === "Delete",
  },
  {
    id: "row-escape",
    keys: ["Esc"],
    group: "Rows",
    scope: "row",
    description: "Hide the value, then back to search",
    match: (e) => plain(e) && e.key === "Escape",
  },
  {
    id: "add-submit",
    keys: ["Ctrl+Enter", "Enter"],
    group: "Vault",
    scope: "add",
    description: "Add the key, from either field",
    match: (e) => e.key === "Enter" && !e.altKey && !e.shiftKey,
  },
];

export const GROUPS: readonly ShortcutGroup[] = ["Vault", "Projects", "Rows"];

export function shortcutById(id: string): Shortcut {
  const found = SHORTCUTS.find((shortcut) => shortcut.id === id);
  if (!found) throw new Error(`no shortcut ${id}`);
  return found;
}

/** The first shortcut in a scope that the event triggers. */
export function findShortcut(event: KeyLike, scope: ShortcutScope): Shortcut | undefined {
  return SHORTCUTS.find((shortcut) => shortcut.scope === scope && shortcut.match(event));
}

/** A tooltip such as "Lock (Ctrl+L)". */
export function titleWithKey(label: string, id: string): string {
  return `${label} (${shortcutById(id).keys[0] ?? ""})`;
}

/** The status bar hint: "Ctrl+K commands · / search · Ctrl+L lock". */
export function hintText(): string {
  return SHORTCUTS.flatMap((s) => (s.hint ? [`${s.hint.key} ${s.hint.label}`] : [])).join(" · ");
}

// The vault as a sheet: # | Key | Value, one project at a time.
//
// Values are never in the page until asked for. The masked cell shows twelve
// dots whatever the value's length, so the mask reveals nothing. At most one
// value is shown at a time; showing another hides the first.

import { api, toFailure } from "@/lib/api.ts";
import { byId, find, mount } from "@/lib/dom.ts";
import { type IconName, icon } from "@/lib/icons.ts";
import {
  DEFAULT_PROJECT,
  nameProblem,
  projectNames,
  sameName,
  visibleRows,
  wouldReplace,
} from "@/lib/rows.ts";
import { announce, showShortcuts, startClipboardTimer } from "@/lib/status.ts";
import type { Row, Snapshot } from "@/lib/types.ts";
import { askProjectName, confirmDelete, openSettings } from "@/views/dialogs.ts";

const MASK = "••••••••••••";
const NEW_PROJECT = "\u0000new";
const ACTIONS = ["reveal", "copy", "edit", "delete"] as const;
type Action = (typeof ACTIONS)[number];
const ACTION_ICONS: Record<Action, IconName> = {
  reveal: "eye",
  copy: "copy",
  edit: "pencil",
  delete: "trash",
};
const ACTION_LABELS: Record<Action, string> = {
  reveal: "Show",
  copy: "Copy",
  edit: "Edit",
  delete: "Delete",
};
/** Activity pings keep the idle lock from firing while someone is working. */
const TOUCH_EVERY_MS = 30_000;

type Revealed = { project: string; key: string; value: string };

type VaultOptions = { vaultPath: string; onLock: () => void };

let snapshot: Snapshot;
let project = DEFAULT_PROJECT;
let pendingProjects: string[] = [];
let revealed: Revealed | undefined;
let lastTouch = 0;
let options: VaultOptions;
/** True while a value is being edited in place; see `render`. */
let editing = false;

function isAction(value: string | undefined): value is Action {
  return ACTIONS.some((action) => action === value);
}

function rowsBody(): HTMLTableSectionElement {
  return byId("rows", HTMLTableSectionElement);
}

function rowFor(target: EventTarget | null): HTMLTableRowElement | undefined {
  return target instanceof Element ? (target.closest("tbody tr") ?? undefined) : undefined;
}

function rowKey(row: HTMLTableRowElement): { project: string; key: string } {
  return { project: row.dataset.project ?? "", key: row.dataset.key ?? "" };
}

function alertText(message: string): void {
  byId("vault-alert", HTMLElement).textContent = message;
}

function touch(): void {
  const now = Date.now();
  if (now - lastTouch < TOUCH_EVERY_MS) return;
  lastTouch = now;
  void api.touch().catch(() => undefined);
}

function actionButton(action: Action, key: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "btn icon";
  button.dataset.action = action;
  const label = `${ACTION_LABELS[action]} ${key}`;
  button.setAttribute("aria-label", label);
  button.title = label;
  if (action === "reveal") button.setAttribute("aria-pressed", "false");
  button.append(icon(ACTION_ICONS[action]));
  return button;
}

/**
 * Masked content: the dots are hidden from assistive technology and replaced
 * by one spoken word, so a list is not read as twelve bullets per row.
 */
function setMasked(text: HTMLElement, empty: boolean): void {
  text.dataset.state = "masked";
  text.dataset.empty = String(empty);
  text.removeAttribute("title");
  if (empty) {
    text.textContent = "(empty)";
    return;
  }
  const dots = document.createElement("span");
  dots.setAttribute("aria-hidden", "true");
  dots.textContent = MASK;
  const spoken = document.createElement("span");
  spoken.className = "visually-hidden";
  spoken.textContent = "hidden";
  text.replaceChildren(dots, spoken);
}

function buildRow(row: Row, index: number): HTMLTableRowElement {
  const tr = document.createElement("tr");
  tr.dataset.project = row.project;
  tr.dataset.key = row.key;

  const num = document.createElement("td");
  num.className = "num";
  num.textContent = String(index + 1);

  const key = document.createElement("th");
  key.scope = "row";
  const keyText = document.createElement("span");
  keyText.className = "cell-text mono";
  keyText.textContent = row.key;
  keyText.title = row.key;
  key.append(keyText);

  const value = document.createElement("td");
  const cell = document.createElement("div");
  cell.className = "value-cell";
  const text = document.createElement("span");
  text.className = "cell-text mono";
  setMasked(text, row.length === 0);
  const actions = document.createElement("span");
  actions.className = "row-actions";
  actions.append(...ACTIONS.map((action) => actionButton(action, row.key)));
  cell.append(text, actions);
  value.append(cell);

  tr.append(num, key, value);
  return tr;
}

function showValue(tr: HTMLTableRowElement, value: string | undefined): void {
  const text = find(tr, ".value-cell .cell-text", HTMLElement);
  const toggle = find(tr, '[data-action="reveal"]', HTMLButtonElement);
  const { key } = rowKey(tr);
  if (value === undefined) {
    setMasked(text, text.dataset.empty === "true");
    toggle.setAttribute("aria-pressed", "false");
    toggle.setAttribute("aria-label", `Show ${key}`);
    toggle.replaceChildren(icon("eye"));
  } else {
    text.dataset.state = "shown";
    text.textContent = value === "" ? "(empty)" : value;
    text.title = value;
    toggle.setAttribute("aria-pressed", "true");
    toggle.setAttribute("aria-label", `Hide ${key}`);
    toggle.replaceChildren(icon("eyeOff"));
  }
}

function findRow(target: { project: string; key: string }): HTMLTableRowElement | undefined {
  return [...rowsBody().rows].find((tr) => {
    const here = rowKey(tr);
    return sameName(here.project, target.project) && sameName(here.key, target.key);
  });
}

function hideRevealed(): void {
  const current = revealed;
  revealed = undefined;
  if (!current) return;
  const tr = findRow(current);
  if (tr) showValue(tr, undefined);
}

function renderProjects(): void {
  const select = byId("project", HTMLSelectElement);
  const names = projectNames(snapshot.projects, pendingProjects);
  if (!names.some((name) => sameName(name, project))) project = DEFAULT_PROJECT;
  const counts = new Map(snapshot.projects.map((p) => [p.name.toLowerCase(), p.count]));
  const choices = names.map((name) => {
    const option = document.createElement("option");
    option.value = name;
    option.textContent = `${name} (${counts.get(name.toLowerCase()) ?? 0})`;
    option.selected = sameName(name, project);
    return option;
  });
  const create = document.createElement("option");
  create.value = NEW_PROJECT;
  create.textContent = "New project…";
  select.replaceChildren(...choices, create);
}

function render(): void {
  // A change from another window must not rebuild the row holding an open edit.
  // The snapshot is kept; the edit renders it when it finishes.
  if (editing) return;
  renderProjects();
  const query = byId("search", HTMLInputElement).value;
  const rows = visibleRows(snapshot.rows, project, query);
  const focusedRow = rowFor(document.activeElement);
  const focusedKey = focusedRow?.dataset.key;
  const focusedIndex = focusedRow ? [...rowsBody().rows].indexOf(focusedRow) : -1;
  const focusedAction =
    document.activeElement instanceof HTMLElement
      ? document.activeElement.dataset.action
      : undefined;

  rowsBody().replaceChildren(...rows.map(buildRow));
  byId("grid-caption", HTMLElement).textContent = `Keys in ${project}`;

  const empty = byId("empty", HTMLElement);
  empty.hidden = rows.length > 0;
  if (rows.length === 0) {
    empty.replaceChildren();
    if (query.trim()) {
      const clear = document.createElement("button");
      clear.type = "button";
      clear.className = "btn";
      clear.textContent = "Clear search";
      clear.addEventListener("click", () => {
        const search = byId("search", HTMLInputElement);
        search.value = "";
        render();
        search.focus();
      });
      empty.append(`No keys in ${project} match "${query.trim()}".`, clear);
    } else {
      empty.append(`No keys in ${project} yet. Add one in the row above.`);
    }
  }

  if (revealed) {
    const tr = findRow(revealed);
    if (tr) showValue(tr, revealed.value);
    else revealed = undefined;
  }
  if (focusedKey) {
    // The same row if it survived; otherwise the row now in its place, the one
    // above, or the add row, so focus never falls to <body> (SC 2.4.3).
    const remaining = [...rowsBody().rows];
    const tr =
      findRow({ project, key: focusedKey }) ??
      remaining[focusedIndex] ??
      remaining[focusedIndex - 1];
    const target = tr?.querySelector<HTMLElement>(`[data-action="${focusedAction ?? "reveal"}"]`);
    (target ?? byId("new-key", HTMLInputElement)).focus();
  }
  updateAddLabel();
}

function updateAddLabel(): void {
  const key = byId("new-key", HTMLInputElement).value;
  byId("add", HTMLButtonElement).textContent = wouldReplace(snapshot.rows, project, key)
    ? "Replace"
    : "Add";
}

/** Adopts a snapshot from any source and reports what arrived from elsewhere. */
export function applySnapshot(next: Snapshot): void {
  snapshot = next;
  pendingProjects = pendingProjects.filter(
    (name) => !next.projects.some((p) => sameName(p.name, name)),
  );
  const arrivals = next.arrivals;
  if (arrivals.length === 1 && arrivals[0]) {
    const { key, project: from, replaced } = arrivals[0];
    announce(`${key} ${replaced ? "was replaced" : "arrived"} in ${from}.`);
  } else if (arrivals.length > 1) {
    announce(`${arrivals.length} keys arrived while the vault was closed.`);
  }
  if (next.headerRestored) {
    alertText(
      "Warning: someone replaced this vault's public key, and lokey put it back. Values added with lokey set while it was replaced may have been readable by whoever changed it. Change those secrets where they were issued.",
    );
  } else if (next.rejected > 0) {
    alertText(
      `${next.rejected} value(s) added from the terminal could not be opened and were discarded.`,
    );
  }
  render();
}

async function onAction(action: Action, tr: HTMLTableRowElement): Promise<void> {
  const target = rowKey(tr);
  try {
    switch (action) {
      case "reveal": {
        const already = revealed && sameName(revealed.key, target.key);
        hideRevealed();
        if (already) return;
        const value = await api.reveal(target.project, target.key);
        revealed = { ...target, value };
        showValue(tr, value);
        return;
      }
      case "copy": {
        const seconds = await api.copy(target.project, target.key);
        startClipboardTimer(seconds);
        announce(`Copied ${target.key}. The clipboard clears in ${seconds} seconds.`);
        return;
      }
      case "edit":
        await startEdit(tr);
        return;
      case "delete":
        confirmDelete({
          title: `Delete ${target.key}?`,
          text: `Removes ${target.key} from ${target.project}. This cannot be undone.`,
          confirm: "Delete key",
          run: (deletion) => api.deleteKey(target.project, target.key, deletion),
          onDone: (next) => {
            if (revealed && sameName(revealed.key, target.key)) revealed = undefined;
            applySnapshot(next);
            announce(`Deleted ${target.key} from ${target.project}.`);
          },
        });
        return;
    }
  } catch (error) {
    handleFailure(error);
  }
}

async function startEdit(tr: HTMLTableRowElement): Promise<void> {
  const target = rowKey(tr);
  hideRevealed();
  const current = await api.reveal(target.project, target.key);
  const cell = find(tr, ".value-cell", HTMLElement);
  const text = find(cell, ".cell-text", HTMLElement);
  const label = document.createElement("label");
  label.className = "cell-text";
  const name = document.createElement("span");
  name.className = "visually-hidden";
  name.textContent = `New value for ${target.key}`;
  const input = document.createElement("input");
  input.className = "input mono";
  input.value = current;
  input.autocomplete = "off";
  input.spellcheck = false;
  label.append(name, input);
  text.replaceWith(label);
  cell.classList.add("editing");
  editing = true;
  input.focus();
  input.select();

  let done = false;
  const finish = async (save: boolean): Promise<void> => {
    if (done) return;
    done = true;
    editing = false;
    if (save && input.value !== current) {
      try {
        applySnapshot(await api.save(target.project, target.key, input.value));
        announce(`Saved the new value of ${target.key}.`);
      } catch (error) {
        handleFailure(error);
        render();
      }
    } else {
      render();
    }
    findRow(target)?.querySelector<HTMLElement>('[data-action="edit"]')?.focus();
  };
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter") void finish(true);
    if (event.key === "Escape") {
      event.preventDefault();
      void finish(false);
    }
  });
  input.addEventListener("blur", () => void finish(true));
}

async function addKey(): Promise<void> {
  const keyInput = byId("new-key", HTMLInputElement);
  const valueInput = byId("new-value", HTMLInputElement);
  const key = keyInput.value.trim();
  const problem = nameProblem("key", key);
  if (problem) {
    keyInput.setAttribute("aria-invalid", "true");
    alertText(problem);
    keyInput.focus();
    return;
  }
  keyInput.removeAttribute("aria-invalid");
  alertText("");
  const button = byId("add", HTMLButtonElement);
  button.setAttribute("aria-busy", "true");
  try {
    const next = await api.save(project, key, valueInput.value);
    keyInput.value = "";
    valueInput.value = "";
    applySnapshot(next);
    announce(`${next.saved ? "Added" : "Replaced"} ${key} in ${project}.`);
    keyInput.focus();
  } catch (error) {
    handleFailure(error);
  } finally {
    button.removeAttribute("aria-busy");
  }
}

function handleFailure(error: unknown): void {
  const failure = toFailure(error);
  if (failure.code === "locked" || failure.code === "stale") {
    options.onLock();
    return;
  }
  alertText(failure.message);
}

/** Up/Down move between rows, keeping the same action column. */
function moveFocus(from: HTMLElement, step: number): void {
  const tr = rowFor(from);
  if (!tr) return;
  const rows = [...rowsBody().rows];
  const next = rows[rows.indexOf(tr) + step];
  const action = from.dataset.action ?? "reveal";
  next?.querySelector<HTMLElement>(`[data-action="${action}"]`)?.focus();
}

function onSheetKey(event: KeyboardEvent): void {
  const target = event.target;
  if (!(target instanceof HTMLElement) || target instanceof HTMLInputElement) return;
  const tr = rowFor(target);
  if (!tr) return;
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    moveFocus(target, event.key === "ArrowDown" ? 1 : -1);
  } else if (event.key === "F2") {
    event.preventDefault();
    void onAction("edit", tr);
  } else if (event.key === "Delete") {
    event.preventDefault();
    void onAction("delete", tr);
  } else if (event.key === "c" && (event.ctrlKey || event.metaKey)) {
    if (window.getSelection()?.toString()) return;
    event.preventDefault();
    void onAction("copy", tr);
  }
}

function onGlobalKey(event: KeyboardEvent): void {
  touch();
  const inField =
    event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement;
  if (event.key === "/" && !inField && !document.querySelector("dialog[open]")) {
    event.preventDefault();
    byId("search", HTMLInputElement).focus();
  } else if (event.key.toLowerCase() === "l" && (event.ctrlKey || event.metaKey)) {
    event.preventDefault();
    options.onLock();
  }
}

export function showVault(initial: Snapshot, vaultOptions: VaultOptions): void {
  options = vaultOptions;
  snapshot = initial;
  revealed = undefined;
  const root = mount("tpl-vault");
  showShortcuts(true);

  const select = find(root, "#project", HTMLSelectElement);
  select.addEventListener("change", () => {
    if (select.value !== NEW_PROJECT) {
      project = select.value;
      hideRevealed();
      render();
      return;
    }
    askProjectName(
      (name) => {
        if (!pendingProjects.some((p) => sameName(p, name))) pendingProjects.push(name);
        project = name;
        render();
        byId("new-key", HTMLInputElement).focus();
        announce(`Project ${name} created. It is saved with its first key.`);
      },
      () => render(),
    );
  });

  const search = find(root, "#search", HTMLInputElement);
  search.addEventListener("input", () => render());
  search.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && search.value) {
      search.value = "";
      render();
    }
  });

  rowsBody().addEventListener("click", (event) => {
    const button = event.target instanceof Element ? event.target.closest("button") : null;
    const tr = rowFor(button);
    const action = button?.dataset.action;
    if (tr && isAction(action)) void onAction(action, tr);
  });
  rowsBody().addEventListener("keydown", onSheetKey);

  const keyInput = find(root, "#new-key", HTMLInputElement);
  const valueInput = find(root, "#new-value", HTMLInputElement);
  keyInput.addEventListener("input", updateAddLabel);
  for (const input of [keyInput, valueInput]) {
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        void addKey();
      }
    });
  }
  find(root, "#add", HTMLButtonElement).addEventListener("click", () => void addKey());

  find(root, "#lock", HTMLButtonElement).addEventListener("click", () => options.onLock());
  find(root, "#settings-open", HTMLButtonElement).addEventListener("click", () => {
    const projectCount = visibleRows(snapshot.rows, project, "").length;
    openSettings({
      vaultPath: options.vaultPath,
      project,
      projectCount,
      totalCount: snapshot.rows.length,
      projectTotal: snapshot.projects.length,
      onSnapshot: applySnapshot,
      onForgetProject: () => {
        pendingProjects = pendingProjects.filter((p) => !sameName(p, project));
        project = DEFAULT_PROJECT;
        render();
      },
    });
  });

  document.addEventListener("keydown", onGlobalKey);
  document.addEventListener("pointerdown", touch);
  applySnapshot(initial);
  keyInput.focus();
}

/** Tears down listeners and forgets every value this view held. */
export function leaveVault(): void {
  document.removeEventListener("keydown", onGlobalKey);
  document.removeEventListener("pointerdown", touch);
  for (const dialog of document.querySelectorAll("dialog")) dialog.close();
  revealed = undefined;
  editing = false;
  showShortcuts(false);
}

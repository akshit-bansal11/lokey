import assert from "node:assert/strict";
import { test } from "node:test";
import {
  findShortcut,
  hintText,
  type KeyLike,
  projectIndex,
  rowMoveKey,
  SHORTCUTS,
} from "./shortcuts.ts";

function key(k: string, extra: Partial<KeyLike> = {}): KeyLike {
  return { key: k, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...extra };
}
const ctrl = { ctrlKey: true };
const idOf = (event: KeyLike, scope: "global" | "row" | "add" = "global") =>
  findShortcut(event, scope)?.id;

test("ids are unique and every shortcut has display keys", () => {
  assert.equal(new Set(SHORTCUTS.map((s) => s.id)).size, SHORTCUTS.length);
  for (const s of SHORTCUTS) assert.ok(s.keys.length > 0 && s.description);
});

test("Ctrl+K opens the palette and no longer searches", () => {
  assert.equal(idOf(key("k", ctrl)), "palette");
  assert.equal(idOf(key("K", { metaKey: true })), "palette");
  assert.equal(idOf(key("f", ctrl)), "search");
  assert.equal(idOf(key("/")), "search");
});

test("Ctrl and Meta are both accepted", () => {
  assert.equal(idOf(key("g", { metaKey: true })), "generate");
  assert.equal(idOf(key("N", { ctrlKey: true, shiftKey: true })), "new-project");
  assert.equal(idOf(key("n", ctrl)), "add-key");
  assert.equal(idOf(key(",", ctrl)), "settings");
  assert.equal(idOf(key("l", ctrl)), "lock");
});

test("single-character keys never fire inside fields", () => {
  for (const tagName of ["INPUT", "SELECT", "TEXTAREA"]) {
    const target = { tagName };
    assert.equal(idOf(key("/", { target })), undefined);
    assert.equal(idOf(key("?", { target })), undefined);
  }
  assert.equal(idOf(key("?", { target: { isContentEditable: true } })), undefined);
  assert.equal(idOf(key("?", { target: { tagName: "BUTTON" } })), "help");
  assert.equal(idOf(key("k", { ...ctrl, target: { tagName: "INPUT" } })), "palette");
  assert.equal(idOf(key("F1", { target: { tagName: "INPUT" } })), "help");
});

test("no bare printable key triggers a global or row shortcut in a field", () => {
  const target = { tagName: "INPUT" };
  for (let code = 32; code < 127; code++) {
    const event = key(String.fromCharCode(code), { target });
    assert.equal(idOf(event), undefined, `key ${event.key}`);
    assert.equal(idOf(event, "row"), undefined, `row key ${event.key}`);
  }
});

test("project keys: PgUp/PgDn, Alt+arrows and Ctrl+1..9", () => {
  assert.equal(idOf(key("PageUp", ctrl)), "project-prev");
  assert.equal(idOf(key("ArrowUp", { altKey: true })), "project-prev");
  assert.equal(idOf(key("PageDown", ctrl)), "project-next");
  assert.equal(idOf(key("ArrowDown", { altKey: true })), "project-next");
  assert.equal(projectIndex(key("1", ctrl)), 0);
  assert.equal(projectIndex(key("9", { metaKey: true })), 8);
  assert.equal(projectIndex(key("0", ctrl)), undefined);
  assert.equal(projectIndex(key("1")), undefined);
  assert.equal(projectIndex(key("1", { ...ctrl, altKey: true })), undefined);
  assert.equal(idOf(key("3", ctrl)), "project-jump");
});

test("row keys need no modifiers; Alt+arrows are not row moves", () => {
  assert.equal(rowMoveKey(key("ArrowDown")), "ArrowDown");
  assert.equal(rowMoveKey(key("End")), "End");
  assert.equal(rowMoveKey(key("ArrowDown", { altKey: true })), undefined);
  assert.equal(idOf(key("F2"), "row"), "row-edit");
  assert.equal(idOf(key("Delete"), "row"), "row-delete");
  assert.equal(idOf(key("Escape"), "row"), "row-escape");
  assert.equal(idOf(key("c", ctrl), "row"), "row-copy");
  assert.equal(idOf(key("C", { ...ctrl, shiftKey: true }), "row"), "row-copy-name");
  assert.equal(idOf(key("c"), "row"), undefined);
});

test("Enter and Ctrl+Enter add a key from the add row", () => {
  assert.equal(idOf(key("Enter"), "add"), "add-submit");
  assert.equal(idOf(key("Enter", ctrl), "add"), "add-submit");
  assert.equal(idOf(key("Enter", { shiftKey: true }), "add"), undefined);
});

test("the status bar hint", () => {
  assert.equal(hintText(), "Ctrl+K commands · / search · Ctrl+L lock");
});

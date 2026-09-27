// The password generator dialog. Passwords come from lokey-core
// (generator.rs). Copying goes through the Rust clipboard code, like a saved
// value, so it stays out of Win+V history and is cleared after 30 seconds.

import { api, toFailure } from "@/lib/api.ts";
import { byId, find } from "@/lib/dom.ts";
import { startClipboardTimer } from "@/lib/status.ts";
import type { Charsets } from "@/lib/types.ts";

const dialog = byId("generate-dialog", HTMLDialogElement);
const output = byId("generated", HTMLElement);
const status = byId("generate-status", HTMLElement);
const error = byId("generate-error", HTMLElement);
const slider = byId("generate-length-slider", HTMLInputElement);
const lengthInput = byId("generate-length", HTMLInputElement);
const boxes = [...dialog.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')];

let current = "";
/** Numbers each request so a slow answer never replaces a newer one. */
let latest = 0;
let onUse: ((value: string) => void) | undefined;

function charsets(): Charsets {
  const on = (name: keyof Charsets) => find(dialog, `[name="${name}"]`, HTMLInputElement).checked;
  return { upper: on("upper"), lower: on("lower"), digits: on("digits"), symbols: on("symbols") };
}

/** At least one set stays chosen: the last checked box cannot be unchecked. */
function keepOneBox(): void {
  const checked = boxes.filter((box) => box.checked);
  for (const box of boxes) box.disabled = checked.length === 1 && box.checked;
}

async function regenerate(): Promise<void> {
  const ticket = ++latest;
  try {
    const next = await api.generate(slider.valueAsNumber, charsets());
    if (ticket !== latest) return;
    current = next.value;
    output.textContent = next.value;
    status.textContent = `About ${next.bits} bits of strength.`;
    error.textContent = "";
  } catch (failure) {
    if (ticket === latest) error.textContent = toFailure(failure).message;
  }
}

async function copyCurrent(): Promise<void> {
  if (!current) return;
  try {
    const seconds = await api.copyText(current);
    startClipboardTimer(seconds);
    status.textContent = `Copied. The clipboard clears in ${seconds} seconds.`;
  } catch (failure) {
    error.textContent = toFailure(failure).message;
  }
}

slider.addEventListener("input", () => {
  lengthInput.value = slider.value;
  void regenerate();
});

lengthInput.addEventListener("change", () => {
  // The slider clamps and rounds what it is given; an empty entry keeps the
  // last length.
  if (!Number.isNaN(lengthInput.valueAsNumber)) slider.value = lengthInput.value;
  lengthInput.value = slider.value;
  void regenerate();
});

for (const box of boxes) {
  box.addEventListener("change", () => {
    keepOneBox();
    void regenerate();
  });
}

byId("generate-again", HTMLButtonElement).addEventListener("click", () => void regenerate());
byId("generate-copy", HTMLButtonElement).addEventListener("click", () => void copyCurrent());
byId("generate-use", HTMLButtonElement).addEventListener("click", () => {
  const value = current;
  dialog.close();
  if (value) onUse?.(value);
});

// Selecting the password and pressing Ctrl+C would copy it the browser's way,
// into clipboard history; take that copy over. Checked on the dialog, since a
// selection dragged in from the title fires the event there, not on the value.
dialog.addEventListener("copy", (event) => {
  if (!document.getSelection()?.containsNode(output, true)) return;
  event.preventDefault();
  void copyCurrent();
});

dialog.addEventListener("close", () => {
  latest++;
  current = "";
  output.textContent = "";
  status.textContent = "";
  error.textContent = "";
});

/** Opens the generator; `use` receives the password for "Use as new key". */
export function openGenerator(use: (value: string) => void): void {
  onUse = use;
  keepOneBox();
  dialog.showModal();
  void regenerate();
}

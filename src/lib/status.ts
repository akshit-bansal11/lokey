// The status bar: one polite live region that exists from first paint
// (UI-D05), plus the clipboard countdown hairline.

import { byId } from "@/lib/dom.ts";

let clearTimer: number | undefined;

export function announce(message: string): void {
  const region = byId("status", HTMLElement);
  // Re-setting identical text is not announced; clear first so a repeat is.
  region.textContent = "";
  region.textContent = message;
}

/** Starts the hairline for a copy that clears in `seconds`. */
export function startClipboardTimer(seconds: number): void {
  const bar = byId("clip-timer", HTMLElement);
  window.clearTimeout(clearTimer);
  bar.hidden = false;
  bar.classList.remove("running");
  // Force a reflow so the animation restarts on a second copy.
  void bar.offsetWidth;
  bar.classList.add("running");
  clearTimer = window.setTimeout(() => {
    bar.hidden = true;
    bar.classList.remove("running");
    announce("Clipboard cleared.");
  }, seconds * 1000);
}

export function showShortcuts(visible: boolean): void {
  byId("shortcuts", HTMLElement).hidden = !visible;
}

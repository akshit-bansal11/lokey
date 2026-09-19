// Small DOM helpers. Views are HTML <template>s in index.html; user data is
// only ever written with textContent, never as markup.

import { fillIcons } from "@/lib/icons.ts";

/** A required element; a missing one is a bug in index.html, not a runtime state. */
export function byId<T extends HTMLElement>(id: string, type: new () => T): T {
  const element = document.getElementById(id);
  if (!(element instanceof type)) throw new Error(`index.html is missing #${id}`);
  return element;
}

export function find<T extends Element>(root: ParentNode, selector: string, type: new () => T): T {
  const element = root.querySelector(selector);
  if (!(element instanceof type)) throw new Error(`missing ${selector}`);
  return element;
}

/** Clones a view template into <main> and returns the new content. */
export function mount(templateId: string): HTMLElement {
  const template = byId(templateId, HTMLTemplateElement);
  const main = byId("main", HTMLElement);
  const fragment = template.content.cloneNode(true);
  if (!(fragment instanceof DocumentFragment)) throw new Error(`#${templateId} is not a template`);
  fillIcons(fragment);
  main.replaceChildren(fragment);
  return main;
}

export function field(form: HTMLFormElement, name: string): HTMLInputElement {
  return find(form, `[name="${name}"]`, HTMLInputElement);
}

/** Shows or clears the error next to a field (UX-17: adjacent, field named). */
export function setFieldError(form: HTMLFormElement, name: string, message: string): void {
  const input = field(form, name);
  const slot = form.querySelector<HTMLElement>(`[data-error-for="${name}"]`);
  if (slot) slot.textContent = message;
  if (message) input.setAttribute("aria-invalid", "true");
  else input.removeAttribute("aria-invalid");
}

export function setFormError(form: HTMLFormElement, message: string): void {
  const slot = form.querySelector<HTMLElement>("[data-form-error]");
  if (slot) slot.textContent = message;
}

/** Marks a button as working (UX-10) and returns a function that restores it. */
export function busy(button: HTMLButtonElement, label: string): () => void {
  const original = button.textContent ?? "";
  button.setAttribute("aria-busy", "true");
  button.disabled = true;
  button.textContent = label;
  return () => {
    button.removeAttribute("aria-busy");
    button.disabled = false;
    button.textContent = original;
  };
}

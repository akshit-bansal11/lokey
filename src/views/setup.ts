// First run: create the vault with a master password and choose whether a
// recovery key can also open it, or restore a backup instead.

import { api, toFailure } from "@/lib/api.ts";
import { busy, field, find, mount, setFieldError, setFormError } from "@/lib/dom.ts";
import type { Opened } from "@/lib/types.ts";

/** Checks one password field on blur, without deriving any key. */
export async function checkOnBlur(form: HTMLFormElement, name: string): Promise<void> {
  const value = field(form, name).value;
  if (value === "") return setFieldError(form, name, "");
  try {
    await api.checkPassword("master", value);
    setFieldError(form, name, "");
  } catch (failure) {
    setFieldError(form, name, toFailure(failure).message);
  }
}

/** Flags `master2` when it differs from `master`; true when they differ. */
export function confirmMismatch(form: HTMLFormElement): boolean {
  const differs = field(form, "master2").value !== field(form, "master").value;
  setFieldError(form, "master2", differs ? "The two passwords do not match." : "");
  return differs;
}

function wantsRecovery(form: HTMLFormElement): boolean {
  return find(form, 'input[name="recovery"][value="key"]', HTMLInputElement).checked;
}

type SetupHandlers = {
  onCreated: (opened: Opened) => void;
  onRestored: (opened: Opened) => void;
  /** Runs when a vault appeared meanwhile, say from a second window. */
  onExists?: () => void;
};

export function showSetup(handlers: SetupHandlers, notice = ""): void {
  const root = mount("tpl-setup");
  const form = find(root, "#setup-form", HTMLFormElement);
  const submit = find(form, 'button[type="submit"]', HTMLButtonElement);
  const acceptRow = find(form, "[data-accept]", HTMLElement);
  const accept = field(form, "accept");
  setFormError(form, notice);

  field(form, "master").addEventListener("blur", () => void checkOnBlur(form, "master"));
  field(form, "master2").addEventListener("blur", () => confirmMismatch(form));
  for (const radio of form.querySelectorAll<HTMLInputElement>('input[name="recovery"]')) {
    radio.addEventListener("change", () => {
      acceptRow.hidden = wantsRecovery(form);
      accept.checked = false;
      setFieldError(form, "accept", "");
    });
  }
  find(form, "[data-restore]", HTMLButtonElement).addEventListener(
    "click",
    () => void chooseBackup(handlers),
  );

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    setFormError(form, "");
    const master = field(form, "master").value;
    if (confirmMismatch(form)) return;
    const recovery = wantsRecovery(form);
    if (!recovery && !accept.checked) {
      setFieldError(form, "accept", "Tick the box to confirm there is no way back in.");
      accept.focus();
      return;
    }
    const restore = busy(submit, "Creating vault…");
    try {
      handlers.onCreated(await api.create(master, recovery));
    } catch (failure) {
      restore();
      const { code, message } = toFailure(failure);
      if (code === "exists" && handlers.onExists) {
        handlers.onExists();
        return;
      }
      setFormError(form, message);
    }
  });

  field(form, "master").focus();
}

/** Opens the Windows Open dialog, then asks for the chosen backup's password. */
async function chooseBackup(handlers: SetupHandlers): Promise<void> {
  try {
    const path = await api.chooseBackup();
    if (path) showRestore(path, handlers);
  } catch (failure) {
    showSetup(handlers, toFailure(failure).message);
  }
}

function showRestore(path: string, handlers: SetupHandlers): void {
  const root = mount("tpl-restore");
  const form = find(root, "#restore-form", HTMLFormElement);
  const submit = find(form, 'button[type="submit"]', HTMLButtonElement);
  const input = field(form, "master");
  find(form, "[data-backup-path]", HTMLElement).textContent = path;
  find(form, "[data-choose]", HTMLButtonElement).addEventListener(
    "click",
    () => void chooseBackup(handlers),
  );
  find(form, "[data-back]", HTMLButtonElement).addEventListener("click", () => showSetup(handlers));

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    setFormError(form, "");
    if (input.value === "") {
      setFormError(form, "Enter the backup's master password.");
      return;
    }
    const restore = busy(submit, "Restoring…");
    try {
      const opened = await api.restore(input.value);
      input.value = "";
      handlers.onRestored(opened);
    } catch (failure) {
      restore();
      const { code, message } = toFailure(failure);
      if (code === "exists" && handlers.onExists) {
        handlers.onExists();
        return;
      }
      setFormError(form, message);
      input.select();
    }
  });

  input.focus();
}

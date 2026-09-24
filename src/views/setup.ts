// First run: create the vault with a master password.

import { api, toFailure } from "@/lib/api.ts";
import { busy, field, find, mount, setFieldError, setFormError } from "@/lib/dom.ts";
import type { Snapshot } from "@/lib/types.ts";

/** Checks one password field on blur, without deriving any key. */
async function checkOnBlur(form: HTMLFormElement, name: "master"): Promise<void> {
  const value = field(form, name).value;
  if (value === "") return setFieldError(form, name, "");
  try {
    await api.checkPassword(name, value);
    setFieldError(form, name, "");
  } catch (failure) {
    setFieldError(form, name, toFailure(failure).message);
  }
}

function confirmMismatch(form: HTMLFormElement): boolean {
  const differs = field(form, "master2").value !== field(form, "master").value;
  setFieldError(form, "master2", differs ? "The two passwords do not match." : "");
  return differs;
}

/** `onExists` runs when a vault appeared meanwhile, say from a second window. */
export function showSetup(
  onCreated: (snapshot: Snapshot) => void,
  notice = "",
  onExists?: () => void,
): void {
  const root = mount("tpl-setup");
  const form = find(root, "#setup-form", HTMLFormElement);
  const submit = find(form, 'button[type="submit"]', HTMLButtonElement);
  setFormError(form, notice);

  field(form, "master").addEventListener("blur", () => void checkOnBlur(form, "master"));
  field(form, "master2").addEventListener("blur", () => confirmMismatch(form));

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    setFormError(form, "");
    const master = field(form, "master").value;
    if (confirmMismatch(form)) return;
    const restore = busy(submit, "Creating vault…");
    try {
      onCreated(await api.create(master));
    } catch (failure) {
      restore();
      const { code, message } = toFailure(failure);
      if (code === "exists" && onExists) {
        onExists();
        return;
      }
      setFormError(form, message);
    }
  });

  field(form, "master").focus();
}

// First run: create the vault with a master and a deletion password.

import { api, toFailure } from "@/lib/api.ts";
import { busy, field, find, mount, setFieldError, setFormError } from "@/lib/dom.ts";
import type { Snapshot } from "@/lib/types.ts";

/** Checks one password field on blur, without deriving any key. */
async function checkOnBlur(form: HTMLFormElement, name: "master" | "deletion"): Promise<void> {
  const value = field(form, name).value;
  if (value === "") return setFieldError(form, name, "");
  try {
    await api.checkPassword(name, value);
    setFieldError(form, name, "");
  } catch (failure) {
    setFieldError(form, name, toFailure(failure).message);
  }
}

function confirmMismatch(
  form: HTMLFormElement,
  name: string,
  again: string,
  label: string,
): boolean {
  const differs = field(form, again).value !== field(form, name).value;
  setFieldError(form, again, differs ? `The two ${label} passwords do not match.` : "");
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
  field(form, "deletion").addEventListener("blur", () => void checkOnBlur(form, "deletion"));
  field(form, "master2").addEventListener("blur", () =>
    confirmMismatch(form, "master", "master2", "master"),
  );
  field(form, "deletion2").addEventListener("blur", () =>
    confirmMismatch(form, "deletion", "deletion2", "deletion"),
  );

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    setFormError(form, "");
    const master = field(form, "master").value;
    const deletion = field(form, "deletion").value;
    const mismatch = [
      confirmMismatch(form, "master", "master2", "master"),
      confirmMismatch(form, "deletion", "deletion2", "deletion"),
    ].some(Boolean);
    if (mismatch) return;
    if (master === deletion) {
      setFieldError(
        form,
        "deletion",
        "The deletion password must differ from the master password.",
      );
      return;
    }
    const restore = busy(submit, "Creating vault…");
    try {
      onCreated(await api.create(master, deletion));
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

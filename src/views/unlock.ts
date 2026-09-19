// Every launch after the first: the master password opens the vault.

import { api, toFailure } from "@/lib/api.ts";
import { busy, field, find, mount, setFormError } from "@/lib/dom.ts";
import { announce } from "@/lib/status.ts";
import type { LockReason, Snapshot } from "@/lib/types.ts";

const REASONS: Record<LockReason, string> = {
  idle: "Locked after 15 minutes without activity.",
  stale:
    "The passwords were changed from the terminal or another window. Unlock with the new master password.",
  missing: "The vault file was removed while it was open.",
};

function lockoutMessage(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  return `Too many wrong passwords. Try again in ${minutes}m ${seconds % 60}s.`;
}

export function showUnlock(
  vaultPath: string,
  lockoutSecs: number,
  onUnlocked: (snapshot: Snapshot) => void,
  reason?: LockReason,
): void {
  const root = mount("tpl-unlock");
  const form = find(root, "#unlock-form", HTMLFormElement);
  const submit = find(form, 'button[type="submit"]', HTMLButtonElement);
  const input = field(form, "master");
  find(form, "[data-reason]", HTMLElement).textContent = reason ? REASONS[reason] : "";
  // Also spoken: focus lands in the field, so a screen reader user would
  // otherwise never hear why the vault locked (SC 4.1.3).
  if (reason) announce(REASONS[reason]);
  find(form, "[data-vault-path]", HTMLElement).textContent = vaultPath;

  if (lockoutSecs > 0) {
    // UX-13: the control is disabled, so the reason is shown beside it.
    setFormError(form, lockoutMessage(lockoutSecs));
    submit.disabled = true;
    window.setTimeout(() => {
      submit.disabled = false;
      setFormError(form, "");
    }, lockoutSecs * 1000);
  }

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    setFormError(form, "");
    if (input.value === "") {
      setFormError(form, "Enter your master password.");
      return;
    }
    const restore = busy(submit, "Unlocking…");
    try {
      const snapshot = await api.unlock(input.value);
      input.value = "";
      onUnlocked(snapshot);
    } catch (error) {
      restore();
      const failure = toFailure(error);
      setFormError(form, failure.message);
      input.select();
    }
  });

  input.focus();
}

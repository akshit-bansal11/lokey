// Every launch after the first: the master password opens the vault, or the
// recovery key does and sets a new master password.

import { api, toFailure } from "@/lib/api.ts";
import { busy, field, find, mount, setFieldError, setFormError } from "@/lib/dom.ts";
import { announce } from "@/lib/status.ts";
import type { LockReason, Opened, Snapshot, Status } from "@/lib/types.ts";
import { checkOnBlur, confirmMismatch } from "@/views/setup.ts";

const REASONS: Record<LockReason, string> = {
  idle: "Locked after 5 minutes without activity.",
  stale: "The master password was changed in another window. Unlock with the new one.",
  missing: "The vault file was removed while it was open.",
};

function lockoutMessage(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  return `Too many wrong attempts. Try again in ${minutes}m ${seconds % 60}s.`;
}

/** UX-13: the control is disabled, so the reason is shown beside it. */
function holdForLockout(form: HTMLFormElement, submit: HTMLButtonElement, seconds: number): void {
  if (seconds <= 0) return;
  setFormError(form, lockoutMessage(seconds));
  submit.disabled = true;
  window.setTimeout(() => {
    submit.disabled = false;
    setFormError(form, "");
  }, seconds * 1000);
}

type UnlockHandlers = {
  onUnlocked: (snapshot: Snapshot) => void;
  onRecovered: (opened: Opened) => void;
};

export function showUnlock(status: Status, handlers: UnlockHandlers, reason?: LockReason): void {
  const root = mount("tpl-unlock");
  const form = find(root, "#unlock-form", HTMLFormElement);
  const submit = find(form, 'button[type="submit"]', HTMLButtonElement);
  const input = field(form, "master");
  const forgot = find(form, "[data-forgot]", HTMLButtonElement);
  find(form, "[data-reason]", HTMLElement).textContent = reason ? REASONS[reason] : "";
  // Also spoken: focus lands in the field, so a screen reader user would
  // otherwise never hear why the vault locked (SC 4.1.3).
  if (reason) announce(REASONS[reason]);
  find(form, "[data-vault-path]", HTMLElement).textContent = status.vaultPath;
  forgot.hidden = !status.recovery;
  forgot.addEventListener("click", () => showRecover(status, handlers));
  holdForLockout(form, submit, status.lockoutSecs);

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
      handlers.onUnlocked(snapshot);
    } catch (error) {
      restore();
      const failure = toFailure(error);
      setFormError(form, failure.message);
      input.select();
    }
  });

  input.focus();
}

function recoverFailureMessage(code: string, message: string): string {
  return code === "wrong-password"
    ? message.replace(/^Wrong password/, "That recovery key does not open this vault")
    : message;
}

function showRecover(status: Status, handlers: UnlockHandlers): void {
  const root = mount("tpl-recover");
  const form = find(root, "#recover-form", HTMLFormElement);
  const submit = find(form, 'button[type="submit"]', HTMLButtonElement);
  const keyInput = field(form, "recoveryKey");
  find(form, "[data-back]", HTMLButtonElement).addEventListener("click", () =>
    showUnlock(status, handlers),
  );
  field(form, "master").addEventListener("blur", () => void checkOnBlur(form, "master"));
  field(form, "master2").addEventListener("blur", () => confirmMismatch(form));
  holdForLockout(form, submit, status.lockoutSecs);

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    setFormError(form, "");
    setFieldError(form, "recoveryKey", "");
    if (keyInput.value.trim() === "") {
      setFieldError(form, "recoveryKey", "Enter the recovery key you wrote down.");
      keyInput.focus();
      return;
    }
    if (confirmMismatch(form)) return;
    const restore = busy(submit, "Recovering…");
    try {
      const opened = await api.recover(keyInput.value, field(form, "master").value);
      form.reset();
      handlers.onRecovered(opened);
    } catch (error) {
      restore();
      const { code, message } = toFailure(error);
      if (code === "bad-recovery-key") {
        setFieldError(form, "recoveryKey", message);
        keyInput.focus();
      } else if (code === "weak-password") {
        setFieldError(form, "master", message);
      } else {
        setFormError(form, recoverFailureMessage(code, message));
      }
    }
  });

  keyInput.focus();
}

// The <dialog>s declared in index.html. Native showModal() supplies the
// focus trap, Escape to close, the inert background and focus return (UI-10).

import { api, toFailure } from "@/lib/api.ts";
import { busy, byId, field, find, setFieldError, setFormError } from "@/lib/dom.ts";
import { fillIcons } from "@/lib/icons.ts";
import { sameRecoveryKey } from "@/lib/recovery.ts";
import { nameProblem } from "@/lib/rows.ts";
import { announce } from "@/lib/status.ts";
import type { Snapshot } from "@/lib/types.ts";

// Dialogs live in the page, not in a template, so mount() never fills their
// icon placeholders; fill them once here.
for (const dialog of document.querySelectorAll("dialog")) {
  fillIcons(dialog);
  for (const close of dialog.querySelectorAll<HTMLButtonElement>("[data-close]")) {
    close.addEventListener("click", () => dialog.close());
  }
}

type DeleteRequest = {
  title: string;
  text: string;
  confirm: string;
  run: () => Promise<Snapshot>;
  onDone: (snapshot: Snapshot) => void;
};

let pendingDelete: DeleteRequest | undefined;

const deleteDialog = byId("delete-dialog", HTMLDialogElement);
const deleteForm = byId("delete-form", HTMLFormElement);
const deleteButton = byId("delete-confirm", HTMLButtonElement);

deleteForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const request = pendingDelete;
  if (!request) return;
  setFormError(deleteForm, "");
  const restore = busy(deleteButton, "Deleting…");
  try {
    const snapshot = await request.run();
    restore();
    deleteDialog.close();
    request.onDone(snapshot);
  } catch (error) {
    restore();
    setFormError(deleteForm, toFailure(error).message);
  }
});

deleteDialog.addEventListener("close", () => {
  pendingDelete = undefined;
});

/**
 * Every delete names its object and consequence and waits for a confirm. The
 * maintainer chose this over an undo window or typing the name (UX-05), on
 * 2026-09-24, when the deletion password was removed.
 */
export function confirmDelete(request: DeleteRequest): void {
  pendingDelete = request;
  byId("delete-title", HTMLElement).textContent = request.title;
  byId("delete-text", HTMLElement).textContent = request.text;
  deleteButton.textContent = request.confirm;
  setFormError(deleteForm, "");
  deleteDialog.showModal();
  deleteButton.focus();
}

const projectDialog = byId("project-dialog", HTMLDialogElement);
const projectForm = byId("project-form", HTMLFormElement);
let onProjectCreated: ((name: string) => void) | undefined;
let projectCreated = false;

projectForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const name = field(projectForm, "name").value.trim();
  const problem = nameProblem("project", name);
  setFieldError(projectForm, "name", problem);
  if (problem) return;
  projectCreated = true;
  projectDialog.close();
  onProjectCreated?.(name);
});

export function askProjectName(onCreated: (name: string) => void, onCancel: () => void): void {
  onProjectCreated = onCreated;
  projectCreated = false;
  const input = field(projectForm, "name");
  input.value = "";
  setFieldError(projectForm, "name", "");
  projectDialog.addEventListener(
    "close",
    () => {
      if (!projectCreated) onCancel();
    },
    { once: true },
  );
  projectDialog.showModal();
  input.focus();
}

type MasterPrompt = { title: string; text: string; action: string; busyLabel: string };

const confirmMasterDialog = byId("confirm-master-dialog", HTMLDialogElement);
const confirmMasterForm = byId("confirm-master-form", HTMLFormElement);
const confirmMasterSubmit = byId("confirm-master-submit", HTMLButtonElement);
const confirmMasterCancel = find(confirmMasterForm, "[data-close]", HTMLButtonElement);
let pendingMaster: { run: (master: string) => Promise<void>; busyLabel: string } | undefined;
let masterRunning = false;

// Once the password is sent, the change goes through whatever happens to the
// dialog. Closing it then would drop the result, such as a new recovery key
// whose predecessor already stopped working, so it stays open until done.
confirmMasterDialog.addEventListener("cancel", (event) => {
  if (masterRunning) event.preventDefault();
});

confirmMasterForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const pending = pendingMaster;
  const input = field(confirmMasterForm, "master");
  if (!pending) return;
  if (input.value === "") {
    setFormError(confirmMasterForm, "Enter your master password.");
    return;
  }
  setFormError(confirmMasterForm, "");
  const restore = busy(confirmMasterSubmit, pending.busyLabel);
  masterRunning = true;
  confirmMasterCancel.disabled = true;
  try {
    await pending.run(input.value);
    confirmMasterDialog.close();
  } catch (error) {
    setFormError(confirmMasterForm, toFailure(error).message);
    input.select();
  } finally {
    masterRunning = false;
    confirmMasterCancel.disabled = false;
    restore();
  }
});

confirmMasterDialog.addEventListener("close", () => {
  pendingMaster = undefined;
  field(confirmMasterForm, "master").value = "";
  setFormError(confirmMasterForm, "");
});

/**
 * Asks for the master password again, then runs `run` with it. Resolves to
 * what `run` returned once the dialog closes, or undefined when cancelled.
 * Used before anything that adds a way in or takes a copy of the vault away.
 */
function askMaster<T>(
  prompt: MasterPrompt,
  run: (master: string) => Promise<T>,
): Promise<T | undefined> {
  return new Promise((resolve) => {
    let result: T | undefined;
    pendingMaster = {
      busyLabel: prompt.busyLabel,
      run: async (master) => {
        result = await run(master);
      },
    };
    confirmMasterDialog.addEventListener("close", () => resolve(result), { once: true });
    byId("confirm-master-title", HTMLElement).textContent = prompt.title;
    byId("confirm-master-text", HTMLElement).textContent = prompt.text;
    confirmMasterSubmit.textContent = prompt.action;
    confirmMasterDialog.showModal();
    field(confirmMasterForm, "master").focus();
  });
}

const recoveryDialog = byId("recovery-dialog", HTMLDialogElement);
const recoveryForm = byId("recovery-form", HTMLFormElement);
const recoveryKeyText = byId("recovery-key", HTMLElement);
let shownKey = "";

// The key is shown once, so Escape must not close the dialog before it is saved.
recoveryDialog.addEventListener("cancel", (event) => event.preventDefault());

recoveryForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const typed = field(recoveryForm, "typed");
  if (!sameRecoveryKey(typed.value, shownKey)) {
    setFieldError(
      recoveryForm,
      "typed",
      "That does not match the key above. Check each group of four.",
    );
    typed.focus();
    return;
  }
  recoveryDialog.close();
});

recoveryDialog.addEventListener("close", () => {
  // Out of the page as soon as it is confirmed.
  shownKey = "";
  recoveryKeyText.textContent = "";
  field(recoveryForm, "typed").value = "";
  setFieldError(recoveryForm, "typed", "");
});

/** Shows a new recovery key once, and resolves when it is typed back. */
export function showRecoveryKey(key: string, text: string): Promise<void> {
  return new Promise((resolve) => {
    shownKey = key;
    recoveryKeyText.textContent = key;
    byId("recovery-text", HTMLElement).textContent = text;
    recoveryDialog.addEventListener("close", () => resolve(), { once: true });
    recoveryDialog.showModal();
    field(recoveryForm, "typed").focus();
  });
}

const settingsDialog = byId("settings-dialog", HTMLDialogElement);
const masterForm = byId("master-form", HTMLFormElement);

masterForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const master = field(masterForm, "master").value;
  let problem = "";
  try {
    await api.checkPassword("master", master);
  } catch (error) {
    problem = toFailure(error).message;
  }
  setFieldError(masterForm, "master", problem);
  const mismatch = field(masterForm, "master2").value !== master;
  setFieldError(masterForm, "master2", mismatch ? "The two passwords do not match." : "");
  if (problem || mismatch) return;
  const outcome = await askMaster(
    {
      title: "Change master password",
      text: "Enter the current master password to set the new one.",
      action: "Change password",
      busyLabel: "Changing…",
    },
    async (current) => ({ key: await api.changeMaster(current, master) }),
  );
  if (!outcome) return;
  masterForm.reset();
  setFormError(masterForm, "");
  announce("Master password changed. Use the new one next time you unlock.");
  if (outcome.key) {
    await showRecoveryKey(
      outcome.key,
      "The master password changed, so the vault has a new recovery key. The old one no longer works.",
    );
  }
});

settingsDialog.addEventListener("close", () => {
  masterForm.reset();
  setFormError(masterForm, "");
});

type SettingsContext = {
  vaultPath: string;
  project: string;
  projectCount: number;
  totalCount: number;
  projectTotal: number;
  onSnapshot: (snapshot: Snapshot) => void;
  onForgetProject: () => void;
};

function plural(count: number, word: string): string {
  return `${count} ${word}${count === 1 ? "" : "s"}`;
}

let recoveryOn = false;

/** Reads whether a recovery key opens the vault and shows the matching buttons. */
async function refreshRecoveryState(): Promise<void> {
  try {
    recoveryOn = (await api.status()).recovery;
  } catch (error) {
    announce(toFailure(error).message);
    return;
  }
  byId("recovery-state", HTMLElement).textContent = recoveryOn
    ? "On. The master password or the recovery key you wrote down opens this vault."
    : "Off. Only the master password opens this vault. Forget it and everything is lost.";
  byId("recovery-new", HTMLButtonElement).textContent = recoveryOn
    ? "Make a new recovery key"
    : "Turn on recovery key";
  byId("recovery-off", HTMLButtonElement).hidden = !recoveryOn;
}

async function exportBackup(): Promise<void> {
  const written = await askMaster(
    {
      title: "Export a backup",
      text: "Enter the master password, then choose where to save the encrypted copy.",
      action: "Choose location…",
      busyLabel: "Exporting…",
    },
    async (master) => ({ path: await api.exportBackup(master) }),
  );
  if (!written) return;
  announce(written.path ? `Backup saved to ${written.path}.` : "Export cancelled.");
}

async function newRecoveryKey(): Promise<void> {
  const wasOn = recoveryOn;
  const issued = await askMaster(
    {
      title: wasOn ? "Make a new recovery key" : "Turn on recovery key",
      text: wasOn
        ? "The current recovery key stops working, and backups exported from now on need the new one."
        : "You get a key to write down. It opens the vault if you forget the master password.",
      action: wasOn ? "Make new key" : "Turn on",
      busyLabel: "Making a key…",
    },
    (master) => api.setRecovery(master, true),
  );
  if (!issued) return;
  await refreshRecoveryState();
  await showRecoveryKey(
    issued,
    wasOn ? "Your previous recovery key no longer works." : "Recovery is on.",
  );
  announce("New recovery key saved.");
}

async function removeRecoveryKey(): Promise<void> {
  const removed = await askMaster(
    {
      title: "Remove the recovery key?",
      text: "Only the master password will open the vault. If you forget it, everything in the vault is lost.",
      action: "Remove recovery key",
      busyLabel: "Removing…",
    },
    async (master) => {
      await api.setRecovery(master, false);
      return true;
    },
  );
  if (!removed) return;
  await refreshRecoveryState();
  announce("Recovery key removed. Only the master password opens the vault now.");
}

byId("export", HTMLButtonElement).addEventListener("click", () => void exportBackup());
byId("recovery-new", HTMLButtonElement).addEventListener("click", () => void newRecoveryKey());
byId("recovery-off", HTMLButtonElement).addEventListener("click", () => void removeRecoveryKey());

export function openSettings(context: SettingsContext): void {
  byId("settings-path", HTMLElement).textContent = context.vaultPath;
  void refreshRecoveryState();
  byId("delete-project-label", HTMLElement).textContent = `Delete project ${context.project}`;
  const deleteProject = byId("delete-project", HTMLButtonElement);
  const deleteAll = byId("delete-all", HTMLButtonElement);

  deleteProject.onclick = () => {
    if (context.projectCount === 0) {
      // A project with no saved keys exists only in this window.
      settingsDialog.close();
      context.onForgetProject();
      announce(`Removed the empty project ${context.project}.`);
      return;
    }
    // Opened on top of Settings, so Cancel returns focus inside it.
    confirmDelete({
      title: `Delete project ${context.project}?`,
      text: `Deletes ${context.project} and its ${plural(context.projectCount, "key")}. This cannot be undone.`,
      confirm: "Delete project",
      run: () => api.deleteProject(context.project),
      onDone: (snapshot) => {
        settingsDialog.close();
        context.onSnapshot(snapshot);
        announce(
          `Deleted project ${context.project} and its ${plural(context.projectCount, "key")}.`,
        );
      },
    });
  };

  deleteAll.onclick = () => {
    if (context.totalCount === 0) {
      announce("There are no keys to delete.");
      return;
    }
    // Opened on top of Settings, so Cancel returns focus inside it.
    confirmDelete({
      title: "Delete all keys?",
      text: `Deletes all ${plural(context.totalCount, "key")} in ${plural(context.projectTotal, "project")}. The vault and its password stay. This cannot be undone.`,
      confirm: "Delete all keys",
      run: () => api.truncate(),
      onDone: (snapshot) => {
        settingsDialog.close();
        context.onSnapshot(snapshot);
        announce(`Deleted ${plural(context.totalCount, "key")}. The vault is empty.`);
      },
    });
  };

  settingsDialog.showModal();
}

/** The shortcuts list is static markup; opening it is all there is. */
export function showKeys(): void {
  byId("keys-dialog", HTMLDialogElement).showModal();
}

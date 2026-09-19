// The three <dialog>s declared in index.html. Native showModal() supplies the
// focus trap, Escape to close, the inert background and focus return (UI-10).

import { api, toFailure } from "@/lib/api.ts";
import { busy, byId, field, find, setFieldError, setFormError } from "@/lib/dom.ts";
import { nameProblem } from "@/lib/rows.ts";
import { announce } from "@/lib/status.ts";
import type { Snapshot } from "@/lib/types.ts";

for (const dialog of document.querySelectorAll("dialog")) {
  for (const close of dialog.querySelectorAll<HTMLButtonElement>("[data-close]")) {
    close.addEventListener("click", () => dialog.close());
  }
}

type DeleteRequest = {
  title: string;
  text: string;
  confirm: string;
  run: (deletion: string) => Promise<Snapshot>;
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
  const input = field(deleteForm, "deletion");
  if (input.value === "") {
    setFormError(deleteForm, "Enter the deletion password.");
    return;
  }
  setFormError(deleteForm, "");
  const restore = busy(deleteButton, "Deleting…");
  try {
    const snapshot = await request.run(input.value);
    restore();
    deleteDialog.close();
    request.onDone(snapshot);
  } catch (error) {
    restore();
    setFormError(deleteForm, toFailure(error).message);
    input.select();
  }
});

deleteDialog.addEventListener("close", () => {
  field(deleteForm, "deletion").value = "";
  pendingDelete = undefined;
});

/** Every delete names its object and consequence, and needs the deletion password. */
export function confirmDelete(request: DeleteRequest): void {
  pendingDelete = request;
  byId("delete-title", HTMLElement).textContent = request.title;
  byId("delete-text", HTMLElement).textContent = request.text;
  deleteButton.textContent = request.confirm;
  setFormError(deleteForm, "");
  deleteDialog.showModal();
  field(deleteForm, "deletion").focus();
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

const settingsDialog = byId("settings-dialog", HTMLDialogElement);
const masterForm = byId("master-form", HTMLFormElement);
const deletionForm = byId("deletion-form", HTMLFormElement);

async function policyError(label: string, password: string): Promise<string> {
  try {
    await api.checkPassword(label, password);
    return "";
  } catch (error) {
    return toFailure(error).message;
  }
}

masterForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const master = field(masterForm, "master").value;
  const problem = await policyError("master", master);
  setFieldError(masterForm, "master", problem);
  const mismatch = field(masterForm, "master2").value !== master;
  setFieldError(masterForm, "master2", mismatch ? "The two passwords do not match." : "");
  if (problem || mismatch) return;
  const restore = busy(find(masterForm, 'button[type="submit"]', HTMLButtonElement), "Changing…");
  try {
    await api.changePasswords({ newMaster: master });
    masterForm.reset();
    setFormError(masterForm, "");
    announce("Master password changed. Use the new one next time you unlock.");
  } catch (error) {
    setFormError(masterForm, toFailure(error).message);
  } finally {
    restore();
  }
});

deletionForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const current = field(deletionForm, "current").value;
  const next = field(deletionForm, "deletion").value;
  const problem = await policyError("deletion", next);
  setFieldError(deletionForm, "deletion", problem);
  const mismatch = field(deletionForm, "deletion2").value !== next;
  setFieldError(deletionForm, "deletion2", mismatch ? "The two passwords do not match." : "");
  if (problem || mismatch) return;
  if (current === "") {
    setFormError(deletionForm, "Enter the current deletion password.");
    return;
  }
  const restore = busy(find(deletionForm, 'button[type="submit"]', HTMLButtonElement), "Changing…");
  try {
    await api.changePasswords({ currentDeletion: current, newDeletion: next });
    deletionForm.reset();
    setFormError(deletionForm, "");
    announce("Deletion password changed.");
  } catch (error) {
    setFormError(deletionForm, toFailure(error).message);
  } finally {
    restore();
  }
});

settingsDialog.addEventListener("close", () => {
  masterForm.reset();
  deletionForm.reset();
  for (const form of [masterForm, deletionForm]) setFormError(form, "");
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

export function openSettings(context: SettingsContext): void {
  byId("settings-path", HTMLElement).textContent = context.vaultPath;
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
    settingsDialog.close();
    confirmDelete({
      title: `Delete project ${context.project}?`,
      text: `Deletes ${context.project} and its ${plural(context.projectCount, "key")}. This cannot be undone.`,
      confirm: "Delete project",
      run: (deletion) => api.deleteProject(context.project, deletion),
      onDone: (snapshot) => {
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
    settingsDialog.close();
    confirmDelete({
      title: "Delete all keys?",
      text: `Deletes all ${plural(context.totalCount, "key")} in ${plural(context.projectTotal, "project")}. The vault and its passwords stay. This cannot be undone.`,
      confirm: "Delete all keys",
      run: (deletion) => api.truncate(deletion),
      onDone: (snapshot) => {
        context.onSnapshot(snapshot);
        announce(`Deleted ${plural(context.totalCount, "key")}. The vault is empty.`);
      },
    });
  };

  settingsDialog.showModal();
}

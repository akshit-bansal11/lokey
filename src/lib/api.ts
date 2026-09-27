// The only module that talks to the Rust side. Every call is typed here, and
// every rejection is narrowed to a Failure before a view sees it.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Failure, LockReason, Opened, Snapshot, Status } from "@/lib/types.ts";

const FAILURE_CODES = new Set([
  "wrong-password",
  "locked-out",
  "stale",
  "no-recovery",
  "bad-recovery-key",
  "no-vault",
  "exists",
  "not-found",
  "invalid-name",
  "weak-password",
  "too-large",
  "corrupt",
  "unsupported",
  "io",
  "locked",
]);

function isFailure(value: unknown): value is Failure {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value &&
    typeof value.code === "string" &&
    FAILURE_CODES.has(value.code) &&
    typeof value.message === "string"
  );
}

/** lokey-core writes terse lowercase messages; the app shows them as sentences. */
function sentence(message: string): string {
  const text = message.trim();
  const capital = text.charAt(0).toUpperCase() + text.slice(1);
  return /[.!?]$/.test(capital) ? capital : `${capital}.`;
}

/** Anything thrown by `invoke` becomes a Failure with a readable message. */
export function toFailure(error: unknown): Failure {
  if (isFailure(error)) return { ...error, message: sentence(error.message) };
  const message = error instanceof Error ? error.message : String(error);
  return { code: "io", message: sentence(message), waitMs: 0 };
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toFailure(error);
  }
}

export const api = {
  status: () => call<Status>("status"),
  checkPassword: (label: string, password: string) =>
    call<null>("check_password", { label, password }),
  create: (master: string, recovery: boolean) => call<Opened>("create", { master, recovery }),
  unlock: (master: string) => call<Snapshot>("unlock", { master }),
  recover: (recoveryKey: string, newMaster: string) =>
    call<Opened>("recover", { recoveryKey, newMaster }),
  /** Shows the Open dialog; resolves to the chosen path, or null if cancelled. */
  chooseBackup: () => call<string | null>("choose_backup"),
  restore: (master: string) => call<Opened>("restore", { master }),
  /** Shows the Save dialog; resolves to the path written, or null if cancelled. */
  exportBackup: (master: string) => call<string | null>("export_backup", { master }),
  lock: () => call<null>("lock"),
  touch: () => call<null>("touch"),
  reveal: (project: string, key: string) => call<string>("reveal", { project, key }),
  copy: (project: string, key: string) => call<number>("copy", { project, key }),
  save: (project: string, key: string, value: string) =>
    call<Snapshot>("save", { project, key, value }),
  deleteKey: (project: string, key: string) => call<Snapshot>("delete_key", { project, key }),
  deleteProject: (project: string) => call<Snapshot>("delete_project", { project }),
  truncate: () => call<Snapshot>("truncate"),
  /** Resolves to the replacement recovery key when the vault has one. */
  changeMaster: (currentMaster: string, newMaster: string) =>
    call<string | null>("change_master", { currentMaster, newMaster }),
  /** Resolves to the new recovery key when turning it on. */
  setRecovery: (master: string, on: boolean) => call<string | null>("set_recovery", { master, on }),
};

export function onVaultChanged(handler: (snapshot: Snapshot) => void): Promise<UnlistenFn> {
  return listen<Snapshot>("vault-changed", (event) => handler(event.payload));
}

export function onVaultLocked(handler: (reason: LockReason) => void): Promise<UnlistenFn> {
  return listen<LockReason>("vault-locked", (event) => handler(event.payload));
}

export function onVaultError(handler: (message: string) => void): Promise<UnlistenFn> {
  return listen<string>("vault-error", (event) => handler(event.payload));
}

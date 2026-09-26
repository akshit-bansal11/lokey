// Entry point: picks the first screen and follows the vault's lock state.

import { api, onVaultChanged, onVaultError, onVaultLocked, toFailure } from "@/lib/api.ts";
import { announce } from "@/lib/status.ts";
import type { LockReason, Opened, Snapshot, Status } from "@/lib/types.ts";
import { showRecoveryKey } from "@/views/dialogs.ts";
import { showSetup } from "@/views/setup.ts";
import { showUnlock } from "@/views/unlock.ts";
import { applySnapshot, leaveVault, showVault } from "@/views/vault.ts";

let inVault = false;

function enterVault(status: Status, snapshot: Snapshot): void {
  inVault = true;
  showVault(snapshot, {
    vaultPath: status.vaultPath,
    onLock: () => {
      void api.lock().finally(() => void route("manual"));
    },
  });
}

/** Enters a vault that was just created or recovered, then shows its new key. */
function enterOpened(status: Status, opened: Opened, text: string): void {
  enterVault(status, opened.snapshot);
  if (opened.recoveryKey) void showRecoveryKey(opened.recoveryKey, text);
}

async function route(reason?: LockReason | "manual"): Promise<void> {
  if (inVault) {
    leaveVault();
    inVault = false;
  }
  const status = await api.status();
  if (!status.vaultPath) {
    showSetup(
      { onCreated: () => undefined, onRestored: () => undefined },
      "lokey cannot find your local app data folder (LOCALAPPDATA).",
    );
    return;
  }
  if (!status.exists) {
    showSetup(
      {
        onCreated: (opened) =>
          enterOpened(status, opened, "It opens the vault if you forget the master password."),
        onRestored: (snapshot) => {
          enterVault(status, snapshot);
          announce("Backup restored.");
        },
        onExists: () => void route(),
      },
      reason === "missing" ? "The vault file was removed. Create a new vault." : "",
    );
    return;
  }
  showUnlock(
    status,
    {
      onUnlocked: (snapshot) => enterVault(status, snapshot),
      onRecovered: (opened) =>
        enterOpened(
          status,
          opened,
          "Your master password is changed. The recovery key you used no longer works; this one replaces it.",
        ),
    },
    reason === "manual" ? undefined : reason,
  );
  if (reason === "manual") announce("Locked.");
}

void onVaultChanged((snapshot) => {
  if (inVault) applySnapshot(snapshot);
});
void onVaultLocked((reason) => void route(reason));
void onVaultError((message) => announce(`Could not read the vault just now: ${message}`));

route().catch((error: unknown) => announce(toFailure(error).message));

// Entry point: picks the first screen and follows the vault's lock state.

import { api, onVaultChanged, onVaultError, onVaultLocked, toFailure } from "@/lib/api.ts";
import { announce } from "@/lib/status.ts";
import type { LockReason, Snapshot, Status } from "@/lib/types.ts";
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

async function route(reason?: LockReason | "manual"): Promise<void> {
  if (inVault) {
    leaveVault();
    inVault = false;
  }
  const status = await api.status();
  if (!status.vaultPath) {
    showSetup(() => undefined, "lokey cannot find your local app data folder (LOCALAPPDATA).");
    return;
  }
  if (!status.exists) {
    showSetup(
      (snapshot) => enterVault(status, snapshot),
      reason === "missing" ? "The vault file was removed. Create a new vault." : "",
      () => void route(),
    );
    return;
  }
  showUnlock(
    status.vaultPath,
    status.lockoutSecs,
    (snapshot) => enterVault(status, snapshot),
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

// Shapes returned by the Rust side (src-tauri/src/dto.rs). Values are never
// part of a snapshot; the page asks for one at a time.

export type Status = {
  vaultPath: string;
  exists: boolean;
  unlocked: boolean;
  lockoutSecs: number;
  /** Whether a recovery key also opens the vault. */
  recovery: boolean;
};

export type Row = {
  project: string;
  key: string;
  length: number;
  updated: number;
};

export type Project = { name: string; count: number };

export type Arrival = { project: string; key: string; replaced: boolean };

export type Snapshot = {
  rows: Row[];
  projects: Project[];
  arrivals: Arrival[];
  rejected: number;
  saved: boolean | null;
};

/** A vault just created, recovered or restored, and the recovery key to show once. */
export type Opened = { snapshot: Snapshot; recoveryKey: string | null };

export type FailureCode =
  | "wrong-password"
  | "locked-out"
  | "stale"
  | "no-recovery"
  | "bad-recovery-key"
  | "no-vault"
  | "exists"
  | "not-found"
  | "invalid-name"
  | "weak-password"
  | "too-large"
  | "corrupt"
  | "unsupported"
  | "io"
  | "locked";

export type Failure = { code: FailureCode; message: string; waitMs: number };

/** Which kinds of character a generated password may contain. */
export type Charsets = { upper: boolean; lower: boolean; digits: boolean; symbols: boolean };

/** A generated password and its strength in bits. */
export type Generated = { value: string; bits: number };

/** Why the vault locked itself, pushed by the watcher. */
export type LockReason = "idle" | "stale" | "missing";

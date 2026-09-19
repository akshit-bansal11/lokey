// Shapes returned by the Rust side (src-tauri/src/dto.rs). Values are never
// part of a snapshot; the page asks for one at a time.

export type Status = {
  vaultPath: string;
  exists: boolean;
  unlocked: boolean;
  lockoutSecs: number;
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
  headerRestored: boolean;
  saved: boolean | null;
};

export type FailureCode =
  | "wrong-password"
  | "locked-out"
  | "stale"
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

/** Why the vault locked itself, pushed by the watcher. */
export type LockReason = "idle" | "stale" | "missing";

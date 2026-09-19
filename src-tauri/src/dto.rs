//! What crosses to the page. Values never appear here; the page asks for one
//! at a time through `reveal`. Lengths are sent so the masked cell can be
//! drawn without the value.

use lokey_core::{ChangeKind, Error, Report, Session};
use serde::Serialize;

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub vault_path: String,
    pub exists: bool,
    pub unlocked: bool,
    pub lockout_secs: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub project: String,
    pub key: String,
    pub length: usize,
    pub updated: u64,
}

#[derive(Clone, Serialize)]
pub struct Project {
    pub name: String,
    pub count: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Arrival {
    pub project: String,
    pub key: String,
    pub replaced: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub rows: Vec<Row>,
    pub projects: Vec<Project>,
    /// Values that arrived through password-less `set` (usually the CLI).
    pub arrivals: Vec<Arrival>,
    pub rejected: usize,
    pub header_restored: bool,
    /// Set by `save`: true when the key was new, false when it was replaced.
    pub saved: Option<bool>,
}

impl Snapshot {
    pub fn new(session: &Session, report: &Report) -> Self {
        Self {
            rows: session
                .entries()
                .iter()
                .map(|entry| Row {
                    project: entry.project.clone(),
                    key: entry.key.clone(),
                    length: entry.value.chars().count(),
                    updated: entry.updated,
                })
                .collect(),
            projects: session
                .projects()
                .into_iter()
                .map(|(name, count)| Project { name, count })
                .collect(),
            arrivals: report
                .changes
                .iter()
                .map(|change| Arrival {
                    project: change.project.clone(),
                    key: change.key.clone(),
                    replaced: change.kind == ChangeKind::Replaced,
                })
                .collect(),
            rejected: report.rejected,
            header_restored: report.header_restored,
            saved: None,
        }
    }
}

/// An error the page can branch on (`code`) and show (`message`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub code: &'static str,
    pub message: String,
    /// Backoff already served before this answer, for the page to explain.
    pub wait_ms: u64,
}

impl Failure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            wait_ms: 0,
        }
    }

    pub fn locked() -> Self {
        Self::new("locked", "The vault is locked. Unlock it to continue.")
    }

    pub fn no_data_folder() -> Self {
        Self::new(
            "unsupported",
            "lokey cannot find your local app data folder (LOCALAPPDATA).",
        )
    }
}

impl From<Error> for Failure {
    fn from(err: Error) -> Self {
        let message = err.to_string();
        match err {
            Error::WrongPassword { wait, .. } => Self {
                code: "wrong-password",
                message,
                wait_ms: u64::try_from(wait.as_millis()).unwrap_or(u64::MAX),
            },
            Error::LockedOut { .. } => Self::new("locked-out", message),
            Error::Stale => Self::new("stale", message),
            Error::NoVault => Self::new("no-vault", message),
            Error::VaultExists => Self::new("exists", message),
            Error::NotFound { .. } => Self::new("not-found", message),
            Error::InvalidName(_) => Self::new("invalid-name", message),
            Error::WeakPassword(_) => Self::new("weak-password", message),
            Error::TooLarge(_) => Self::new("too-large", message),
            Error::Corrupt(_) => Self::new("corrupt", message),
            Error::Unsupported(_) => Self::new("unsupported", message),
            Error::Io(_) => Self::new("io", message),
        }
    }
}

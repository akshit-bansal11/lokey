use std::{fmt, io, time::Duration};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// No vault file exists yet.
    NoVault,
    /// `create` was called but a vault already exists.
    VaultExists,
    /// A password did not match. The caller should wait `wait` before
    /// answering, which is what slows down guessing through the prompt.
    WrongPassword {
        attempts_left: u32,
        wait: Duration,
    },
    /// Too many wrong passwords in a row.
    LockedOut {
        remaining: Duration,
    },
    /// The vault was re-keyed by another process (passwords changed), so the
    /// key held in memory no longer opens it. The holder must unlock again.
    Stale,
    NotFound {
        project: String,
        key: String,
    },
    InvalidName(String),
    WeakPassword(String),
    TooLarge(String),
    /// The file exists but is not a vault this build can read.
    Corrupt(String),
    Unsupported(&'static str),
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoVault => write!(f, "no vault yet"),
            Self::VaultExists => write!(f, "a vault already exists"),
            Self::WrongPassword { attempts_left, .. } => write!(
                f,
                "wrong password, {attempts_left} attempt(s) left before a 15 minute lockout"
            ),
            Self::LockedOut { remaining } => {
                let secs = remaining.as_secs();
                write!(
                    f,
                    "locked after too many wrong passwords, try again in {}m {}s",
                    secs / 60,
                    secs % 60
                )
            }
            Self::Stale => write!(f, "the vault's passwords changed, unlock again"),
            Self::NotFound { project, key } => write!(f, "no key '{key}' in project '{project}'"),
            Self::InvalidName(why)
            | Self::WeakPassword(why)
            | Self::TooLarge(why)
            | Self::Corrupt(why) => write!(f, "{why}"),
            Self::Unsupported(what) => write!(f, "{what}"),
            Self::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

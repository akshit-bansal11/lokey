//! lokey-core: the only code that touches cryptography or the vault file.
//!
//! The CLI and the desktop app are thin front-ends over this crate, so the two
//! can never disagree about the file format.

pub mod clipboard;
mod crypto;
mod error;
mod format;
mod lockout;
mod names;
mod password;
mod store;
mod vault;

pub use error::{Error, Result};
pub use names::{DEFAULT_PROJECT, check_name, check_value, same as same_name};
pub use password::{MIN_PASSWORD_CHARS, check_new as check_new_password};
pub use store::Store;
pub use vault::{Change, ChangeKind, Entry, Report, Session};
